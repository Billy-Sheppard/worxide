use {
    crate::Worker,
    anyhow::{Result, anyhow},
    std::{
        future::Future,
        io,
        rc::Rc,
        task::{Context, Poll, Waker},
    },
    wasm_bindgen::{JsCast, JsValue},
};

/// Configures dedicated Rayon workers.
///
/// Workers join the caller's existing shared memory. Memory limits therefore
/// belong to that memory, not to this builder.
#[derive(Clone, Debug)]
#[must_use = "call .spawn().await to construct the pool"]
#[derive(Default)]
pub struct RayonPoolBuilder {
    threads: Option<usize>,
    glue_url: Option<String>,
}

impl RayonPoolBuilder {
    pub fn new() -> Self { Self::default() }

    /// Override the number of dedicated Rayon workers.
    ///
    /// When omitted, uses the browser-reported logical processor count,
    /// falling back to one.
    ///
    /// The coordinator is not included in this count.
    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    fn default_threads() -> usize {
        let Ok(scope) = js_sys::global().dyn_into::<web_sys::WorkerGlobalScope>() else { return 1 };
        let reported = scope.navigator().hardware_concurrency();

        if reported.is_finite() && reported >= 1.0 { reported.floor() as usize } else { 1 }
    }

    /// Override application glue discovery.
    ///
    /// Normally unnecessary inside a worxide worker because bootstrap seeds
    /// the resolved glue URL for nested worker creation.
    pub fn glue_url(mut self, url: impl Into<String>) -> Self {
        self.glue_url = Some(url.into());
        self
    }

    /// Boot dedicated workers and construct a local Rayon pool.
    ///
    /// Must be called from a Web Worker, not the browser main thread.
    pub async fn spawn(self) -> Result<rayon::ThreadPool> {
        if !crate::is_worker() {
            return Err(anyhow!("worxide: create RayonPool inside a coordinator Web Worker, not the browser main thread"));
        }

        let threads = self.threads.unwrap_or_else(Self::default_threads);

        if threads == 0 {
            return Err(anyhow!("worxide: RayonPool requires at least one thread"));
        }

        let mut workers = Vec::new();
        workers.try_reserve_exact(threads)?;

        for _ in 0..threads {
            let worker = match &self.glue_url {
                Some(url) => Worker::with_glue(url).await?,
                None => Worker::new().await?,
            };

            workers.push(Rc::new(worker));
        }

        let mut available = workers.into_iter();

        let inner = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .spawn_handler(move |thread| {
                let worker = available
                    .next()
                    .ok_or_else(|| io::Error::other("worxide: Rayon requested an unexpected extra worker"))?;

                start_thread(worker, move || thread.run())
            })
            .build()
            .map_err(|error| anyhow!("worxide: unable to construct Rayon pool: {error}"))?;

        Ok(inner)
    }
}

/// Poll once so the shared task is posted before returning from Rayon's
/// synchronous spawn callback.
///
/// Merely calling spawn_local here would defer posting until the coordinator
/// returns to its event loop. A subsequent blocking pool operation could then
/// prevent the thread tasks from ever being posted.
fn start_thread<F>(worker: Rc<Worker>, operation: F) -> io::Result<()>
where
    F: FnOnce() + Send + 'static,
{
    let mut completion = Box::pin(async move { worker.run_blocking(operation).await });
    let mut context = Context::from_waker(Waker::noop());

    match completion.as_mut().poll(&mut context) {
        Poll::Ready(Err(error)) => Err(io::Error::other(format!("worxide: unable to start Rayon worker: {error:#}"))),
        Poll::Ready(Ok(())) => Err(io::Error::other("worxide: Rayon worker exited during pool construction")),
        Poll::Pending => {
            wasm_bindgen_futures::spawn_local(async move {
                if let Err(error) = completion.await {
                    report_error(&format!("worxide: dedicated Rayon worker failed: {error:#}"));
                }
            });
            Ok(())
        }
    }
}

#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
    export function worxide_rayon_error(message) {
        console.error(message);
    }
"#)]
extern "C" {
    fn worxide_rayon_error(message: &JsValue);
}

fn report_error(message: &str) { worxide_rayon_error(&JsValue::from_str(message)); }
