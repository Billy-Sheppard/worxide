//! worxide — spawn Rust functions on Web Workers via shared memory.

#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub mod private;

pub use private::Worker;

/// Detects if the current thread is a Web Worker
///
/// This is useful for guarding main-thread-only work (DOM access, UI setup, etc),
///  since the wasm module is instantiated on every worker too.
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
    // `WorkerGlobalScope` exists only inside workers. If the global is an instance of it, we're on a worker thread.
    global.is_instance_of::<web_sys::WorkerGlobalScope>()
}

/// Spawn a synchronous function on a Web Worker and await its result.
///
/// The worker runs `func(args...)` on its own thread; the call site gets back a future of `anyhow::Result<R>`,
/// with `R` inferred from `func`'s return type. Use this for CPU-bound work that would otherwise block the caller.
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
/// The worker drives the future to completion on its own event loop (a JS Promise).
///
/// ```ignore
/// async fn get_image(url: &str) -> anyhow::Result<String> { reqwest::get(url).text().await? }
/// let result = worxide::spawn!(get_image, "https://example.com").await; // this returns anyhow::Result<anyhow::Result<String>>
/// let result = worxide::spawn!(get_image, "https://example.com").await?; // this returns anyhow::Result<String>
/// let result = worxide::spawn!(get_image, "https://example.com").await.flatten()?; // this returns String
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
