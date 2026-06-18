//! Internal implementation details for worxide.
//!
//! This module is `#[doc(hidden)]` and exists only so the `spawn!` and
//! `spawn_blocking!` macros have something to call. It is NOT part of the
//! public API and provides no stability guarantees — do not reference any of
//! it directly. (Rust macros that span crate boundaries require their callees
//! to be `pub`, so this can't be a private module; `#[doc(hidden)]` keeps it
//! out of the generated docs. This is the same pattern serde, wasm-bindgen,
//! and friends use for their macro-support modules.)
//!
//! The one exception is [`Worker`], the persistent worker handle, which IS
//! public API and is re-exported from the crate root as `worxide::Worker`.

use {
    anyhow::{Context, Result, anyhow},
    js_sys::{Promise, Reflect},
    std::{
        cell::{Cell, RefCell},
        collections::HashMap,
        fmt::Display,
        future::Future,
        pin::Pin,
        rc::Rc,
        str::FromStr,
        task::{Poll, RawWaker, RawWakerVTable, Waker},
    },
    wasm_bindgen::{closure::Closure, prelude::*},
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        Blob, BlobPropertyBag, MessageChannel, MessageEvent, MessagePort, Url, Worker as WebWorker, WorkerOptions,
        WorkerType,
    },
};

/// Worker bootstrap source, embedded at compile time. Turned into a Blob
/// URL on first spawn so the worker has nothing to serve from disk.
const WORKER_JS: &str = include_str!("worker.js");

/// Width of a pointer on the target, in bytes.
/// Pointers cross the worker boundary as exactly this many little-endian bytes
/// (see [`ptr_to_js`] / [`ptr_from_js`] / [`decode_task_ptr`]) rather than as
/// an `f64`. On wasm32 an `f64` would round-trip a 32-bit pointer losslessly,
/// but encoding the raw bytes keeps the transport honest (a pointer is an
/// integer, not a float) and stays correct on wasm64/memory64, where a pointer
/// can exceed the 2^53 exact-integer range of a double.
const PTR_LEN: usize = std::mem::size_of::<usize>();

thread_local! {
    /// Cached glue-file URL, resolved once per thread.
    ///
    /// On the main thread it is computed on first spawn (see `cached_glue_url`).
    /// On a worker it is *seeded* with the URL received over `postMessage` (see
    /// `__worxide_seed_glue_url`), so any nested spawn the worker performs
    /// reuses that already-resolved URL instead of re-deriving it.
    static GLUE_URL: RefCell<Option<String>> = const { RefCell::new(None) };
}

// Glue-URL resolvers (main-thread only — workers receive the resolved URL over
// postMessage and never call these).
//
//  * `worxide_glue_url`          — bare name, resolved relative to the snippet.
//  * `worxide_glue_url_from_path`— explicit path/URL, resolved against the page.
//  * `worxide_app_js_path`       — optional `globalThis.app_js_path` the
//                                  consumer may set in HTML; `None` if unset.
#[wasm_bindgen(inline_js = r#"
    export function worxide_glue_url(crate_name) {
        // Cargo replaces hyphens with underscores in library output filenames,
        // so a crate named "my-app" produces "my_app.js". Strip a trailing
        // ".js" so callers may pass either form.
        const file = crate_name.replace(/-/g, "_").replace(/\.js$/, "");
        return new URL("../../" + file + ".js", import.meta.url).href;
    }
    export function worxide_glue_url_from_path(path) {
        // Resolve against the document base; do NOT mangle the string.
        return new URL(path, document.baseURI).href;
    }
    export function worxide_app_js_path() {
        // Optional consumer-set global, e.g. one line in HTML:
        //   globalThis.app_js_path = "my_app.js";   // or "/static/my_app.js"
        // Read from globalThis so it is safe to call from any context; returns
        // null when unset / not a non-empty string so wasm-bindgen maps it to
        // `None`.
        const p = globalThis.app_js_path;
        return (typeof p === "string" && p.length > 0) ? p : null;
    }
"#)]
extern "C" {
    fn worxide_glue_url(crate_name: &str) -> String;
    fn worxide_glue_url_from_path(path: &str) -> String;
    fn worxide_app_js_path() -> Option<String>;
}

/// Resolve (and memoize) the glue-file URL for *this* thread.
///
/// Precedence:
///   1. the consumer-set `globalThis.app_js_path` global (resolved against the
///      page base, used verbatim),
///   2. the crate name the macro captured via `CARGO_PKG_NAME` (correct for a
///      single-crate app; see the note in `lib.rs` about libraries).
///
/// Resolution happens once and is memoized, so `globalThis.app_js_path` must be
/// set before the first spawn. On a worker, `GLUE_URL` has already been seeded
/// with the resolved URL (see `__worxide_seed_glue_url`), so neither resolver
/// runs there — which is why a worker never needs a global it cannot see.
fn cached_glue_url(crate_name: &str) -> String {
    GLUE_URL.with(|cell| {
        cell.borrow_mut()
            .get_or_insert_with(|| {
                if let Some(p) = worxide_app_js_path() {
                    // A consumer-supplied path/name; resolve it against the page
                    // base rather than mangling it like a crate name.
                    worxide_glue_url_from_path(&p)
                } else {
                    worxide_glue_url(crate_name)
                }
            })
            .clone()
    })
}

/// Resolve a glue URL for a persistent [`Worker`] — *without* the crate-name
/// fallback the macros use. Prefers an already-cached URL (e.g. seeded by a
/// prior spawn or worker on this thread), otherwise the consumer-set
/// `globalThis.app_js_path`. Errors if neither is available, since a persistent
/// `Worker` has no `CARGO_PKG_NAME` to fall back on — the glue lives in the
/// app's wasm, which the page identifies via `app_js_path`.
fn glue_url_via_app_path() -> Result<String> {
    if let Some(url) = GLUE_URL.with(|c| c.borrow().clone()) {
        return Ok(url);
    }
    let url = worxide_app_js_path().map(|p| worxide_glue_url_from_path(&p)).context(
        "worxide::Worker: no glue URL available — set `globalThis.app_js_path` in your page, or construct with \
         `Worker::with_glue(path)`",
    )?;
    GLUE_URL.with(|c| *c.borrow_mut() = Some(url.clone()));
    Ok(url)
}

/// Resolve an explicit glue path/URL against the document base and cache it.
fn glue_url_explicit(path: &str) -> String {
    let url = worxide_glue_url_from_path(path);
    GLUE_URL.with(|c| *c.borrow_mut() = Some(url.clone()));
    url
}

fn worker_url() -> Result<String> {
    // Create a fresh Blob URL on each spawn. Browsers may revoke or restrict Blob URLs used by previous Workers, so don't cache.
    let array = js_sys::Array::new();
    array.push(&JsValue::from_str(WORKER_JS));
    let opts = BlobPropertyBag::new();
    opts.set_type("application/javascript");
    let blob = Blob::new_with_str_sequence_and_options(&array, &opts).map_err(|e| js_err("Blob construction failed", e))?;
    Url::create_object_url_with_blob(&blob).map_err(|e| js_err("URL.createObjectURL failed", e))
}

/// Sync task: closure returns the boxed result pointer directly.
pub struct WorkerTask {
    func: Box<dyn FnOnce() -> *mut () + Send>,
}

impl WorkerTask {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> *mut () + Send + 'static,
    {
        Self { func: Box::new(f) }
    }

    pub fn run(self) -> *mut () { (self.func)() }

    pub fn into_ptr(self) -> usize { Box::into_raw(Box::new(self)) as usize }

    /// # Safety: ptr must come from `into_ptr` and not be freed yet.
    pub unsafe fn from_ptr(ptr: usize) -> Box<WorkerTask> { unsafe { Box::from_raw(ptr as *mut WorkerTask) } }
}

/// Async task: closure returns a future that resolves to the boxed result.
/// The *outer* closure — which is what carries the captured arguments across
/// the thread boundary — is `Send`. The future it produces is created and
/// driven entirely on the worker thread and is deliberately **not** required
/// to be `Send`; most wasm futures aren't.
pub struct AsyncWorkerTask {
    func: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = *mut ()>>> + Send>,
}

impl AsyncWorkerTask {
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = *mut ()> + 'static,
    {
        Self { func: Box::new(move || Box::pin(f())) }
    }

    pub fn run(self) -> Pin<Box<dyn Future<Output = *mut ()>>> { (self.func)() }

    pub fn into_ptr(self) -> usize { Box::into_raw(Box::new(self)) as usize }

    /// # Safety: ptr must come from `into_ptr` and not be freed yet.
    pub unsafe fn from_ptr(ptr: usize) -> Box<AsyncWorkerTask> { unsafe { Box::from_raw(ptr as *mut AsyncWorkerTask) } }
}

pub fn box_result<T: Send + 'static>(value: T) -> *mut () { Box::into_raw(Box::new(value)) as *mut () }

/// # Safety: ptr must come from `box_result::<T>` and not be freed yet.
pub unsafe fn unbox_result<T: Send + 'static>(ptr: *mut ()) -> T { unsafe { *Box::from_raw(ptr as *mut T) } }

/// Free a result box produced by [`box_result`] as its concrete `R`, *without*
/// returning the value. Used on the cancellation/abandon path: the awaiting
/// `run`/`run_blocking` future that would normally [`unbox_result`] (and drop)
/// is gone, but the worker still produced a box that has to be freed somewhere.
///
/// # Safety: `ptr` must come from `box_result::<R>` and not have been taken yet.
unsafe fn free_result_box<R: 'static>(ptr: usize) { drop(unsafe { Box::from_raw(ptr as *mut R) }); }

// A pointer crosses the `postMessage` boundary — and resolves the async
// worker's Promise — as a little-endian byte payload, never as an `f64`. The
// worker entry points take `&[u8]` and return `Vec<u8>`, so the bytes are
// reassembled into a `usize` in Rust (`from_le_bytes`) and JS never has to turn
// them back into a (double-precision) number.

/// Encode a pointer as a fresh `Uint8Array` of little-endian bytes.
fn ptr_to_js(ptr: usize) -> JsValue {
    // `Uint8Array::from(&[u8])` allocates its OWN (non-shared) buffer and
    // copies the bytes in — it is *not* a view over wasm linear memory, so
    // structured clone just copies these few bytes instead of entangling the
    // backing SharedArrayBuffer. (Do not switch this to `Uint8Array::view`.)
    js_sys::Uint8Array::from(&ptr.to_le_bytes()[..]).into()
}

/// Decode a pointer previously produced by [`ptr_to_js`].
fn ptr_from_js(v: &JsValue) -> Result<usize> {
    let arr = v.dyn_ref::<js_sys::Uint8Array>().context("worker reply was not a Uint8Array")?;
    let bytes = arr.to_vec();
    let buf: [u8; PTR_LEN] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("worker reply was {} bytes, expected a {PTR_LEN}-byte pointer", bytes.len()))?;
    Ok(usize::from_le_bytes(buf))
}

/// Encode a 64-bit call id as little-endian bytes (a fresh `Uint8Array`).
///
/// Sent verbatim by worker.js in the matching `result` envelope. Bytes rather than
/// an `f64` so the id round-trips losslessly past 2^53 — a collision between two
/// in-flight ids would route a result to the wrong call and unbox it as the
/// wrong type, so the transport has to be exact.
fn id_to_js(id: u64) -> JsValue { js_sys::Uint8Array::from(&id.to_le_bytes()[..]).into() }

/// Decode a call id produced by [`id_to_js`].
fn id_from_js(v: &JsValue) -> Option<u64> {
    let arr = v.dyn_ref::<js_sys::Uint8Array>()?;
    let buf: [u8; 8] = arr.to_vec().as_slice().try_into().ok()?;
    Some(u64::from_le_bytes(buf))
}

/// Decode a pointer handed to a worker entry point as `&[u8]`.
///
/// Returns `Err` (surfaced to JS, caught by worker.js as a per-call error
/// envelope) rather than trapping on a wrong-length payload. With the control
/// channel now private this is unreachable from a consumer, but failing one call
/// instead of aborting the whole shared instance is the right behaviour for any
/// stray or buggy envelope regardless.
fn decode_task_ptr(bytes: &[u8]) -> Result<usize, JsValue> {
    let buf: [u8; PTR_LEN] =
        bytes.try_into().map_err(|_| JsValue::from_str("worxide: task pointer payload had the wrong length"))?;
    Ok(usize::from_le_bytes(buf))
}

fn js_err(context: &'static str, v: JsValue) -> anyhow::Error {
    // Try a sequence of strategies to get something readable.
    // 1. If it's already a string.
    if let Some(s) = v.as_string() {
        return anyhow!("{context}: {s}");
    }
    // 2. DOMException / Error — read .name + .message via Reflect.
    let name = js_sys::Reflect::get(&v, &"name".into()).ok().and_then(|x| x.as_string());
    let msg = js_sys::Reflect::get(&v, &"message".into()).ok().and_then(|x| x.as_string());
    if name.is_some() || msg.is_some() {
        let n = name.as_deref().unwrap_or("Error");
        let m = msg.as_deref().unwrap_or("(no message)");
        return anyhow!("{context}: {n}: {m}");
    }
    // 3. Fall back to JSON.stringify.
    let stringified =
        js_sys::JSON::stringify(&v).ok().and_then(|s| s.as_string()).unwrap_or_else(|| "<unprintable JsValue>".to_owned());
    anyhow!("{context}: {stringified}")
}

#[derive(Clone, Copy)]
enum WorkerExecution {
    Sync,
    Async,
}
impl Display for WorkerExecution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerExecution::Sync => "sync",
            WorkerExecution::Async => "async",
        }
        .fmt(f)
    }
}
impl FromStr for WorkerExecution {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sync" => Ok(WorkerExecution::Sync),
            "async" => Ok(WorkerExecution::Async),
            other => Err(anyhow::anyhow!("`{other}` is not a valid WorkerExecution!")),
        }
    }
}

async fn construct_worker(
    task_ptr: usize,
    kind: WorkerExecution,
    glue_url: String,
    module: JsValue,
    memory: JsValue,
    worker_url: String,
) -> Result<(WebWorker, Promise)> {
    let opts = WorkerOptions::new();
    opts.set_type(WorkerType::Module);
    let worker = WebWorker::new_with_options(&worker_url, &opts).map_err(|e| js_err("Worker construction failed", e))?;

    let (promise, resolve, reject) = {
        let mut resolve_slot = None;
        let mut reject_slot = None;
        let promise = Promise::new(&mut |res, rej| {
            resolve_slot = Some(res);
            reject_slot = Some(rej);
        });
        (
            promise,
            resolve_slot.context("Promise executor did not capture resolve")?,
            reject_slot.context("Promise executor did not capture reject")?,
        )
    };

    let on_message = {
        let resolve = resolve.clone();
        Closure::once_into_js(move |evt: MessageEvent| {
            // The reply is a Uint8Array of little-endian pointer bytes.
            // Forward it verbatim; run_inner decodes and validates it via
            // ptr_from_js (a non-Uint8Array reply surfaces as an Err there).
            resolve.call1(&JsValue::NULL, &evt.data()).unwrap();
        })
    };
    worker.set_onmessage(Some(on_message.unchecked_ref()));

    let on_error = {
        let reject = reject.clone();
        Closure::once_into_js(move |evt: JsValue| {
            reject.call1(&JsValue::NULL, &evt).unwrap();
        })
    };
    worker.set_onerror(Some(on_error.unchecked_ref()));

    let msg = js_sys::Object::new();
    Reflect::set(&msg, &"kind".into(), &JsValue::from_str(&kind.to_string()))
        .map_err(|e| js_err("set kind on message", e))?;
    Reflect::set(&msg, &"module".into(), &module).map_err(|e| js_err("set module on message", e))?;
    Reflect::set(&msg, &"memory".into(), &memory).map_err(|e| js_err("set memory on message", e))?;
    // Pointer travels as little-endian bytes (a Uint8Array), never as an f64.
    Reflect::set(&msg, &"ptr".into(), &ptr_to_js(task_ptr)).map_err(|e| js_err("set ptr on message", e))?;
    // The *resolved* glue URL goes over the wire so the worker (and any nested
    // spawn it makes) never has to re-derive it from a crate name or a global
    // it can't see. worker.js feeds this to `__worxide_seed_glue_url`.
    Reflect::set(&msg, &"glue_url".into(), &JsValue::from_str(&glue_url))
        .map_err(|e| js_err("set glue_url on message", e))?;

    // Once postMessage succeeds, ownership of the boxed task has passed to
    // the worker thread; the worker is responsible for freeing it.
    worker.post_message(&msg).map_err(|e| js_err("postMessage to worker failed", e))?;
    Ok((worker, promise))
}

/// Spawn a worker, post the task pointer, await the reply.
/// `kind` is "sync" or "async" — tells worker.js which entry to call.
async fn run_inner(task_ptr: usize, kind: WorkerExecution, crate_name: &str) -> Result<usize> {
    let glue_url = cached_glue_url(crate_name);
    let worker_url = worker_url()?;
    let module = wasm_bindgen::module();
    let memory = wasm_bindgen::memory();

    let (worker, promise) = match construct_worker(task_ptr, kind, glue_url, module, memory, worker_url).await {
        Ok(tuple) => tuple,
        Err(e) => {
            // We failed before/at postMessage, so the worker never took
            // ownership of the task: reclaim and drop it here, freeing it as
            // the *same* type we boxed (sync vs async). `kind` is `Copy`, so it
            // is still available even though the block above captured a copy.
            // SAFETY: task_ptr came from `WorkerTask::into_ptr` (Sync) or
            // `AsyncWorkerTask::into_ptr` (Async) on this thread, per `kind`,
            // and has not been freed.
            unsafe {
                match kind {
                    WorkerExecution::Sync => drop(WorkerTask::from_ptr(task_ptr)),
                    WorkerExecution::Async => drop(AsyncWorkerTask::from_ptr(task_ptr)),
                }
            }
            return Err(e);
        }
    };

    let resolved = JsFuture::from(promise).await.map_err(|e| js_err("worker future rejected", e))?;
    worker.terminate();

    // `resolved` is a structured-clone copy of the worker's Uint8Array, owned
    // by this (main) thread, so decoding it after `terminate()` is fine.
    ptr_from_js(&resolved)
}

/// Submit a sync closure to a worker and await its result. `R` is inferred
/// from the closure's return type — no turbofish needed at the call site.
pub async fn run_blocking<F, R>(f: F, crate_name: &str) -> Result<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let task = WorkerTask::new(move || box_result(f()));
    let result_ptr = run_inner(task.into_ptr(), WorkerExecution::Sync, crate_name).await?;

    // SAFETY: result_ptr came from box_result::<R> on the worker, and R: Send
    // so taking ownership of it on this thread is sound.
    Ok(unsafe { unbox_result::<R>(result_ptr as *mut ()) })
}

/// Submit an async closure to a worker and await its result.
pub async fn run_async<F, Fut, R>(f: F, crate_name: &str) -> Result<R>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = R> + 'static,
    R: Send + 'static,
{
    let task = AsyncWorkerTask::new(move || async move { box_result(f().await) });
    let result_ptr = run_inner(task.into_ptr(), WorkerExecution::Async, crate_name).await?;

    // SAFETY: result_ptr came from box_result::<R> on the worker, and R: Send
    // so taking ownership of it on this thread is sound.
    Ok(unsafe { unbox_result::<R>(result_ptr as *mut ()) })
}

// ===========================================================================
// Persistent worker handle
// ===========================================================================
//
// `spawn!` / `spawn_blocking!` (via `run_inner` above) create a worker, run one
// task, and terminate it. `Worker` instead boots a worker, attaches it to this
// thread's shared memory once, and keeps it alive so many tasks run over the
// same instance.
//
// Protocol (see worker.js):
//   main -> worker  { type: "init",  module, memory, glue_url } + [control port]
//   worker -> port  { type: "ready" }
//   main -> port    { type: "call",  id, kind, ptr }
//   worker -> port  { type: "result", id, result }            // success
//   worker -> port  { type: "result", id, error }             // task threw
//
// Only the one `init` envelope rides the worker's default postMessage channel,
// and it transfers a `MessageChannel` port. Everything after — `ready`, every
// `call`, every `result` — runs over that private port. The worker's default
// channel is thereby left entirely to the consumer (see `Worker::raw`): their
// traffic physically cannot reach worxide's dispatch, and a `call` they forge on
// the default channel goes nowhere. That is what makes `raw()` safe.
//
// The one-shot envelope used by `run_inner` carries no `type` and rides the
// default channel (one-shot workers are never shared, so there is no port and no
// `raw()`); worker.js keeps routing it through the old path untouched.
//
// Per-call error isolation: a Rust panic under `panic = "abort"` traps to JS as
// a RuntimeError, which worker.js catches and reports as a `result` envelope with
// an `error`. That rejects the one call's future; the worker instance survives
// and keeps serving. (The trapped task's box does not get its `Drop` run —
// abort does not unwind — a bounded, per-panic leak.) A worker-level `error`
// event, by contrast, means something we did not catch; the handle is then
// marked dead and every in-flight call is rejected.
//
// Remaining bounded leaks (both tied to worker lifetime, freed at teardown):
// the trapped task's box on a per-panic abort (above), and a task box still
// in flight if the worker dies mid-call — the worker owned it and never
// replied. A dropped result future no longer leaks: see `AbandonGuard`.

/// Where a call's outcome lands. `on_message` deposits into the shared slot and
/// fires the call's signal; the awaiting `dispatch` future takes it on resume.
/// If that future was dropped first, [`AbandonGuard`] frees any deposited result
/// box, so a cancelled call leaks nothing.
enum Slot {
    /// No reply yet.
    Waiting,
    /// Result pointer, decoded from the reply, not yet taken.
    Ok(usize),
    /// Task threw, worker died, or the reply was malformed.
    Err(String),
    /// The awaiting future is gone; `on_message` must free the box itself.
    Abandoned,
    /// Outcome consumed by the awaiter.
    Done,
}

/// One in-flight call. Lives in `Worker::pending` keyed by id until its `result`
/// envelope arrives (or the worker dies).
struct Pending {
    /// Shared with the awaiting `dispatch` future and its [`AbandonGuard`].
    slot: Rc<RefCell<Slot>>,
    /// Resolve fn of the call's signal promise; called (no payload) to wake the
    /// awaiter once `slot` holds an outcome.
    signal: js_sys::Function,
    /// Frees a deposited result box as its concrete `R`; used only when the call
    /// was abandoned (the live path unboxes in `run`/`run_blocking`).
    free_result: unsafe fn(usize),
}

/// RAII guard held across `dispatch`'s await. If the awaiting future is dropped
/// before it takes the outcome, this reaps the result box that would otherwise
/// leak:
///   * one that already landed (`Slot::Ok`) is freed here, and
///   * one still in flight is handled by flipping the slot to `Abandoned`, so
///     the late `result` envelope is freed by `on_message` instead of delivered.
///
/// Disarmed once `dispatch` has taken the outcome.
struct AbandonGuard {
    slot: Rc<RefCell<Slot>>,
    free_result: unsafe fn(usize),
    armed: Cell<bool>,
}

impl AbandonGuard {
    fn disarm(&self) { self.armed.set(false); }
}

impl Drop for AbandonGuard {
    fn drop(&mut self) {
        if !self.armed.get() {
            return;
        }
        // Awaiter dropped before taking its outcome. Free a result that already
        // landed; otherwise mark the slot so a late envelope is reaped downstream.
        // SAFETY: a `Slot::Ok` ptr came from `box_result::<R>` on the worker and
        // `free_result` is that same `R`'s deallocator; nothing else has taken it
        // (the slot was not `Done`).
        if let Slot::Ok(ptr) = std::mem::replace(&mut *self.slot.borrow_mut(), Slot::Abandoned) {
            unsafe { (self.free_result)(ptr) }
        }
    }
}

/// Create a fresh JS `Promise`, returning it alongside its `resolve`/`reject`
/// functions pulled out of the executor.
fn new_promise() -> Result<(Promise, js_sys::Function, js_sys::Function)> {
    let mut resolve_slot = None;
    let mut reject_slot = None;
    let promise = Promise::new(&mut |res, rej| {
        resolve_slot = Some(res);
        reject_slot = Some(rej);
    });
    Ok((
        promise,
        resolve_slot.context("Promise executor did not capture resolve")?,
        reject_slot.context("Promise executor did not capture reject")?,
    ))
}

/// Read the `type` discriminator off a worker envelope, if present.
fn envelope_type(data: &JsValue) -> Option<String> {
    Reflect::get(data, &JsValue::from_str("type")).ok().and_then(|v| v.as_string())
}

/// A persistent Web Worker attached to this thread's shared memory.
///
/// Construct it with [`Worker::new`]
/// (which resolves the wasm glue via the consumer-set `globalThis.app_js_path`) or [`Worker::with_glue`] (an explicit path/URL).
/// There is no `spawn_persistent!` macro: once you hold a handle, run work with the plain methods [`Worker::run_blocking`] (sync, CPU-bound)
/// and [`Worker::run`] (async). Arguments and results cross by pointer through shared memory.
///
/// Dropping the handle terminates the worker.
///
/// ```ignore
/// let w = worxide::Worker::new().await?;              // glue via globalThis.app_js_path
/// let n = w.run_blocking(move || crunch(data)).await?;
///
/// w.terminate();
/// ```
pub struct Worker {
    inner: WebWorker,
    // Our end of the private control channel; the other end was transferred to
    // the worker at boot. Every `call` goes out here and every `ready`/`result`
    // comes back here, isolated from the consumer's use of `inner`.
    port: MessagePort,
    pending: Rc<RefCell<HashMap<u64, Pending>>>,
    next_id: Rc<Cell<u64>>,
    dead: Rc<Cell<bool>>,
    // Retained so the listeners outlive `boot` and keep firing for the life of
    // the handle. Dropping the handle drops these closures and terminates the
    // worker (see `Drop`). NOT `forget()`-ed: a persistent worker must keep its
    // handlers callable rather than leak them.
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(JsValue)>,
}

impl Worker {
    /// Spawn a persistent worker, attach it to this thread's shared memory, and
    /// resolve once it reports ready.
    ///
    /// Resolves the wasm glue via the consumer-set `globalThis.app_js_path`
    /// (the path the app's `init` script uses). A library embedding worxide
    /// relies on this, since the glue lives in the *app's* wasm, not the
    /// library's. Errors if `app_js_path` is unset and nothing has seeded the
    /// glue URL yet — use [`Worker::with_glue`] to pass it explicitly.
    pub async fn new() -> Result<Self> { Self::boot(glue_url_via_app_path()?).await }

    /// Like [`Worker::new`], but with an explicit glue path/URL (resolved
    /// against the document base) instead of `globalThis.app_js_path`.
    pub async fn with_glue(glue_path: &str) -> Result<Self> { Self::boot(glue_url_explicit(glue_path)).await }

    async fn boot(glue_url: String) -> Result<Self> {
        let worker_url = worker_url()?;
        let module = wasm_bindgen::module();
        let memory = wasm_bindgen::memory();

        let opts = WorkerOptions::new();
        opts.set_type(WorkerType::Module);
        let inner = WebWorker::new_with_options(&worker_url, &opts).map_err(|e| js_err("Worker construction failed", e))?;

        // Private control channel. `port1` stays here and carries every
        // ready/call/result; `port2` is transferred to the worker in the `init`
        // envelope below. After that handover, the worker's *default* channel
        // belongs entirely to the consumer — worxide never reads it again — so
        // nothing a consumer posts via `raw()` can reach this dispatch.
        let channel = MessageChannel::new().map_err(|e| js_err("MessageChannel construction failed", e))?;
        let port = channel.port1();
        let worker_port = channel.port2();

        let pending: Rc<RefCell<HashMap<u64, Pending>>> = Rc::new(RefCell::new(HashMap::new()));
        let next_id = Rc::new(Cell::new(0u64));
        let dead = Rc::new(Cell::new(false));

        // Resolves when the worker posts `{ type: "ready" }` after initSync.
        let (ready, resolve_ready, reject_ready) = new_promise()?;

        // One persistent listener on the control port handles the ready
        // handshake and every later `result` envelope, routing each result to
        // its pending call by id.
        let on_message = {
            let pending = pending.clone();
            let ready_resolve = resolve_ready.clone();
            Closure::<dyn FnMut(MessageEvent)>::new(move |ev: MessageEvent| {
                let data = ev.data();
                match envelope_type(&data).as_deref() {
                    Some("ready") => {
                        ready_resolve.call0(&JsValue::NULL).ok();
                    }
                    Some("result") => {
                        let Some(id) = Reflect::get(&data, &"id".into()).ok().and_then(|v| id_from_js(&v)) else {
                            return;
                        };
                        // Unknown id: the call already settled (a late or
                        // duplicate result). Drop it.
                        let Some(p) = pending.borrow_mut().remove(&id) else {
                            return;
                        };
                        // Decode the envelope into an outcome before touching the
                        // slot, so the abandoned branch can free or the live
                        // branch can deposit.
                        let err = Reflect::get(&data, &"error".into()).ok();
                        let outcome = match err {
                            Some(e) if !e.is_undefined() && !e.is_null() => {
                                Slot::Err(e.as_string().unwrap_or_else(|| "worxide: worker task error".to_owned()))
                            }
                            _ => {
                                let result = Reflect::get(&data, &"result".into()).unwrap_or(JsValue::UNDEFINED);
                                match ptr_from_js(&result) {
                                    Ok(ptr) => Slot::Ok(ptr),
                                    Err(e) => Slot::Err(format!("{e:#}")),
                                }
                            }
                        };
                        let mut slot = p.slot.borrow_mut();
                        if matches!(&*slot, Slot::Abandoned) {
                            // Awaiter already gone: free the result box ourselves
                            // rather than delivering it into the void.
                            if let Slot::Ok(ptr) = outcome {
                                // SAFETY: ptr came from box_result::<R> on the
                                // worker; free_result is that R's deallocator.
                                unsafe { (p.free_result)(ptr) };
                            }
                        } else {
                            *slot = outcome;
                            drop(slot);
                            // Wake the awaiter; it takes the outcome from `slot`.
                            p.signal.call0(&JsValue::NULL).ok();
                        }
                    }
                    _ => {}
                }
            })
        };
        port.add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref())
            .map_err(|e| js_err("addEventListener(\"message\") on control port failed", e))?;
        // Required when listening via addEventListener (unlike `onmessage=`, which
        // auto-starts). Buffered envelopes — e.g. an early `ready` — are delivered
        // once started, so this is race-free even though we start before `init`.
        port.start();

        // A worker-level `error` event — not a per-call failure (those arrive as
        // `result` envelopes). Mark the handle dead, fail the boot handshake if
        // still pending, and reject every in-flight call.
        let on_error = {
            let pending = pending.clone();
            let dead = dead.clone();
            let ready_reject = reject_ready.clone();
            Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
                dead.set(true);
                ready_reject.call1(&JsValue::NULL, &e).ok();
                let why = e.as_string().unwrap_or_else(|| "worxide: worker error".to_owned());
                let drained: Vec<Pending> = pending.borrow_mut().drain().map(|(_, p)| p).collect();
                for p in drained {
                    let mut slot = p.slot.borrow_mut();
                    // A dead worker produced no result box, so there is nothing
                    // to free on the abandoned path — just fail the live calls.
                    if !matches!(&*slot, Slot::Abandoned) {
                        *slot = Slot::Err(why.clone());
                        drop(slot);
                        p.signal.call0(&JsValue::NULL).ok();
                    }
                }
            })
        };
        inner
            .add_event_listener_with_callback("error", on_error.as_ref().unchecked_ref())
            .map_err(|e| js_err("addEventListener(\"error\") failed", e))?;

        // Hand the worker the module, the shared memory, the resolved glue URL,
        // and — transferred — its end of the control port. worker.js imports the
        // glue, runs initSync ONCE, adopts the port, seeds the glue URL, and
        // replies `{ type: "ready" }` over the port. This is the only envelope on
        // the default channel; everything else is on the port.
        let msg = js_sys::Object::new();
        Reflect::set(&msg, &"type".into(), &"init".into()).map_err(|e| js_err("set type on init", e))?;
        Reflect::set(&msg, &"module".into(), &module).map_err(|e| js_err("set module on init", e))?;
        Reflect::set(&msg, &"memory".into(), &memory).map_err(|e| js_err("set memory on init", e))?;
        Reflect::set(&msg, &"glue_url".into(), &JsValue::from_str(&glue_url))
            .map_err(|e| js_err("set glue_url on init", e))?;
        // Transfer `worker_port`: it arrives in the worker as `event.ports[0]`.
        let transfer = js_sys::Array::of1(&worker_port);
        inner.post_message_with_transfer(&msg, &transfer).map_err(|e| js_err("postMessage(init) failed", e))?;

        JsFuture::from(ready).await.map_err(|e| js_err("worker init failed", e))?;

        Ok(Self { inner, port, pending, next_id, dead, _on_message: on_message, _on_error: on_error })
    }

    /// Run a synchronous closure on the worker and await its result. `R` is
    /// inferred from the closure's return type. The closure (with its captured
    /// arguments) and the result cross by pointer through shared memory.
    pub async fn run_blocking<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let task = WorkerTask::new(move || box_result(f()));
        let result_ptr = self.dispatch(task.into_ptr(), WorkerExecution::Sync, free_result_box::<R>).await?;
        // SAFETY: result_ptr came from box_result::<R> on the worker; R: Send.
        Ok(unsafe { unbox_result::<R>(result_ptr as *mut ()) })
    }

    /// Run an asynchronous closure on the worker and await its result. The
    /// worker drives the future to completion on its own event loop.
    pub async fn run<F, Fut, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = R> + 'static,
        R: Send + 'static,
    {
        let task = AsyncWorkerTask::new(move || async move { box_result(f().await) });
        let result_ptr = self.dispatch(task.into_ptr(), WorkerExecution::Async, free_result_box::<R>).await?;
        // SAFETY: result_ptr came from box_result::<R> on the worker; R: Send.
        Ok(unsafe { unbox_result::<R>(result_ptr as *mut ()) })
    }

    /// Post a `call` envelope for `task_ptr`, register a pending entry under a
    /// fresh id, and await its outcome — the result pointer, or an `Err` if the
    /// task threw or the worker died.
    ///
    /// `free_result` is the deallocator for this call's result type. It runs only
    /// if the awaiting future is dropped while a result is (or becomes) in flight
    /// (see [`AbandonGuard`]); the normal path returns the pointer and lets
    /// `run`/`run_blocking` unbox it.
    async fn dispatch(&self, task_ptr: usize, kind: WorkerExecution, free_result: unsafe fn(usize)) -> Result<usize> {
        if self.dead.get() {
            // The worker will never take the task: reclaim it now.
            // SAFETY: task_ptr came from {WorkerTask,AsyncWorkerTask}::into_ptr
            // per `kind`, and has not been freed.
            unsafe {
                match kind {
                    WorkerExecution::Sync => drop(WorkerTask::from_ptr(task_ptr)),
                    WorkerExecution::Async => drop(AsyncWorkerTask::from_ptr(task_ptr)),
                }
            }
            return Err(anyhow!("worxide::Worker is dead (an earlier worker error tore it down); construct a new one"));
        }

        let id = {
            let n = self.next_id.get();
            self.next_id.set(n.wrapping_add(1));
            n
        };
        let slot = Rc::new(RefCell::new(Slot::Waiting));
        // The signal promise carries no payload — the outcome travels in `slot`.
        // We only need its resolve fn; it is never rejected.
        let (signal_promise, signal, _) = new_promise()?;
        self.pending.borrow_mut().insert(id, Pending { slot: slot.clone(), signal, free_result });

        // Arm before posting: from here until we take the outcome, a dropped
        // future must reap any result the worker produces.
        let guard = AbandonGuard { slot: slot.clone(), free_result, armed: Cell::new(true) };

        let msg = js_sys::Object::new();
        Reflect::set(&msg, &"type".into(), &"call".into()).map_err(|e| js_err("set type on call", e))?;
        Reflect::set(&msg, &"id".into(), &id_to_js(id)).map_err(|e| js_err("set id on call", e))?;
        Reflect::set(&msg, &"kind".into(), &JsValue::from_str(&kind.to_string()))
            .map_err(|e| js_err("set kind on call", e))?;
        Reflect::set(&msg, &"ptr".into(), &ptr_to_js(task_ptr)).map_err(|e| js_err("set ptr on call", e))?;

        if let Err(e) = self.port.post_message(&msg) {
            // Never reached the worker: drop the pending entry and reclaim the
            // task box (the worker never took ownership). Nothing was deposited,
            // so disarm — there is no result to reap.
            guard.disarm();
            self.pending.borrow_mut().remove(&id);
            // SAFETY: as above; task_ptr not yet handed off.
            unsafe {
                match kind {
                    WorkerExecution::Sync => drop(WorkerTask::from_ptr(task_ptr)),
                    WorkerExecution::Async => drop(AsyncWorkerTask::from_ptr(task_ptr)),
                }
            }
            return Err(js_err("postMessage(call) failed", e));
        }

        // Park until `on_message` deposits the outcome and fires the signal. If
        // this await is dropped, `guard` reaps; otherwise we disarm and take.
        // The signal promise resolves with no value and never rejects, so the
        // result of the await itself is irrelevant — the outcome is in `slot`.
        JsFuture::from(signal_promise).await.ok();
        guard.disarm();

        match std::mem::replace(&mut *slot.borrow_mut(), Slot::Done) {
            Slot::Ok(ptr) => Ok(ptr),
            Slot::Err(why) => Err(anyhow!("{why}")),
            // Signalled without an outcome, or signalled twice — neither should
            // happen; treat as a lost reply rather than fabricating a pointer.
            Slot::Waiting | Slot::Abandoned | Slot::Done => Err(anyhow!("worxide: worker reply was lost")),
        }
    }

    /// The underlying worker.
    ///
    /// Exposed so a consumer can `post_message_with_transfer` (e.g. to hand the
    /// worker an `OffscreenCanvas`) and attach its own `addEventListener`
    /// side-channels alongside worxide's.
    ///
    /// This is safe to use freely: worxide's call/result protocol runs over a
    /// private [`MessagePort`] the worker adopted at boot, so the worker's
    /// default message channel — the one reached through here — carries none of
    /// it. Your `postMessage` traffic cannot collide with worxide's dispatch (a
    /// forged `call` on this channel goes nowhere), and worxide never reads this
    /// channel after init, so it will not touch your envelopes. Receive your own
    /// envelopes on the worker side with your own `self.addEventListener`.
    pub fn worker_handle(&self) -> &WebWorker { &self.inner }

    /// Terminate the worker immediately. Idempotent; also runs on drop.
    pub fn terminate(&self) {
        self.port.close();
        self.inner.terminate();
    }
}

impl Drop for Worker {
    fn drop(&mut self) { self.terminate(); }
}

// Drives a future to completion on the worker's own event loop, returning
// a Promise that resolves with the future's *mut () output (as pointer bytes).
// Why not wasm_bindgen_futures? With atomics on (needed for shared memory) it uses a multithread executor that coordinates wakeups via Atomics.waitAsync and a helper worker
// Reschedule polls through queueMicrotask; JsFuture's own Promise.then wakeups also land on this event loop

type SharedFut = Rc<RefCell<Pin<Box<dyn Future<Output = *mut ()>>>>>;

struct DriveState {
    fut: SharedFut,
    resolve: js_sys::Function,
    scheduled: RefCell<bool>,
    // Self-reference so the waker can reschedule a poll. Set once,
    // immediately after construction.
    this: RefCell<Option<Rc<DriveState>>>,
}

impl DriveState {
    fn poll(self: &Rc<Self>) {
        *self.scheduled.borrow_mut() = false;
        let waker = self.clone().into_waker();
        let mut cx = std::task::Context::from_waker(&waker);
        let mut slot = self.fut.borrow_mut();
        if let Poll::Ready(result_ptr) = slot.as_mut().poll(&mut cx) {
            drop(slot);
            // Resolve with the result pointer encoded as little-endian bytes.
            self.resolve.call1(&JsValue::NULL, &ptr_to_js(result_ptr as usize)).unwrap();
            // Break the self-cycle so everything can be freed.
            *self.this.borrow_mut() = None;
        }
    }

    /// Schedule a re-poll on the microtask queue (idempotent per wake).
    fn schedule(self: &Rc<Self>) {
        if *self.scheduled.borrow() {
            return;
        }
        *self.scheduled.borrow_mut() = true;
        let this = self.clone();
        let cb = Closure::once_into_js(move || this.poll());
        queue_microtask(&cb);
    }

    /// Build a `Waker` from this `Rc<DriveState>` via a hand-rolled vtable.
    /// `Waker::from_raw` is unsafe because it makes *us* assert the `Waker`
    /// `Send + Sync` contract by hand — there is no static check. This waker is
    /// **not** thread-safe: it is `Rc`-backed (non-atomic refcount) and its
    /// wake path calls `queueMicrotask`, a per-thread JS import. We satisfy the
    /// contract by *confinement*, not by thread-safety: the `DriveState` is
    /// created, polled, cloned, woken, and dropped entirely on the single
    /// worker thread running `__worxide_worker_entry_async`, and a safe future
    /// cannot move a `Waker` onto another thread in this environment (there is
    /// no `postMessage` for wakers and no usable `thread::spawn`). The vtable
    /// ops therefore never run cross-thread. An `Arc`-based waker would not help
    /// here — it would only make the refcount atomic while the payload stays
    /// `!Send` and the wake path stays thread-bound.
    fn into_waker(self: Rc<Self>) -> Waker {
        unsafe fn clone(data: *const ()) -> RawWaker {
            let rc = unsafe { Rc::from_raw(data as *const DriveState) };
            let cloned = rc.clone();
            std::mem::forget(rc);
            RawWaker::new(Rc::into_raw(cloned) as *const (), &VTABLE)
        }
        unsafe fn wake(data: *const ()) {
            let rc = unsafe { Rc::from_raw(data as *const DriveState) };
            rc.schedule();
        }
        unsafe fn wake_by_ref(data: *const ()) {
            let rc = unsafe { Rc::from_raw(data as *const DriveState) };
            rc.schedule();
            std::mem::forget(rc);
        }
        unsafe fn drop_fn(data: *const ()) { drop(unsafe { Rc::from_raw(data as *const DriveState) }); }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop_fn);
        let raw = Rc::into_raw(self) as *const ();
        // SAFETY: see the doc comment above. The waker never leaves its origin
        // (worker) thread, so the Rc-based vtable ops are never called
        // cross-thread, upholding the Waker Send + Sync contract by confinement.
        unsafe { Waker::from_raw(RawWaker::new(raw, &VTABLE)) }
    }
}

pub fn drive_to_promise(fut: Pin<Box<dyn Future<Output = *mut ()>>>) -> js_sys::Promise {
    // Stash the future in an Option so the (FnMut) Promise executor closure
    // can move it out exactly once into the shared state.
    let mut fut_holder = Some(fut);
    js_sys::Promise::new(&mut |resolve, _reject| {
        let fut = fut_holder.take().expect("Promise executor ran more than once");
        let state = Rc::new(DriveState {
            fut: Rc::new(RefCell::new(fut)),
            resolve,
            scheduled: RefCell::new(false),
            this: RefCell::new(None),
        });
        // Keep ourselves alive until the future completes.
        *state.this.borrow_mut() = Some(state.clone());
        state.poll();
    })
}

#[wasm_bindgen(inline_js = r#"
    export function __worxide_queue_microtask(cb) { queueMicrotask(cb); }
"#)]
extern "C" {
    #[wasm_bindgen(js_name = __worxide_queue_microtask)]
    fn queue_microtask(cb: &JsValue);
}

// These are #[wasm_bindgen] exports (so they end up callable in the worker's
// glue) and therefore must be `pub`. The leading `__worxide_` and their
// presence in this hidden module mark them as internal.
// The entry points take the task pointer as little-endian `&[u8]` and return
// the result pointer as little-endian `Vec<u8>` (a Uint8Array on the JS side).
// worker.js forwards these payloads verbatim and never converts them to a
// number.

/// Seed this thread's resolved glue-URL cache.
///
/// Called by `worker.js` immediately after `initSync`, passing the `glue_url`
/// the worker received over `postMessage`. This makes any *nested* spawn the
/// worker performs reuse the already-resolved URL instead of re-deriving it —
/// crucial because `window` / `globalThis.app_js_path` is not set on workers.
#[wasm_bindgen]
pub fn __worxide_seed_glue_url(url: String) { GLUE_URL.with(|cell| *cell.borrow_mut() = Some(url)); }

/// Worker thread entry point for sync tasks. Returns the result pointer bytes,
/// or `Err` (thrown to JS and caught by worker.js as a per-call error envelope) if
/// the payload is malformed — see [`decode_task_ptr`].
#[wasm_bindgen]
pub fn __worxide_worker_entry(ptr_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    let task_ptr = decode_task_ptr(ptr_bytes)?;
    // SAFETY: pointer came from WorkerTask::into_ptr on the main thread.
    let task = unsafe { WorkerTask::from_ptr(task_ptr) };
    Ok((task.run() as usize).to_le_bytes().to_vec())
}

/// Worker thread entry point for async tasks. Returns a Promise that
/// resolves with the result pointer bytes once the task's future completes.
/// We deliberately avoid `wasm_bindgen_futures::future_to_promise`
/// `spawn_local` here. With atomics enabled (required for shared memory),
/// wasm-bindgen-futures selects its *multithread* executor, which schedules
/// wakeups through `Atomics.waitAsync` and a coordinator worker. That
/// machinery assumes a persistent thread-pool runtime we don't have — in our
/// one-shot worker it produces an undefined wakeup promise and dies. Instead
/// we drive the future ourselves on the worker's own event loop, rescheduling
/// polls via `queueMicrotask`. JsFuture wakeups (Promise.then callbacks) run
/// on that same event loop, so progress happens without any cross-thread
/// futex coordination.
#[wasm_bindgen]
pub fn __worxide_worker_entry_async(ptr_bytes: &[u8]) -> Result<js_sys::Promise, JsValue> {
    // Throw synchronously on a malformed payload (caught by worker.js as a
    // per-call error envelope). Returning `Result` rather than calling
    // `Promise::reject` keeps the error path on the existing `__wbindgen_throw`
    // import instead of pulling in a fresh glue shim.
    let task_ptr = decode_task_ptr(ptr_bytes)?;
    // SAFETY: pointer came from AsyncWorkerTask::into_ptr on the main thread.
    let task = unsafe { AsyncWorkerTask::from_ptr(task_ptr) };
    Ok(drive_to_promise(task.run()))
}
