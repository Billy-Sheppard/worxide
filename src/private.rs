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

/// Worker bootstrap source, embedded at compile time. Turned into a fresh Blob
/// URL for each spawned worker so consumers do not need to serve a worker file.
const WORKER_JS: &str = include_str!("worker.js");

/// Width of a pointer on the target, in bytes.
///
/// Pointers cross the worker boundary as exactly this many little-endian bytes
/// (see [`ptr_to_js`], [`ptr_from_js`], and [`decode_task_ptr`]), never as JS
/// numbers. This keeps the transport honest — a pointer is an address, not an
/// `f64` — and stays correct for wasm64 / memory64 where a pointer may exceed
/// the 2^53 exact-integer range of a JavaScript `Number`.
const PTR_LEN: usize = std::mem::size_of::<usize>();

thread_local! {
    /// Cached glue-file URL, resolved once per thread.
    ///
    /// On the main thread this is resolved on first spawn. On a worker it is
    /// seeded by `worker.js` via [`__worxide_seed_glue_url`], so nested spawns
    /// reuse the already-resolved app glue URL instead of trying to derive it
    /// from page globals that are unavailable inside workers.
    static GLUE_URL: RefCell<Option<String>> = const { RefCell::new(None) };
}

// Glue-URL helpers.
//
//  * `worxide_glue_url`           — derives the app glue file from a crate name,
//                                   resolved relative to this wasm-bindgen snippet.
//  * `worxide_glue_url_from_path` — resolves an explicit consumer-provided path
//                                   or URL against the document base.
//  * `worxide_app_js_path`        — optional `globalThis.app_js_path` set by the
//                                   consumer page.
//
// Workers receive the already-resolved URL over `postMessage`, then seed
// `GLUE_URL`; they should not need these resolver helpers themselves.
#[wasm_bindgen(inline_js = r#"
    export function worxide_glue_url(crate_name) {
        // Cargo replaces hyphens with underscores in generated library filenames.
        // Strip a trailing ".js" so callers may pass either a crate-ish name or
        // an already-js-looking filename stem.
        const file = crate_name.replace(/-/g, "_").replace(/\.js$/, "");
        return new URL("../../" + file + ".js", import.meta.url).href;
    }
    export function worxide_glue_url_from_path(path) {
        // Resolve explicit consumer paths/URLs against the page base. Do not
        // mangle these the way crate names are mangled.
        return new URL(path, document.baseURI).href;
    }
    export function worxide_app_js_path() {
        // Optional consumer-set global, e.g.:
        //   globalThis.app_js_path = "my_app.js";
        //   globalThis.app_js_path = "/static/my_app.js";
        const p = globalThis.app_js_path;
        return (typeof p === "string" && p.length > 0) ? p : null;
    }
"#)]
extern "C" {
    fn worxide_glue_url(crate_name: &str) -> String;
    fn worxide_glue_url_from_path(path: &str) -> String;
    fn worxide_app_js_path() -> Option<String>;
}

/// Resolve and memoize the glue-file URL for this thread.
///
/// Precedence:
///   1. consumer-set `globalThis.app_js_path`, resolved against the page base;
///   2. the crate name captured by the macro via `CARGO_PKG_NAME`.
///
/// Resolution happens once. If `globalThis.app_js_path` is used, it must be set
/// before the first spawn. On workers, `GLUE_URL` is already seeded by
/// `worker.js`, so workers avoid page-global lookup entirely.
fn cached_glue_url(crate_name: &str) -> String {
    GLUE_URL.with(|cell| {
        cell.borrow_mut()
            .get_or_insert_with(|| {
                if let Some(p) = worxide_app_js_path() {
                    worxide_glue_url_from_path(&p)
                } else {
                    worxide_glue_url(crate_name)
                }
            })
            .clone()
    })
}

/// Resolve a glue URL for a persistent [`Worker`] without the macro crate-name
/// fallback.
///
/// Persistent worker construction has no macro call site and therefore no
/// captured `CARGO_PKG_NAME`. It uses an already-cached URL if one exists, or
/// the consumer-set `globalThis.app_js_path`; otherwise the caller must use
/// [`Worker::with_glue`].
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
    // Create a fresh Blob URL on each spawn. Browsers may revoke or restrict
    // previous Blob URLs used by Workers, so do not cache this URL. Callers
    // revoke it immediately after worker construction succeeds or fails.
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

    pub fn into_ptr(self) -> usize { Box::into_raw(Box::new(self)).expose_provenance() }

    /// # Safety
    ///
    /// `ptr` must come from [`WorkerTask::into_ptr`] and must not have been
    /// freed or consumed yet.
    pub unsafe fn from_ptr(ptr: usize) -> Box<WorkerTask> {
        unsafe { Box::from_raw(std::ptr::with_exposed_provenance_mut::<WorkerTask>(ptr)) }
    }
}

/// Async task: closure returns a future that resolves to the boxed result.
///
/// The outer closure carries captured arguments across the worker boundary, so
/// it is `Send`. The future it creates is constructed and driven entirely on the
/// worker thread and is deliberately not required to be `Send`.
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

    pub fn into_ptr(self) -> usize { Box::into_raw(Box::new(self)).expose_provenance() }

    /// # Safety
    ///
    /// `ptr` must come from [`AsyncWorkerTask::into_ptr`] and must not have been
    /// freed or consumed yet.
    pub unsafe fn from_ptr(ptr: usize) -> Box<AsyncWorkerTask> {
        unsafe { Box::from_raw(std::ptr::with_exposed_provenance_mut::<AsyncWorkerTask>(ptr)) }
    }
}

pub fn box_result<T: Send + 'static>(value: T) -> *mut () { Box::into_raw(Box::new(value)).cast::<()>() }

/// # Safety
///
/// `ptr` must come from `box_result::<T>` and must not have been freed or
/// consumed yet.
pub unsafe fn unbox_result<T: Send + 'static>(ptr: usize) -> T {
    unsafe { *Box::from_raw(std::ptr::with_exposed_provenance_mut::<T>(ptr)) }
}

/// Free a result box produced by [`box_result`] as its concrete `R`, without
/// returning the value.
///
/// This is used on cancellation / abandon paths: the awaiting future that would
/// normally call [`unbox_result`] is gone, but the worker still produced a box
/// that must be dropped.
///
/// # Safety
///
/// `ptr` must come from `box_result::<R>` and must not have been freed or
/// consumed yet.
unsafe fn free_result_box<R: Send + 'static>(ptr: usize) {
    drop(unsafe { Box::from_raw(std::ptr::with_exposed_provenance_mut::<R>(ptr)) });
}

// Pointer addresses cross `postMessage` as exposed-provenance addresses encoded
// in little-endian bytes, never as `f64`.
//
// Raw pointer -> address uses `expose_provenance()`.
// Address -> raw pointer uses `with_exposed_provenance_mut::<T>()`.
//
// This makes the intended provenance model explicit. The unsafe invariant is
// still the usual `Box::from_raw` one: the address must come from the matching
// `Box::into_raw`, for the same concrete type, and must be consumed exactly once.

/// Encode a pointer address as a fresh `Uint8Array` of little-endian bytes.
fn ptr_to_js(ptr: usize) -> JsValue {
    // `Uint8Array::from(&[u8])` allocates its own non-shared buffer and copies
    // the bytes. It is not a view over wasm linear memory, so structured clone
    // copies these few bytes rather than entangling the backing SharedArrayBuffer.
    // Do not switch this to `Uint8Array::view`.
    js_sys::Uint8Array::from(&ptr.to_le_bytes()[..]).into()
}

/// Decode a pointer address previously produced by [`ptr_to_js`].
fn ptr_from_js(v: &JsValue) -> Result<usize> {
    let arr = v.dyn_ref::<js_sys::Uint8Array>().context("worker reply was not a Uint8Array")?;
    let bytes = arr.to_vec();
    let buf: [u8; PTR_LEN] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("worker reply was {} bytes, expected a {PTR_LEN}-byte pointer", bytes.len()))?;
    Ok(usize::from_le_bytes(buf))
}

/// Encode a 64-bit call id as little-endian bytes.
///
/// The id is echoed by `worker.js` in the matching `result` envelope. Bytes are
/// used rather than an `f64` so ids round-trip losslessly past 2^53; a collision
/// would route a result to the wrong call and potentially unbox it as the wrong
/// type.
fn id_to_js(id: u64) -> JsValue { js_sys::Uint8Array::from(&id.to_le_bytes()[..]).into() }

/// Decode a call id produced by [`id_to_js`].
fn id_from_js(v: &JsValue) -> Option<u64> {
    let arr = v.dyn_ref::<js_sys::Uint8Array>()?;
    let buf: [u8; 8] = arr.to_vec().as_slice().try_into().ok()?;
    Some(u64::from_le_bytes(buf))
}

/// Decode a task pointer payload handed to a worker entry point.
///
/// Wrong-length payloads return `Err`, which `worker.js` reports as a per-call
/// error envelope. This avoids aborting the whole worker instance for malformed
/// protocol data.
fn decode_task_ptr(bytes: &[u8]) -> Result<usize, JsValue> {
    let buf: [u8; PTR_LEN] =
        bytes.try_into().map_err(|_| JsValue::from_str("worxide: task pointer payload had the wrong length"))?;
    Ok(usize::from_le_bytes(buf))
}

fn js_err(context: &'static str, v: JsValue) -> anyhow::Error {
    if let Some(s) = v.as_string() {
        return anyhow!("{context}: {s}");
    }

    let name = Reflect::get(&v, &"name".into()).ok().and_then(|x| x.as_string());
    let msg = Reflect::get(&v, &"message".into()).ok().and_then(|x| x.as_string());

    if name.is_some() || msg.is_some() {
        let n = name.as_deref().unwrap_or("Error");
        let m = msg.as_deref().unwrap_or("(no message)");
        return anyhow!("{context}: {n}: {m}");
    }

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
            other => Err(anyhow!("`{other}` is not a valid WorkerExecution!")),
        }
    }
}

async fn construct_worker(
    task_ptr: usize,
    kind: WorkerExecution,
    glue_url: String,
    module: JsValue,
    memory: JsValue,
    worker_url: &str,
) -> Result<(WebWorker, Promise)> {
    let opts = WorkerOptions::new();
    opts.set_type(WorkerType::Module);

    let worker = WebWorker::new_with_options(worker_url, &opts).map_err(|e| js_err("Worker construction failed", e))?;

    // Note: `Closure::once_into_js` is deliverately avoided here as its owning
    // `ScopedClosure` can be double-freed if not called, which corrupts the heap.
    // An owned `FnMut` closure released with `forget()` has exactly one path
    // to destruction, so it is safe whether the callback fires or not.
    // See: `can't access property "_wbg_cb_unref", arg0 is null`
    let setup = (|| -> Result<Promise> {
        let (promise, resolve, reject) = new_promise()?;

        let on_message = {
            let resolve = resolve.clone();
            Closure::<dyn FnMut(MessageEvent)>::new(move |evt: MessageEvent| {
                // The reply is a Uint8Array of little-endian pointer bytes.
                // Forward it verbatim; run_inner decodes and validates it via
                // ptr_from_js (a non-Uint8Array reply surfaces as an Err there).
                resolve.call1(&JsValue::NULL, &evt.data()).unwrap();
            })
        };
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();

        let on_error = {
            let reject = reject.clone();
            Closure::<dyn FnMut(JsValue)>::new(move |evt: JsValue| {
                reject.call1(&JsValue::NULL, &evt).unwrap();
            })
        };
        worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        on_error.forget();

        let msg = js_sys::Object::new();

        Reflect::set(&msg, &"kind".into(), &JsValue::from_str(&kind.to_string()))
            .map_err(|e| js_err("set kind on message", e))?;
        Reflect::set(&msg, &"module".into(), &module).map_err(|e| js_err("set module on message", e))?;
        Reflect::set(&msg, &"memory".into(), &memory).map_err(|e| js_err("set memory on message", e))?;
        Reflect::set(&msg, &"ptr".into(), &ptr_to_js(task_ptr)).map_err(|e| js_err("set ptr on message", e))?;
        Reflect::set(&msg, &"glue_url".into(), &JsValue::from_str(&glue_url))
            .map_err(|e| js_err("set glue_url on message", e))?;

        // Once this succeeds, ownership of the task box has passed to the
        // worker. If anything before this fails, the caller reclaims the task.
        worker.post_message(&msg).map_err(|e| js_err("postMessage to worker failed", e))?;

        Ok(promise)
    })();

    match setup {
        Ok(promise) => Ok((worker, promise)),
        Err(e) => {
            worker.terminate();
            Err(e)
        }
    }
}

/// Spawn a one-shot worker, post the task pointer, await the reply, then
/// terminate the worker.
async fn run_inner(task_ptr: usize, kind: WorkerExecution, crate_name: &str) -> Result<usize> {
    let glue_url = cached_glue_url(crate_name);
    let worker_url = worker_url()?;
    let module = wasm_bindgen::module();
    let memory = wasm_bindgen::memory();

    let constructed = construct_worker(task_ptr, kind, glue_url, module, memory, &worker_url).await;
    Url::revoke_object_url(&worker_url).ok();

    let (worker, promise) = match constructed {
        Ok(tuple) => tuple,
        Err(e) => {
            // Worker construction/setup failed before ownership of the task
            // crossed the boundary. Reclaim it as the same concrete task type.
            unsafe {
                match kind {
                    WorkerExecution::Sync => drop(WorkerTask::from_ptr(task_ptr)),
                    WorkerExecution::Async => drop(AsyncWorkerTask::from_ptr(task_ptr)),
                }
            }
            return Err(e);
        }
    };

    let resolved = match JsFuture::from(promise).await {
        Ok(value) => value,
        Err(e) => {
            worker.terminate();
            return Err(js_err("worker future rejected", e));
        }
    };

    worker.terminate();

    // `resolved` is a structured-clone copy of the worker's Uint8Array, owned
    // by this thread, so decoding it after `terminate()` is fine.
    ptr_from_js(&resolved)
}

/// Submit a sync closure to a one-shot worker and await its result. `R` is
/// inferred from the closure's return type.
pub async fn run_blocking<F, R>(f: F, crate_name: &str) -> Result<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let task = WorkerTask::new(move || box_result(f()));
    let result_ptr = run_inner(task.into_ptr(), WorkerExecution::Sync, crate_name).await?;

    // SAFETY: result_ptr came from `box_result::<R>` on the worker, and `R: Send`
    // permits taking ownership back on this thread.
    Ok(unsafe { unbox_result::<R>(result_ptr) })
}

/// Submit an async closure to a one-shot worker and await its result.
///
/// Note: `Fut` is intentionally not required to be `Send`. The future is created
/// and polled entirely on the worker thread. This relies on [`drive_to_promise`]
/// being a worker-local executor.
pub async fn run_async<F, Fut, R>(f: F, crate_name: &str) -> Result<R>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = R> + 'static,
    R: Send + 'static,
{
    let task = AsyncWorkerTask::new(move || async move { box_result(f().await) });
    let result_ptr = run_inner(task.into_ptr(), WorkerExecution::Async, crate_name).await?;

    // SAFETY: result_ptr came from `box_result::<R>` on the worker, and `R: Send`
    // permits taking ownership back on this thread.
    Ok(unsafe { unbox_result::<R>(result_ptr) })
}

// ===========================================================================
// Persistent worker handle
// ===========================================================================
//
// `spawn!` / `spawn_blocking!` create a worker, run one task, and terminate it.
// `Worker` boots one worker, attaches it to this thread's shared memory, and
// keeps it alive so many tasks can run over the same instance.
//
// Protocol:
//   main -> worker  { type: "init",  module, memory, glue_url } + [control port]
//   worker -> port  { type: "ready" }
//   main -> port    { type: "call",  id, kind, ptr }
//   worker -> port  { type: "result", id, result }  // success
//   worker -> port  { type: "result", id, error }   // per-call failure
//
// Only the initial `init` envelope uses the worker's default postMessage
// channel. It transfers a private `MessagePort`; all worxide `ready` / `call` /
// `result` traffic runs over that port. The worker's default channel is then
// left available to the consumer via `worker_handle()`.

/// Where a call's outcome lands.
///
/// `on_message` deposits into the shared slot and fires the call's signal; the
/// awaiting `dispatch` future takes the outcome on resume. If that future was
/// dropped first, [`AbandonGuard`] ensures any deposited result box is freed.
enum Slot {
    /// No reply yet.
    Waiting,
    /// Result pointer, decoded from the reply, not yet taken.
    Ok(usize),
    /// Task threw, worker died, or the reply was malformed.
    Err(String),
    /// Awaiter is gone; `on_message` must free any successful result itself.
    Abandoned,
    /// Outcome consumed by the awaiter.
    Done,
}

/// One in-flight persistent-worker call.
///
/// Stored in `Worker::pending` under the call id until a matching `result`
/// envelope arrives or the worker dies.
struct Pending {
    /// Shared with the awaiting `dispatch` future and its [`AbandonGuard`].
    slot: Rc<RefCell<Slot>>,
    /// Resolve function for the call's signal promise. It carries no payload;
    /// the actual outcome is stored in `slot`.
    signal: js_sys::Function,
    /// Frees a deposited result as its concrete `R`. Used only when the call was
    /// abandoned; the normal live path unboxes in `run` / `run_blocking`.
    free_result: unsafe fn(usize),
}

/// RAII guard held across `dispatch`'s await.
///
/// If the awaiting future is dropped before it takes the outcome, this prevents
/// leaking the result box:
///
///   * if `Slot::Ok` already landed, the guard frees it;
///   * if the call is still in flight, the guard marks the slot `Abandoned`, so
///     the late `result` envelope is freed by `on_message`.
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
        if let Slot::Ok(ptr) = std::mem::replace(&mut *self.slot.borrow_mut(), Slot::Abandoned) {
            // SAFETY: the pointer came from `box_result::<R>` on the worker and
            // `free_result` is the matching deallocator for that concrete `R`.
            unsafe { (self.free_result)(ptr) }
        }
    }
}

/// Create a fresh JS `Promise`, returning it with the executor's `resolve` and
/// `reject` functions.
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

/// Reject and wake every live pending call.
///
/// Used when a persistent worker dies or is explicitly terminated. A dead worker
/// produces no result box, so abandoned calls need no extra freeing here.
fn reject_pending_calls(pending: &Rc<RefCell<HashMap<u64, Pending>>>, why: String) {
    let drained: Vec<Pending> = pending.borrow_mut().drain().map(|(_, p)| p).collect();

    for p in drained {
        let mut slot = p.slot.borrow_mut();

        if !matches!(&*slot, Slot::Abandoned) {
            *slot = Slot::Err(why.clone());
            drop(slot);
            p.signal.call0(&JsValue::NULL).ok();
        }
    }
}

/// Read the `type` discriminator from a JS envelope, if present.
fn envelope_type(data: &JsValue) -> Option<String> {
    Reflect::get(data, &JsValue::from_str("type")).ok().and_then(|v| v.as_string())
}

/// A persistent Web Worker attached to this thread's shared memory.
///
/// Construct it with [`Worker::new`] using `globalThis.app_js_path`, or with
/// [`Worker::with_glue`] using an explicit glue path/URL.
///
/// There is no `spawn_persistent!` macro: once you hold a handle, run work with
/// [`Worker::run_blocking`] for sync CPU-bound work and [`Worker::run`] for async
/// work. Arguments and results cross by pointer through shared memory.
///
/// Dropping the handle terminates the worker.
pub struct Worker {
    inner: WebWorker,
    // Our end of the private control channel. The other end is transferred to
    // the worker at boot. All worxide `ready` / `call` / `result` envelopes use
    // this port, isolated from the consumer's use of `inner`.
    port: MessagePort,
    pending: Rc<RefCell<HashMap<u64, Pending>>>,
    next_id: Rc<Cell<u64>>,
    dead: Rc<Cell<bool>>,
    // Retained so the listeners outlive `boot` and keep firing for the life of
    // the handle. Dropping the handle drops these closures and terminates the
    // worker. NOT `forget()`-ed: persistent workers keep their handlers alive by
    // owning them, not by leaking them.
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(JsValue)>,
}

impl Worker {
    /// Spawn a persistent worker, attach it to this thread's shared memory, and
    /// resolve once it reports ready.
    ///
    /// Uses `globalThis.app_js_path` unless a glue URL was already cached. Use
    /// [`Worker::with_glue`] if the page does not set `app_js_path`.
    pub async fn new() -> Result<Self> { Self::boot(glue_url_via_app_path()?).await }

    /// Like [`Worker::new`], but with an explicit glue path/URL resolved against
    /// the document base.
    pub async fn with_glue(glue_path: &str) -> Result<Self> { Self::boot(glue_url_explicit(glue_path)).await }

    async fn boot(glue_url: String) -> Result<Self> {
        let worker_url = worker_url()?;
        let module = wasm_bindgen::module();
        let memory = wasm_bindgen::memory();

        let opts = WorkerOptions::new();
        opts.set_type(WorkerType::Module);

        let inner = match WebWorker::new_with_options(&worker_url, &opts) {
            Ok(worker) => worker,
            Err(e) => {
                Url::revoke_object_url(&worker_url).ok();
                return Err(js_err("Worker construction failed", e));
            }
        };

        Url::revoke_object_url(&worker_url).ok();

        let setup = async {
            // Private control channel. After the worker adopts `worker_port`,
            // worxide never needs the worker's default channel again.
            let channel = MessageChannel::new().map_err(|e| js_err("MessageChannel construction failed", e))?;
            let port = channel.port1();
            let worker_port = channel.port2();

            let pending: Rc<RefCell<HashMap<u64, Pending>>> = Rc::new(RefCell::new(HashMap::new()));
            let next_id = Rc::new(Cell::new(0u64));
            let dead = Rc::new(Cell::new(false));

            // Resolves when the worker posts `{ type: "ready" }` after initSync.
            let (ready, resolve_ready, reject_ready) = new_promise()?;

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

                            // Unknown id: the call already settled, the awaiter
                            // abandoned it and it was removed, or this is a
                            // duplicate/late envelope.
                            let Some(p) = pending.borrow_mut().remove(&id) else {
                                return;
                            };

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
                                // Awaiter is gone. If this is a successful result,
                                // free it here instead of delivering into the void.
                                if let Slot::Ok(ptr) = outcome {
                                    // SAFETY: ptr came from `box_result::<R>` on
                                    // the worker; `free_result` is that same R's
                                    // deallocator.
                                    unsafe { (p.free_result)(ptr) };
                                }
                            } else {
                                *slot = outcome;
                                drop(slot);
                                p.signal.call0(&JsValue::NULL).ok();
                            }
                        }
                        _ => {}
                    }
                })
            };

            port.add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref())
                .map_err(|e| js_err("addEventListener(\"message\") on control port failed", e))?;

            // Required when listening via addEventListener. Buffered messages
            // are delivered once started, so this is race-free before `init`.
            port.start();

            let on_error = {
                let pending = pending.clone();
                let dead = dead.clone();
                let ready_reject = reject_ready.clone();

                Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
                    dead.set(true);
                    ready_reject.call1(&JsValue::NULL, &e).ok();

                    let why = e.as_string().unwrap_or_else(|| "worxide: worker error".to_owned());
                    reject_pending_calls(&pending, why);
                })
            };

            inner
                .add_event_listener_with_callback("error", on_error.as_ref().unchecked_ref())
                .map_err(|e| js_err("addEventListener(\"error\") failed", e))?;

            // Hand the worker the compiled module, shared memory, resolved glue
            // URL, and its end of the private control port. `worker.js` imports
            // the glue, calls initSync once, seeds GLUE_URL, and replies ready.
            let msg = js_sys::Object::new();

            Reflect::set(&msg, &"type".into(), &"init".into()).map_err(|e| js_err("set type on init", e))?;
            Reflect::set(&msg, &"module".into(), &module).map_err(|e| js_err("set module on init", e))?;
            Reflect::set(&msg, &"memory".into(), &memory).map_err(|e| js_err("set memory on init", e))?;
            Reflect::set(&msg, &"glue_url".into(), &JsValue::from_str(&glue_url))
                .map_err(|e| js_err("set glue_url on init", e))?;

            let transfer = js_sys::Array::of1(&worker_port);

            inner.post_message_with_transfer(&msg, &transfer).map_err(|e| js_err("postMessage(init) failed", e))?;

            JsFuture::from(ready).await.map_err(|e| js_err("worker init failed", e))?;

            Ok::<_, anyhow::Error>((port, pending, next_id, dead, on_message, on_error))
        }
        .await;

        match setup {
            Ok((port, pending, next_id, dead, on_message, on_error)) => {
                Ok(Self { inner, port, pending, next_id, dead, _on_message: on_message, _on_error: on_error })
            }
            Err(e) => {
                inner.terminate();
                Err(e)
            }
        }
    }

    /// Run a synchronous closure on the worker and await its result. `R` is
    /// inferred from the closure's return type.
    pub async fn run_blocking<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let task = WorkerTask::new(move || box_result(f()));
        let result_ptr = self.dispatch(task.into_ptr(), WorkerExecution::Sync, free_result_box::<R>).await?;

        // SAFETY: result_ptr came from `box_result::<R>` on the worker; `R: Send`
        // permits taking ownership back on this thread.
        Ok(unsafe { unbox_result::<R>(result_ptr) })
    }

    /// Run an asynchronous closure on the worker and await its result.
    ///
    /// Note: `Fut` is intentionally not required to be `Send`. The future is
    /// created and polled entirely on this persistent worker's event loop.
    pub async fn run<F, Fut, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = R> + 'static,
        R: Send + 'static,
    {
        let task = AsyncWorkerTask::new(move || async move { box_result(f().await) });
        let result_ptr = self.dispatch(task.into_ptr(), WorkerExecution::Async, free_result_box::<R>).await?;

        // SAFETY: result_ptr came from `box_result::<R>` on the worker; `R: Send`
        // permits taking ownership back on this thread.
        Ok(unsafe { unbox_result::<R>(result_ptr) })
    }

    /// Post a call envelope, register a pending entry, and await its outcome.
    ///
    /// If setup fails before `postMessage`, the worker never takes ownership and
    /// the task box is reclaimed here. If the awaiting future is dropped after
    /// posting, [`AbandonGuard`] handles result cleanup.
    async fn dispatch(&self, task_ptr: usize, kind: WorkerExecution, free_result: unsafe fn(usize)) -> Result<usize> {
        if self.dead.get() {
            // The worker will never take the task: reclaim it now.
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

        let setup = (|| -> Result<(Promise, AbandonGuard)> {
            let (signal_promise, signal, _) = new_promise()?;

            self.pending.borrow_mut().insert(id, Pending { slot: slot.clone(), signal, free_result });

            let guard = AbandonGuard { slot: slot.clone(), free_result, armed: Cell::new(true) };

            let msg = js_sys::Object::new();

            Reflect::set(&msg, &"type".into(), &"call".into()).map_err(|e| js_err("set type on call", e))?;
            Reflect::set(&msg, &"id".into(), &id_to_js(id)).map_err(|e| js_err("set id on call", e))?;
            Reflect::set(&msg, &"kind".into(), &JsValue::from_str(&kind.to_string()))
                .map_err(|e| js_err("set kind on call", e))?;
            Reflect::set(&msg, &"ptr".into(), &ptr_to_js(task_ptr)).map_err(|e| js_err("set ptr on call", e))?;

            if let Err(e) = self.port.post_message(&msg) {
                guard.disarm();
                self.pending.borrow_mut().remove(&id);
                return Err(js_err("postMessage(call) failed", e));
            }

            Ok((signal_promise, guard))
        })();

        let (signal_promise, guard) = match setup {
            Ok(ok) => ok,
            Err(e) => {
                self.pending.borrow_mut().remove(&id);

                // The worker never took ownership: reclaim the task box.
                unsafe {
                    match kind {
                        WorkerExecution::Sync => drop(WorkerTask::from_ptr(task_ptr)),
                        WorkerExecution::Async => drop(AsyncWorkerTask::from_ptr(task_ptr)),
                    }
                }

                return Err(e);
            }
        };

        // The signal promise carries no payload; the outcome is in `slot`.
        // If this await is dropped, `guard` reaps or marks the call abandoned.
        JsFuture::from(signal_promise).await.ok();
        guard.disarm();

        match std::mem::replace(&mut *slot.borrow_mut(), Slot::Done) {
            Slot::Ok(ptr) => Ok(ptr),
            Slot::Err(why) => Err(anyhow!("{why}")),
            Slot::Waiting | Slot::Abandoned | Slot::Done => Err(anyhow!("worxide: worker reply was lost")),
        }
    }

    /// The underlying worker.
    ///
    /// Exposed so a consumer can `post_message_with_transfer` (for example, to
    /// hand the worker an `OffscreenCanvas`) and attach its own event listeners.
    ///
    /// This is safe to use freely: worxide's call/result protocol runs over a
    /// private [`MessagePort`] adopted at boot. The worker's default message
    /// channel carries none of worxide's protocol traffic after init, so consumer
    /// messages cannot collide with worxide dispatch.
    pub fn worker_handle(&self) -> &WebWorker { &self.inner }

    /// Terminate the worker immediately. Idempotent; also runs on drop.
    ///
    /// Any in-flight calls are rejected so their awaiting futures do not hang.
    /// Task boxes already handed to the worker remain owned by the worker; if
    /// the worker is killed mid-call, those boxes may leak, but no caller is left
    /// parked on a never-resolving signal.
    pub fn terminate(&self) {
        if !self.dead.replace(true) {
            reject_pending_calls(&self.pending, "worxide::Worker was terminated".to_owned());
        }

        self.port.close();
        self.inner.terminate();
    }
}

impl Drop for Worker {
    fn drop(&mut self) { self.terminate(); }
}

// Drives a future to completion on the worker's own event loop, returning a
// Promise that resolves with the future's `*mut ()` result as pointer bytes.
//
// This is intentionally a worker-local executor. `run_async` only requires the
// outer closure to be `Send`; the future it creates may be `!Send` because it is
// created, polled, woken, and dropped on this worker's event loop.
//
// We avoid `wasm_bindgen_futures::future_to_promise` here. With wasm atomics
// enabled, wasm-bindgen-futures may choose multithreaded scheduling machinery
// that assumes a runtime model worxide is not using. Instead, wakeups are
// rescheduled through this worker's own `queueMicrotask` queue.
//
// Safety note for the custom RawWaker below:
//
// `std::task::Waker` is `Send + Sync`, so a general RawWaker normally needs
// thread-safe data. This executor relies on worker-local confinement: the waker
// is only handed to futures polled on this worker thread, and worxide exposes no
// API that moves that waker to another worker. If that assumption changes, this
// must be replaced with an owner-thread wake routing mechanism.

type SharedFut = Rc<RefCell<Pin<Box<dyn Future<Output = *mut ()>>>>>;

struct DriveState {
    fut: SharedFut,
    resolve: js_sys::Function,

    // Coalesces multiple wakeups into one queued microtask. This is intentionally
    // `RefCell<bool>` rather than `AtomicBool` because the executor is local to
    // this worker thread. If wakeups can originate from another worker in the
    // future, routing back to the owner thread is required; merely replacing
    // `Rc` with `Arc` would not be sufficient for a `!Send` future.
    scheduled: RefCell<bool>,

    // Keeps the executor state alive while the future is pending.
    //
    // `drive_to_promise` creates an `Rc<DriveState>` and calls `state.poll()`.
    // If the future returns `Pending`, there may be no ordinary Rust owner left
    // after the Promise executor returns. This self-reference is the owning ref
    // that keeps the state alive until the future resolves.
    //
    // When the future returns `Ready`, `poll()` sets this to `None`, breaking the
    // cycle and allowing the state, future, and JS resolve function to be freed.
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
            self.resolve.call1(&JsValue::NULL, &ptr_to_js(result_ptr.expose_provenance())).unwrap();
            *self.this.borrow_mut() = None;
        }
    }

    /// Schedule a re-poll on the microtask queue (idempotent per wake).
    ///
    /// Avodiding using `Closure::once_into_js` as it can double-free,
    /// which corrupts the heap. See note in `construct_worker`
    fn schedule(self: &Rc<Self>) {
        if *self.scheduled.borrow() {
            return;
        }

        *self.scheduled.borrow_mut() = true;

        let this = self.clone();
        let cb = Closure::<dyn FnMut()>::new(move || this.poll());
        queue_microtask(cb.as_ref());
        cb.forget();
    }

    /// Build a `Waker` from this `Rc<DriveState>` via a hand-rolled vtable.
    ///
    /// `Waker::from_raw` is unsafe because a `Waker` is `Send + Sync`: code
    /// holding a `Waker` may clone it, store it, and call `wake()` from any
    /// thread. The usual implementation strategy is therefore an
    /// `Arc<T: Send + Sync>`.
    ///
    /// This executor is narrower than a general executor:
    ///
    /// - it is created inside one Web Worker;
    /// - the future is created on that same worker;
    /// - every poll happens via this worker's microtask queue;
    /// - the future is not required to be `Send`;
    /// - worxide does not expose the waker or any spawn API that can move it to
    ///   another worker.
    ///
    /// The `Rc`-based vtable is therefore justified by worker-local confinement.
    /// Safe futures may clone and later wake this waker, but in this runtime
    /// those wakeups are expected to occur on the same worker event loop.
    ///
    /// If worxide ever supports cross-worker wakeups, this must be redesigned:
    /// simply replacing `Rc` with `Arc` is insufficient because the `!Send`
    /// future must still be polled only on its owning worker thread.
    fn into_waker(self: Rc<Self>) -> Waker {
        // TODO(soundness): consider replacing this with a true owner-thread wake
        // mechanism once stable Rust exposes a non-Send `LocalWaker` for
        // `Context`, or if worxide grows an explicit cross-worker wake routing
        // mechanism.

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

        // SAFETY: see the doc comment above. Under worxide's current executor
        // model, this waker is confined to the worker thread that owns the
        // future, so the Rc-backed vtable is not invoked cross-thread.
        unsafe { Waker::from_raw(RawWaker::new(raw, &VTABLE)) }
    }
}

pub fn drive_to_promise(fut: Pin<Box<dyn Future<Output = *mut ()>>>) -> js_sys::Promise {
    // Stash the future in an Option so the Promise executor can move it out
    // exactly once into the driver state.
    let mut fut_holder = Some(fut);

    Promise::new(&mut |resolve, _reject| {
        let fut = fut_holder.take().expect("Promise executor ran more than once");

        let state = Rc::new(DriveState {
            fut: Rc::new(RefCell::new(fut)),
            resolve,
            scheduled: RefCell::new(false),
            this: RefCell::new(None),
        });

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

// These are #[wasm_bindgen] exports, so they are callable from generated worker
// glue and therefore must be `pub`. The `__worxide_` prefix and hidden module
// mark them as internal runtime ABI.
//
// Entry points take task pointer bytes and return result pointer bytes.
// `worker.js` forwards those payloads verbatim and never converts them to JS
// numbers.

/// Seed this thread's resolved glue-URL cache.
///
/// Called by `worker.js` immediately after `initSync`, passing the resolved
/// `glue_url` received over `postMessage`. This lets nested spawns from workers
/// reuse the app's glue URL without reading page globals.
#[wasm_bindgen]
pub fn __worxide_seed_glue_url(url: String) { GLUE_URL.with(|cell| *cell.borrow_mut() = Some(url)); }

/// Worker-thread entry point for sync tasks.
///
/// Returns result pointer bytes, or `Err` for malformed pointer payloads.
#[wasm_bindgen]
pub fn __worxide_worker_entry(ptr_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    let task_ptr = decode_task_ptr(ptr_bytes)?;
    // SAFETY: the pointer is produced by `WorkerTask::into_ptr` on the sending
    // thread and consumed exactly once by the worker protocol.
    let task = unsafe { WorkerTask::from_ptr(task_ptr) };
    Ok(task.run().expose_provenance().to_le_bytes().to_vec())
}

/// Worker-thread entry point for async tasks.
///
/// Returns a Promise that resolves with result pointer bytes once the task's
/// future completes.
///
/// This uses worxide's local future driver rather than
/// `wasm_bindgen_futures::future_to_promise`, because with atomics enabled the
/// wasm-bindgen-futures scheduler may assume a multithreaded runtime setup that
/// worxide is not using for these workers.
#[wasm_bindgen]
pub fn __worxide_worker_entry_async(ptr_bytes: &[u8]) -> Result<Promise, JsValue> {
    let task_ptr = decode_task_ptr(ptr_bytes)?;
    // SAFETY: the pointer is produced by `AsyncWorkerTask::into_ptr` on the
    // sending thread and consumed exactly once by the worker protocol.
    let task = unsafe { AsyncWorkerTask::from_ptr(task_ptr) };
    Ok(drive_to_promise(task.run()))
}
