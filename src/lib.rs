//! worxide — spawn Rust functions on Web Workers via shared memory.

#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub mod private;

/// A persistent Web Worker attached to this thread's shared memory.
///
/// Where [`spawn!`] / [`spawn_blocking!`] create a worker, run one task, and
/// terminate it, a `Worker` is constructed once, attaches to this thread's
/// shared memory, and stays alive so subsequent calls reuse the same instance.
/// Use it when the worker must hold state across calls or receive a transferred
/// object (e.g. an `OffscreenCanvas`); reach for the macros for one-off jobs.
///
/// Construct it with [`Worker::new`] (which resolves the wasm glue via the
/// consumer-set `globalThis.app_js_path`) or [`Worker::with_glue`] (an explicit
/// path/URL). There is no `spawn_persistent!` macro: once you hold a handle,
/// run work with the plain methods [`Worker::run_blocking`] (sync, CPU-bound)
/// and [`Worker::run`] (async). Arguments and results cross by pointer through
/// shared memory — no serialization, no copy.
///
/// Dropping the handle terminates the worker.
///
/// ```ignore
/// let w = worxide::Worker::new().await?;            // glue via globalThis.app_js_path
/// let n = w.run_blocking(move || crunch(data)).await?;
/// let raw: &web_sys::Worker = w.raw();              // for transfers + side-channels
/// w.terminate();
/// ```
pub use private::Worker;

/// Returns `true` if the current thread is a Web Worker, `false` if it is the
/// main (window) thread.
///
/// This is useful for guarding main-thread-only work (DOM access, UI setup),
/// since the wasm module is instantiated on every worker too. Prefer this over
/// ad-hoc checks like `web_sys::window().is_none()`: it positively identifies a
/// worker by testing the global scope against `WorkerGlobalScope`, rather than
/// inferring "worker" from the absence of a window.
///
/// ```ignore
/// #[wasm_bindgen]
/// pub fn run() {
///     if worxide::is_worker() { return; } // never build UI on a worker
///     // ... main-thread setup ...
/// }
/// ```
pub fn is_worker() -> bool {
    use wasm_bindgen::{JsCast, JsValue};
    let global: JsValue = js_sys::global().into();
    // `WorkerGlobalScope` exists only inside workers. If the global is an
    // instance of it, we're on a worker thread.
    global.is_instance_of::<web_sys::WorkerGlobalScope>()
}

/// Spawn a synchronous function on a Web Worker and await its result.
///
/// The worker runs `func(args...)` on its own thread; the call site gets back
/// a future of `anyhow::Result<R>`, with `R` inferred from `func`'s return
/// type. Use this for CPU-bound work that would otherwise block the caller.
///
/// ```ignore
/// fn crunch(n: u32) -> u64 { (n as u64) * 2 }
/// let result = worxide::spawn_blocking!(crunch, 42).await?;
/// ```
#[macro_export]
macro_rules! spawn_blocking {
    ($func:path $(, $arg:expr)* $(,)?) => {{
        $crate::private::run_blocking(
            move || $func($($arg),*),
            env!("CARGO_PKG_NAME"),
        )
    }};
}

/// Spawn an asynchronous function on a Web Worker and await its result.
///
/// Like [`spawn_blocking!`], but for `async fn` / future-returning functions.
/// The worker drives the future to completion on its own event loop.
///
/// ```ignore
/// async fn crunch(n: u32) -> u64 { (n as u64) * 2 }
/// let result = worxide::spawn!(crunch, 42).await?;
/// ```
#[macro_export]
macro_rules! spawn {
    ($func:path $(, $arg:expr)* $(,)?) => {{
        $crate::private::run_async(
            move || async move { $func($($arg),*).await },
            env!("CARGO_PKG_NAME"),
        )
    }};
}
