//! worxide — spawn Rust functions on Web Workers via shared memory.

use wasm_bindgen::prelude::*;

/// Implementation details exposed for use by the `spawn!` / `spawn_blocking!`
/// macros only. Not part of the stable API — do not reference directly.
#[doc(hidden)]
pub mod __private {
    use anyhow::{anyhow, Context, Result};
    use js_sys::{Promise, Reflect};
    use std::future::Future;
    use std::pin::Pin;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Blob, BlobPropertyBag, MessageEvent, Url, Worker, WorkerOptions, WorkerType};

    /// Worker bootstrap source, embedded at compile time. Turned into a Blob
    /// URL on first spawn so the worker has nothing to serve from disk.
    const WORKER_JS: &str = include_str!("worker.js");

    /// Sync task: closure returns the boxed result pointer directly.
    pub struct WorkerTask {
        func: Box<dyn FnOnce() -> *mut ()>,
    }

    impl WorkerTask {
        pub fn new<F>(f: F) -> Self
        where
            F: FnOnce() -> *mut () + 'static,
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
            Box::from_raw(ptr as *mut WorkerTask)
        }
    }

    /// Async task: closure returns a future that resolves to the boxed result.
    pub struct AsyncWorkerTask {
        func: Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = *mut ()>>>>,
    }

    impl AsyncWorkerTask {
        pub fn new<F, Fut>(f: F) -> Self
        where
            F: FnOnce() -> Fut + 'static,
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
            Box::from_raw(ptr as *mut AsyncWorkerTask)
        }
    }

    pub fn box_result<T: 'static>(value: T) -> *mut () {
        Box::into_raw(Box::new(value)) as *mut ()
    }

    /// # Safety: ptr must come from `box_result::<T>` and not be freed yet.
    pub unsafe fn unbox_result<T: 'static>(ptr: *mut ()) -> T {
        *Box::from_raw(ptr as *mut T)
    }

    /// JS snippet bundled into the wasm-bindgen output. Resolves the URL of
    /// the consumer's wasm-bindgen glue file. The consumer's crate name is
    /// passed in by the macro via env!("CARGO_PKG_NAME").
    ///
    /// The snippet lives at `snippets/worxide-<hash>/inline0.js`, two dirs
    /// below the wasm-bindgen output root — hence the `../../` prefix.
    #[wasm_bindgen(inline_js = r#"
        export function worxide_glue_url(crate_name) {
            // Cargo replaces hyphens with underscores in library output filenames,
            // so a crate named "my-app" produces "my_app.js" / "my_app_bg.wasm".
            const file_name = crate_name.replace(/-/g, "_");
            return new URL("../../" + file_name + ".js", import.meta.url).href;
        }
    "#)]
    extern "C" {
        fn worxide_glue_url(crate_name: &str) -> String;
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

    static mut GLUE_URL: Option<String> = None;

    fn cached_glue_url(crate_name: &str) -> &'static str {
        // SAFETY: single-threaded main wasm thread, never mutated after init.
        // We DO NOT use ptr::read here — for a non-Copy type like Option<String>
        // that would clone the bookkeeping then drop it, freeing the heap buffer
        // out from under us. Instead we read via a const-pointer dereference,
        // which gives us a reference without moving.
        unsafe {
            if (*std::ptr::addr_of!(GLUE_URL)).is_none() {
                GLUE_URL = Some(worxide_glue_url(crate_name));
            }
            (*std::ptr::addr_of!(GLUE_URL)).as_deref().unwrap()
        }
    }

    fn worker_url() -> Result<String> {
        // Create a fresh Blob URL on each spawn. Browsers may revoke or
        // restrict Blob URLs used by previous Workers, so we don't cache.
        let array = js_sys::Array::new();
        array.push(&JsValue::from_str(WORKER_JS));
        let opts = BlobPropertyBag::new();
        opts.set_type("application/javascript");
        let blob = Blob::new_with_str_sequence_and_options(&array, &opts)
            .map_err(|e| js_err("Blob construction failed", e))?;
        Url::create_object_url_with_blob(&blob).map_err(|e| js_err("URL.createObjectURL failed", e))
    }

    /// Spawn a worker, post the task pointer, await the reply.
    /// `kind` is "sync" or "async" — tells worker.js which entry to call.
    async fn run_inner(task_ptr: usize, kind: &str, crate_name: &str) -> Result<usize> {
        let glue_url = cached_glue_url(crate_name).to_owned();
        let worker_url = worker_url()?;
        let module = wasm_bindgen::module();
        let memory = wasm_bindgen::memory();

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
            let reject = reject.clone();
            Closure::once_into_js(move |evt: MessageEvent| {
                let data = evt.data();
                if let Some(n) = data.as_f64() {
                    let _ = resolve.call1(&JsValue::NULL, &JsValue::from_f64(n));
                } else {
                    let err = JsValue::from_str("worker posted a non-number reply");
                    let _ = reject.call1(&JsValue::NULL, &err);
                }
            })
        };
        worker.set_onmessage(Some(on_message.unchecked_ref()));

        let on_error = {
            let reject = reject.clone();
            Closure::once_into_js(move |evt: JsValue| {
                let _ = reject.call1(&JsValue::NULL, &evt);
            })
        };
        worker.set_onerror(Some(on_error.unchecked_ref()));

        let msg = js_sys::Object::new();
        Reflect::set(&msg, &"kind".into(), &JsValue::from_str(kind))
            .map_err(|e| js_err("set kind on message", e))?;
        Reflect::set(&msg, &"module".into(), &module)
            .map_err(|e| js_err("set module on message", e))?;
        Reflect::set(&msg, &"memory".into(), &memory)
            .map_err(|e| js_err("set memory on message", e))?;
        Reflect::set(&msg, &"ptr".into(), &JsValue::from_f64(task_ptr as f64))
            .map_err(|e| js_err("set ptr on message", e))?;
        Reflect::set(&msg, &"glue_url".into(), &JsValue::from_str(&glue_url))
            .map_err(|e| js_err("set glue_url on message", e))?;

        worker
            .post_message(&msg)
            .map_err(|e| js_err("postMessage to worker failed", e))?;

        let resolved = JsFuture::from(promise)
            .await
            .map_err(|e| js_err("worker future rejected", e))?;
        worker.terminate();

        Ok(resolved.as_f64().context("worker reply was not a Number")? as usize)
    }

    /// Submit a sync closure to a worker and await its result. `R` is inferred
    /// from the closure's return type — no turbofish needed at the call site.
    pub async fn run_blocking<F, R>(f: F, crate_name: &str) -> Result<R>
    where
        F: FnOnce() -> R + 'static,
        R: 'static,
    {
        let task = WorkerTask::new(move || box_result(f()));
        let result_ptr = run_inner(task.into_ptr(), "sync", crate_name).await?;
        // SAFETY: result_ptr came from box_result::<R> on the worker.
        Ok(unsafe { unbox_result::<R>(result_ptr as *mut ()) })
    }

    /// Submit an async closure to a worker and await its result.
    pub async fn run_async<F, Fut, R>(f: F, crate_name: &str) -> Result<R>
    where
        F: FnOnce() -> Fut + 'static,
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        let task = AsyncWorkerTask::new(move || async move { box_result(f().await) });
        let result_ptr = run_inner(task.into_ptr(), "async", crate_name).await?;
        // SAFETY: result_ptr came from box_result::<R> on the worker.
        Ok(unsafe { unbox_result::<R>(result_ptr as *mut ()) })
    }

    // ── Worker-side future driver ────────────────────────────────────────────
    //
    // Drives a future to completion on the worker's own event loop, returning
    // a Promise that resolves with the future's *mut () output (as an f64).
    //
    // Why not wasm_bindgen_futures? With atomics on (needed for shared memory)
    // it uses a multithread executor that coordinates wakeups via
    // Atomics.waitAsync and a helper worker — infrastructure we don't run.
    // Here we reschedule polls through queueMicrotask; JsFuture's own
    // Promise.then wakeups also land on this event loop, so we make progress
    // with no cross-thread machinery.

    use std::cell::RefCell;
    use std::rc::Rc;
    use std::task::{Poll, RawWaker, RawWakerVTable, Waker};

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
                let _ = self.resolve.call1(
                    &JsValue::NULL,
                    &JsValue::from_f64(result_ptr as usize as f64),
                );
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
            let cb = wasm_bindgen::closure::Closure::once_into_js(move || this.poll());
            queue_microtask(&cb);
        }

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
}

/// Worker thread entry point for sync tasks. Returns the result pointer.
#[wasm_bindgen]
pub fn __worxide_worker_entry(task_ptr: usize) -> usize {
    // SAFETY: pointer came from WorkerTask::into_ptr on the main thread.
    let task = unsafe { __private::WorkerTask::from_ptr(task_ptr) };
    task.run() as usize
}

/// Worker thread entry point for async tasks. Returns a Promise that
/// resolves with the result pointer once the task's future completes.
///
/// We deliberately avoid `wasm_bindgen_futures::future_to_promise` /
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
pub fn __worxide_worker_entry_async(task_ptr: usize) -> js_sys::Promise {
    // SAFETY: pointer came from AsyncWorkerTask::into_ptr on the main thread.
    let task = unsafe { __private::AsyncWorkerTask::from_ptr(task_ptr) };
    __private::drive_to_promise(task.run())
}

/// Spawn a synchronous function on a worker.
///
/// ```ignore
/// fn crunch(n: u32) -> u64 { (n as u64) * 2 }
/// let result = worxide::spawn_blocking!(crunch, 42).await?;
/// ```
#[macro_export]
macro_rules! spawn_blocking {
    ($func:path $(, $arg:expr)* $(,)?) => {{
        $crate::__private::run_blocking(
            move || $func($($arg),*),
            env!("CARGO_PKG_NAME"),
        )
    }};
}

/// Spawn an asynchronous function on a worker.
///
/// ```ignore
/// async fn crunch(n: u32) -> u64 { (n as u64) * 2 }
/// let result = worxide::spawn!(crunch, 42).await?;
/// ```
#[macro_export]
macro_rules! spawn {
    ($func:path $(, $arg:expr)* $(,)?) => {{
        $crate::__private::run_async(
            move || async move { $func($($arg),*).await },
            env!("CARGO_PKG_NAME"),
        )
    }};
}
