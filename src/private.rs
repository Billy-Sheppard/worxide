//! Internal implementation details for worxide.
//!
//! This module is `#[doc(hidden)]` and exists only so the `spawn!` and
//! `spawn_blocking!` macros have something to call. It is NOT part of the
//! public API and provides no stability guarantees — do not reference any of
//! it directly. (Rust macros that span crate boundaries require their callees
//! to be `pub`, so this can't be a private module; `#[doc(hidden)]` keeps it
//! out of the generated docs. This is the same pattern serde, wasm-bindgen,
//! and friends use for their macro-support modules.)

use anyhow::{Context, Result, anyhow};
use js_sys::{Promise, Reflect};
use std::cell::RefCell;
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::str::FromStr;
use std::task::{Poll, RawWaker, RawWakerVTable, Waker};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, MessageEvent, Url, Worker, WorkerOptions, WorkerType};

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

fn worker_url() -> Result<String> {
    // Create a fresh Blob URL on each spawn. Browsers may revoke or restrict Blob URLs used by previous Workers, so don't cache.
    let array = js_sys::Array::new();
    array.push(&JsValue::from_str(WORKER_JS));
    let opts = BlobPropertyBag::new();
    opts.set_type("application/javascript");
    let blob = Blob::new_with_str_sequence_and_options(&array, &opts)
        .map_err(|e| js_err("Blob construction failed", e))?;
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
    pub fn run(self) -> *mut () {
        (self.func)()
    }
    pub fn into_ptr(self) -> usize {
        Box::into_raw(Box::new(self)) as usize
    }

    /// # Safety: ptr must come from `into_ptr` and not be freed yet.
    pub unsafe fn from_ptr(ptr: usize) -> Box<WorkerTask> {
        unsafe { Box::from_raw(ptr as *mut WorkerTask) }
    }
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
        Self {
            func: Box::new(move || Box::pin(f())),
        }
    }
    pub fn run(self) -> Pin<Box<dyn Future<Output = *mut ()>>> {
        (self.func)()
    }
    pub fn into_ptr(self) -> usize {
        Box::into_raw(Box::new(self)) as usize
    }
    /// # Safety: ptr must come from `into_ptr` and not be freed yet.
    pub unsafe fn from_ptr(ptr: usize) -> Box<AsyncWorkerTask> {
        unsafe { Box::from_raw(ptr as *mut AsyncWorkerTask) }
    }
}

pub fn box_result<T: Send + 'static>(value: T) -> *mut () {
    Box::into_raw(Box::new(value)) as *mut ()
}

/// # Safety: ptr must come from `box_result::<T>` and not be freed yet.
pub unsafe fn unbox_result<T: Send + 'static>(ptr: *mut ()) -> T {
    unsafe { *Box::from_raw(ptr as *mut T) }
}

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
    let arr = v
        .dyn_ref::<js_sys::Uint8Array>()
        .context("worker reply was not a Uint8Array")?;
    let bytes = arr.to_vec();
    let buf: [u8; PTR_LEN] = bytes.as_slice().try_into().map_err(|_| {
        anyhow!(
            "worker reply was {} bytes, expected a {PTR_LEN}-byte pointer",
            bytes.len()
        )
    })?;
    Ok(usize::from_le_bytes(buf))
}

/// Decode a pointer handed to a worker entry point as `&[u8]`.
fn decode_task_ptr(bytes: &[u8]) -> usize {
    let buf: [u8; PTR_LEN] = bytes
        .try_into()
        .expect("worxide: task pointer payload had the wrong length");
    usize::from_le_bytes(buf)
}

fn js_err(context: &'static str, v: JsValue) -> anyhow::Error {
    // Try a sequence of strategies to get something readable.
    // 1. If it's already a string.
    if let Some(s) = v.as_string() {
        return anyhow!("{context}: {s}");
    }
    // 2. DOMException / Error — read .name + .message via Reflect.
    let name = js_sys::Reflect::get(&v, &"name".into())
        .ok()
        .and_then(|x| x.as_string());
    let msg = js_sys::Reflect::get(&v, &"message".into())
        .ok()
        .and_then(|x| x.as_string());
    if name.is_some() || msg.is_some() {
        let n = name.as_deref().unwrap_or("Error");
        let m = msg.as_deref().unwrap_or("(no message)");
        return anyhow!("{context}: {n}: {m}");
    }
    // 3. Fall back to JSON.stringify.
    let stringified = js_sys::JSON::stringify(&v)
        .ok()
        .and_then(|s| s.as_string())
        .unwrap_or_else(|| "<unprintable JsValue>".to_owned());
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
) -> Result<(Worker, Promise)> {
    let opts = WorkerOptions::new();
    opts.set_type(WorkerType::Module);
    let worker = Worker::new_with_options(&worker_url, &opts)
        .map_err(|e| js_err("Worker construction failed", e))?;

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
    Reflect::set(&msg, &"module".into(), &module)
        .map_err(|e| js_err("set module on message", e))?;
    Reflect::set(&msg, &"memory".into(), &memory)
        .map_err(|e| js_err("set memory on message", e))?;
    // Pointer travels as little-endian bytes (a Uint8Array), never as an f64.
    Reflect::set(&msg, &"ptr".into(), &ptr_to_js(task_ptr))
        .map_err(|e| js_err("set ptr on message", e))?;
    // The *resolved* glue URL goes over the wire so the worker (and any nested
    // spawn it makes) never has to re-derive it from a crate name or a global
    // it can't see. worker.js feeds this to `__worxide_seed_glue_url`.
    Reflect::set(&msg, &"glue_url".into(), &JsValue::from_str(&glue_url))
        .map_err(|e| js_err("set glue_url on message", e))?;

    // Once postMessage succeeds, ownership of the boxed task has passed to
    // the worker thread; the worker is responsible for freeing it.
    worker
        .post_message(&msg)
        .map_err(|e| js_err("postMessage to worker failed", e))?;
    Ok((worker, promise))
}

/// Spawn a worker, post the task pointer, await the reply.
/// `kind` is "sync" or "async" — tells worker.js which entry to call.
async fn run_inner(task_ptr: usize, kind: WorkerExecution, crate_name: &str) -> Result<usize> {
    let glue_url = cached_glue_url(crate_name);
    let worker_url = worker_url()?;
    let module = wasm_bindgen::module();
    let memory = wasm_bindgen::memory();

    let (worker, promise) =
        match construct_worker(task_ptr, kind, glue_url, module, memory, worker_url).await {
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

    let resolved = JsFuture::from(promise)
        .await
        .map_err(|e| js_err("worker future rejected", e))?;
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
            self.resolve
                .call1(&JsValue::NULL, &ptr_to_js(result_ptr as usize))
                .unwrap();
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
        unsafe fn drop_fn(data: *const ()) {
            drop(unsafe { Rc::from_raw(data as *const DriveState) });
        }
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
        let fut = fut_holder
            .take()
            .expect("Promise executor ran more than once");
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
pub fn __worxide_seed_glue_url(url: String) {
    GLUE_URL.with(|cell| *cell.borrow_mut() = Some(url));
}

/// Worker thread entry point for sync tasks. Returns the result pointer bytes.
#[wasm_bindgen]
pub fn __worxide_worker_entry(ptr_bytes: &[u8]) -> Vec<u8> {
    let task_ptr = decode_task_ptr(ptr_bytes);
    // SAFETY: pointer came from WorkerTask::into_ptr on the main thread.
    let task = unsafe { WorkerTask::from_ptr(task_ptr) };
    (task.run() as usize).to_le_bytes().to_vec()
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
pub fn __worxide_worker_entry_async(ptr_bytes: &[u8]) -> js_sys::Promise {
    let task_ptr = decode_task_ptr(ptr_bytes);
    // SAFETY: pointer came from AsyncWorkerTask::into_ptr on the main thread.
    let task = unsafe { AsyncWorkerTask::from_ptr(task_ptr) };
    drive_to_promise(task.run())
}
