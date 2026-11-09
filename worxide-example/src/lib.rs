//! worxide-example
mod html_macros;
use {
    dominator::events,
    futures_signals::{
        map_ref,
        signal::{Mutable, SignalExt},
        signal_vec::{MutableVec, SignalVecExt},
    },
    html_macros::*,
    std::{collections::HashSet, future::Future, ops::*, pin::Pin, sync::Arc},
    wasm_bindgen::{self, JsCast, prelude::*},
    worxide::{rayon, rayon::iter::*},
};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn collatz(limit: u64) -> u64 {
    let mut total = 0u64;
    for start in 1..=limit {
        let mut n = start;
        let mut steps = 0u64;
        while n != 1 {
            n = if n & 1 == 0 { n / 2 } else { 3 * n + 1 };
            steps += 1;
        }
        total = total.wrapping_add(steps);
    }
    total
}

fn pi_leibniz(terms: u64) -> u64 {
    let mut sum = 0.0f64;
    let mut sign = 1.0f64;
    for k in 0..terms {
        sum += sign / (2 * k + 1) as f64;
        sign = -sign;
    }
    ((sum * 4.0) * 1e9) as u64
}

fn prime_count(limit: u64) -> u64 {
    let mut count = 0u64;
    for n in 2..limit {
        let mut is_prime = true;
        let mut d = 2u64;
        while d * d <= n {
            if n % d == 0 {
                is_prime = false;
                break;
            }
            d += 1;
        }
        if is_prime {
            count += 1;
        }
    }
    count
}

async fn deserialize_json(url: &str) -> anyhow::Result<(usize, serde_json::Value)> {
    let body = reqwest::get(url).await?.error_for_status()?.bytes().await?;
    let size = body.len();
    let res = serde_json::from_slice(&body)?;

    Ok((size, res))
}

type RunFuture = Pin<Box<dyn Future<Output = Result<String, String>>>>;

/// Where a job runs: on a worxide worker, or inline on the main thread (which
/// blocks the UI — useful for demonstrating the difference).
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Worker,
    MainThread,
}

struct PoolTask {
    title: String,
    run: Box<dyn FnOnce(Mode) -> RunFuture>,
}

fn sync_task(func: &str, f: fn(u64) -> u64, param: u64) -> PoolTask {
    PoolTask {
        title: format!("{func}({param})"),
        run: Box::new(move |mode| {
            Box::pin(async move {
                match mode {
                    Mode::Worker => {
                        worxide::spawn_blocking!(f, param).await.map(|r| format!("Res: {r}")).map_err(|e| e.to_string())
                    }
                    // Runs inline on the main thread — blocks the UI until done.
                    Mode::MainThread => Ok(format!("Res: {}", f(param))),
                }
            })
        }),
    }
}

fn build_pool() -> Vec<PoolTask> {
    let mut pool = vec![
        sync_task("fibonacci", fibonacci, 43),
        sync_task("collatz", collatz, 8_000_000),
        sync_task("pi_leibniz", pi_leibniz, 800_000_000),
        sync_task("prime_count", prime_count, 10_000_000),
        sync_task("fibonacci", fibonacci, 44),
        sync_task("collatz", collatz, 10_000_000),
        sync_task("pi_leibniz", pi_leibniz, 900_000_000),
        sync_task("prime_count", prime_count, 20_000_000),
        sync_task("fibonacci", fibonacci, 45),
        sync_task("collatz", collatz, 20_000_000),
        sync_task("pi_leibniz", pi_leibniz, 1_000_000_000),
        sync_task("prime_count", prime_count, 30_000_000),
    ];

    pool.push(PoolTask {
        title: "deserialize_json".into(),
        run: Box::new(move |mode| {
            Box::pin(async move {
                let url = "https://raw.githubusercontent.com/json-iterator/test-data/refs/heads/master/large-file.json";
                let res = match mode {
                    Mode::Worker => worxide::spawn!(deserialize_json, url)
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|v| v.map_err(|e| e.to_string())),
                    // Runs inline on the main thread.
                    Mode::MainThread => deserialize_json(url).await.map_err(|e| e.to_string()),
                };
                res.map(|(size, res)| {
                    let n = res.as_array().map(|a| a.len()).unwrap_or(0);
                    format!("Parsed {n} entries ({} bytes)", size)
                })
            })
        }),
    });

    pool
}

#[derive(Clone)]
struct Worker {
    name: Arc<str>,
    title: Arc<str>,
    result: Mutable<Option<Result<String, String>>>,
}

fn spawn_job(
    names: &Mutable<HashSet<&'static str>>,
    pool: &Mutable<Vec<PoolTask>>,
    workers: &MutableVec<Arc<Worker>>,
    mode: Mode,
) -> bool {
    let key = {
        let mut g = names.lock_mut();
        let Some(k) = g.iter().next().cloned() else {
            return false;
        };
        g.take(&k).unwrap()
    };

    let task = {
        let mut g = pool.lock_mut();
        if g.is_empty() {
            return false;
        }
        let idx = ((js_sys::Math::random() * g.len() as f64) as usize).min(g.len() - 1);
        g.remove(idx)
    };

    let worker = Arc::new(Worker { name: key.into(), title: Arc::from(task.title.as_str()), result: Mutable::new(None) });
    workers.lock_mut().push_cloned(worker.clone());

    wasm_bindgen_futures::spawn_local(async move {
        let r = (task.run)(mode).await;
        worker.result.set(Some(r));
    });

    true
}

fn worker_names() -> HashSet<&'static str> {
    HashSet::from([
        "Abigail",
        "Alexander",
        "Alexis",
        "Alyssa",
        "Andrew",
        "Anna",
        "Anthony",
        "Ashley",
        "Ava",
        "Benjamin",
        "Brandon",
        "Brianna",
        "Chloe",
        "Christian",
        "Christopher",
        "Daniel",
        "David",
        "Dylan",
        "Elizabeth",
        "Emily",
        "Emma",
        "Ethan",
        "Grace",
        "Hannah",
        "Isabella",
        "Jacob",
        "James",
        "Jessica",
        "John",
        "Jonathan",
        "Joseph",
        "Joshua",
        "Kayla",
        "Lauren",
        "Madison",
        "Matthew",
        "Michael",
        "Natalie",
        "Nicholas",
        "Noah",
        "Olivia",
        "Ryan",
        "Samantha",
        "Samuel",
        "Sarah",
        "Sophia",
        "Taylor",
        "Tyler",
        "Victoria",
        "William",
    ])
}
#[wasm_bindgen]
pub async fn run_app() -> Result<(), JsValue> {
    if worxide::is_worker() {
        return Ok(());
    }

    let pool = Mutable::new(build_pool());
    let workers: MutableVec<Arc<Worker>> = MutableVec::new();

    let names = Mutable::new(worker_names());
    let mode = Mutable::new(Mode::Worker);

    let header = {
        let fps = Mutable::new(60.0);
        let pressure: Mutable<Option<String>> = Mutable::new(None);
        let has_pressure = pressure_supported();

        start_fps_meter(fps.clone());
        if has_pressure {
            start_pressure_observer(pressure.clone());
        }

        html_div()
            .class(["columns"])
            .child(
                html_div()
                    .class(["column", "is-narrow"])
                    .child(html_h1().class("title").text("worxide").into_dom())
                    .into_dom(),
            )
            .child(
                html_div()
                    .class(["column", "is-narrow", "buttons", "has-addons"])
                    .child(
                        html_button()
                            .class(["button", "fading-text"])
                            .style("height", "100%")
                            .text("main thread stays active")
                            .into_dom(),
                    )
                    .child_signal({
                        let workers = workers.clone();
                        workers
                            .signal_vec_cloned()
                            .map_signal(|worker| worker.result.signal_cloned())
                            .to_signal_cloned()
                            .map(|vec: Vec<Option<Result<String, String>>>| {
                                vec.iter().any(Option::is_none).then_some(
                                    html_button().class(["button", "is-loading"]).style("height", "100%").into_dom(),
                                )
                            })
                    })
                    .child(
                        html_button()
                            .class("button")
                            .style("height", "100%")
                            .text_signal(fps.signal().map(|f| format!("Main thread FPS: {:.2}", f)))
                            .into_dom(),
                    )
                    .apply_if(has_pressure, |d| {
                        d.child(
                            html_button()
                                .class("button")
                                .style("height", "100%")
                                .text_signal(
                                    pressure
                                        .signal_cloned()
                                        .map(|p| format!("CPU Pressure: {}", p.unwrap_or_else(|| "—".into()))),
                                )
                                .into_dom(),
                        )
                    })
                    .into_dom(),
            )
            .into_dom()
    };

    let dom = html_div()
        .class(["section"])
        .child(header)
        .child(html_hr().into_dom())
        .child(html_h2().class("title").text("Off-thread Examples").into_dom())
        .child(
            html_div()
                .class(["columns"])
                .child(
                    html_div()
                        .class(["column", "is-narrow", "buttons", "has-addons"])
                        .child(
                            html_button()
                                .class("button")
                                .class_signal("is-info", mode.signal().map(|m| m == Mode::Worker))
                                .text("Worker Threads")
                                .event({
                                    let mode = mode.clone();
                                    move |_: events::Click| mode.set(Mode::Worker)
                                })
                                .into_dom(),
                        )
                        .child(
                            html_button()
                                .class("button")
                                .class_signal("is-warning", mode.signal().map(|m| m == Mode::MainThread))
                                .text("Main Thread")
                                .event({
                                    let mode = mode.clone();
                                    move |_: events::Click| mode.set(Mode::MainThread)
                                })
                                .into_dom(),
                        )
                        .into_dom(),
                )
                .child(
                    html_div()
                        .class(["column", "is-narrow", "buttons", "has-addons"])
                        .child_signal({
                            let workers = workers.clone();
                            let pool = pool.clone();
                            let names = names.clone();
                            let mode = mode.clone();
                            pool.signal_ref(|p| p.is_empty()).map(move |empty| {
                                empty.not().then_some(
                                    html_button()
                                        .class(["button", "is-success"])
                                        .text("Spawn Job")
                                        .event({
                                            let workers = workers.clone();
                                            let pool = pool.clone();
                                            let names = names.clone();
                                            let mode = mode.clone();
                                            move |_: events::Click| {
                                                spawn_job(&names, &pool, &workers, mode.get());
                                            }
                                        })
                                        .into_dom(),
                                )
                            })
                        })
                        .child_signal({
                            let workers = workers.clone();
                            let pool = pool.clone();
                            let names = names.clone();
                            let mode = mode.clone();
                            pool.signal_ref(|p| p.is_empty()).map(move |empty| {
                                empty.not().then_some(
                                    html_button()
                                        .class(["button", "is-link"])
                                        .text("Spawn All Jobs")
                                        .event({
                                            let workers = workers.clone();
                                            let pool = pool.clone();
                                            let names = names.clone();
                                            let mode = mode.clone();
                                            move |_: events::Click| {
                                                while spawn_job(&names, &pool, &workers, mode.get()) {}
                                            }
                                        })
                                        .into_dom(),
                                )
                            })
                        })
                        .child_signal({
                            let workers = workers.clone();
                            let pool = pool.clone();
                            let names = names.clone();
                            map_ref! {
                                let pool_empty = pool.signal_ref(|p| p.is_empty()),
                                let all_done = workers
                                    .signal_vec_cloned()
                                    .map_signal(|worker| worker.result.signal_cloned())
                                    .to_signal_cloned()
                                    .map(|vec: Vec<Option<Result<String, String>>>| {
                                        vec.iter().all(Option::is_some)
                                    }),
                                let any_workers = workers.signal_vec_cloned()
                                    .to_signal_cloned()
                                    .map(|v| v.is_empty().not())
                                => (*pool_empty && *all_done && *any_workers).then_some(
                                    html_button()
                                        .class(["button", "is-danger"])
                                        .text("Reset Jobs")
                                        .event({
                                            let workers = workers.clone();
                                            let pool = pool.clone();
                                            let names = names.clone();
                                            move |_: events::Click| {
                                                workers.lock_mut().clear();
                                                *pool.lock_mut() = build_pool();
                                                *names.lock_mut() = worker_names();
                                            }
                                        })
                                        .into_dom(),
                                )
                            }
                        })
                        .into_dom(),
                )
                .into_dom(),
        )
        .child(
            html_div()
                .class(["fixed-grid", "has-4-cols"])
                .child(
                    html_div()
                        .class(["grid", "is-col-min-3"])
                        .children_signal_vec(workers.signal_vec_cloned().map(|worker| {
                            html_div()
                                .class(["cell"])
                                .child(
                                    html_div()
                                        .style("margin", "1rem")
                                        .style("padding", "1rem")
                                        .style("height", "min-content")
                                        .style("width", "100%")
                                        .style("border", "white dashed 1px")
                                        .style("border-radius", "1rem")
                                        .child(html_h2().class("subtitle").text(worker.name.as_ref()).into_dom())
                                        .child(html_h3().text(worker.title.as_ref()).into_dom())
                                        .child_signal(worker.result.signal_cloned().map(|res| {
                                            Some(
                                                html_button()
                                                    .class("button")
                                                    .style("width", "100%")
                                                    .style("height", "40px")
                                                    .apply_if(res.is_none(), |b| b.class("is-loading"))
                                                    .apply_if(res.is_some(), |b| {
                                                        b.text(match res.as_ref().unwrap() {
                                                            Ok(s) => s.as_str(),
                                                            Err(e) => e.as_str(),
                                                        })
                                                    })
                                                    .into_dom(),
                                            )
                                        }))
                                        .into_dom(),
                                )
                                .into_dom()
                        }))
                        .into_dom(),
                )
                .into_dom(),
        )
        .child(html_hr().into_dom())
        .child(html_h2().class("title").text("Other Uses").into_dom())
        .child(other_examples())
        .into_dom();

    dominator::append_dom(&dominator::body(), dom);
    Ok(())
}

fn pressure_supported() -> bool {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("PressureObserver"))
        .map(|v| !v.is_undefined())
        .unwrap_or(false)
}

fn start_fps_meter(fps: Mutable<f64>) {
    let last = Mutable::new(now_ms());
    let callback = Mutable::<Option<Closure<dyn FnMut()>>>::new(None);
    let cb = callback.clone();
    callback.set(Some(Closure::wrap(Box::new(move || {
        let t = now_ms();
        let dt = t - last.get();
        last.set(t);
        if dt.gt(&0.0) {
            let inst = 1000.0.div(dt);
            let prev = fps.get();
            fps.set(prev.mul(0.9).add(inst.mul(0.1)));
        }
        if let Some(w) = web_sys::window()
            && let Some(c) = cb.lock_ref().as_ref()
        {
            w.request_animation_frame(c.as_ref().unchecked_ref()).unwrap();
        }
    }) as Box<dyn FnMut()>)));

    if let Some(w) = web_sys::window()
        && let Some(c) = callback.lock_ref().as_ref()
    {
        w.request_animation_frame(c.as_ref().unchecked_ref()).unwrap();
    }
}

fn start_pressure_observer(pressure: Mutable<Option<String>>) {
    let constructor = if let Ok(v) = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("PressureObserver"))
        && v.is_null_or_undefined().not()
    {
        v.unchecked_into::<js_sys::Function>()
    } else {
        return;
    };

    let cb = Closure::wrap(Box::new(move |records: JsValue| {
        if let Ok(arr) = records.dyn_into::<js_sys::Array>() {
            let len = arr.length();
            if len.gt(&0)
                && let Ok(s) = js_sys::Reflect::get(&arr.get(len.sub(1)), &JsValue::from_str("state"))
                && let Some(s) = s.as_string()
            {
                pressure.set(Some(s));
            }
        }
    }) as Box<dyn FnMut(JsValue)>);

    let observer = if let Ok(o) = js_sys::Reflect::construct(&constructor, &js_sys::Array::of1(cb.as_ref().unchecked_ref()))
    {
        o
    } else {
        cb.forget();
        return;
    };
    cb.forget();

    if let Ok(observe) = js_sys::Reflect::get(&observer, &JsValue::from_str("observe"))
        && let Ok(observe_fn) = observe.dyn_into::<js_sys::Function>()
    {
        let opts = js_sys::Object::new();
        js_sys::Reflect::set(&opts, &JsValue::from_str("sampleInterval"), &JsValue::from_f64(500.0)).unwrap();
        observe_fn.call2(&observer, &JsValue::from_str("cpu"), &opts).unwrap();
    }
}

fn now_ms() -> f64 { web_sys::window().and_then(|w| w.performance()).map(|p| p.now()).unwrap_or(0.0) }

thread_local! {
    // Retain the listener for the lifetime of the worker.
    #[allow(clippy::type_complexity)]static EXAMPLE_LISTENER:
        std::cell::RefCell<Option<Closure<dyn FnMut(web_sys::MessageEvent)>>> =
        const { std::cell::RefCell::new(None) };
}

thread_local! {
    static EXAMPLE_RAYON_POOL:
        std::cell::RefCell<Option<rayon::ThreadPool>> =
        const { std::cell::RefCell::new(None) };

    // Keep the coordinator alive while its pool exists.
    static EXAMPLE_RAYON_COORDINATOR:
        std::cell::RefCell<Option<std::rc::Rc<worxide::Worker>>> =
        const { std::cell::RefCell::new(None) };
}

/// Runs automatically when the module is instantiated in a worker.
///
/// This listener belongs to the example application, not worxide.
#[wasm_bindgen(start)]
pub fn initialize_worker_example() {
    if !worxide::is_worker() {
        return;
    }

    let scope: web_sys::DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();

    let reply_scope = scope.clone();

    let listener = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
        let Some(message) = event.data().as_string() else {
            return;
        };

        let Some(index) = message.strip_prefix("worxide-example:calculate:") else {
            return;
        };

        let result = (|| -> anyhow::Result<String> {
            let index = index.parse::<usize>()?;

            let &(name, function, input, calls) =
                COMPARISONS.get(index).ok_or_else(|| anyhow::anyhow!("Unknown comparison"))?;

            let started = js_sys::Date::now();
            let checksum = run_batch(function, input, calls);
            let elapsed_ms = js_sys::Date::now() - started;

            Ok(format!("Completed\nWorkload: {calls} × {name}({input})\nCompute: {elapsed_ms:.2} ms\nChecksum: {checksum}"))
        })();

        let reply = match result {
            Ok(message) => format!("worxide-example:ok:{message}"),
            Err(error) => format!("worxide-example:error:{error:#}"),
        };

        let _ = reply_scope.post_message(&reply.into());
    });

    scope
        .add_event_listener_with_callback("message", listener.as_ref().unchecked_ref())
        .expect("Unable to install example listener");

    EXAMPLE_LISTENER.with(|slot| {
        *slot.borrow_mut() = Some(listener);
    });
}

fn example_glue_path() -> String {
    // Same Cargo filename convention used by the existing spawn macros.
    // Change this if your build uses a custom wasm-bindgen output name.
    format!("{}.js", env!("CARGO_PKG_NAME").replace('-', "_"))
}

async fn persistent_worker_example(index: usize) -> anyhow::Result<String> {
    let &(name, function, input, calls) = COMPARISONS.get(index).ok_or_else(|| anyhow::anyhow!("Unknown comparison"))?;

    let started = js_sys::Date::now();

    let worker = worxide::WorkerBuilder::default().glue_url(example_glue_path()).spawn().await?;

    let startup_ms = js_sys::Date::now() - started;

    let (first_checksum, first_ms) = worker
        .run_blocking(move || {
            let started = js_sys::Date::now();
            let checksum = run_batch(function, input, calls);

            (checksum, js_sys::Date::now() - started)
        })
        .await?;

    let (second_checksum, second_ms) = worker
        .run_blocking(move || {
            let started = js_sys::Date::now();
            let checksum = run_batch(function, input, calls);

            (checksum, js_sys::Date::now() - started)
        })
        .await?;

    anyhow::ensure!(first_checksum == second_checksum, "Repeated calls produced different checksums");

    Ok(format!(
        "PASS: repeated checksums match\nWorkload per batch: {calls} × {name}({input})\nWorker startup: {startup_ms:.2} \
         ms\nFirst batch: {first_ms:.2} ms\nSecond batch: {second_ms:.2} ms\nChecksum per batch: {first_checksum}\nBoth \
         batches used the same worker.",
    ))
}

async fn isolated_worker_example(index: usize) -> anyhow::Result<String> {
    anyhow::ensure!(index < COMPARISONS.len(), "Unknown comparison",);

    let started = js_sys::Date::now();

    let worker = worxide::WorkerBuilder::default().glue_url(example_glue_path()).isolated().max_256mib().spawn().await?;

    let startup_ms = js_sys::Date::now() - started;

    let mut functions = None;

    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        functions = Some((resolve, reject));
    });

    let (resolve, reject) = functions.expect("Promise executor must run synchronously");

    let message_reject = reject.clone();

    let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
        let Some(reply) = event.data().as_string() else {
            return;
        };

        if let Some(result) = reply.strip_prefix("worxide-example:ok:") {
            resolve.call1(&JsValue::NULL, &result.into()).ok();
        } else if let Some(error) = reply.strip_prefix("worxide-example:error:") {
            message_reject.call1(&JsValue::NULL, &error.into()).ok();
        }
    });

    let on_error = Closure::<dyn FnMut(JsValue)>::new(move |error: JsValue| {
        reject.call1(&JsValue::NULL, &error).ok();
    });

    worker.worker_handle().set_onmessage(Some(on_message.as_ref().unchecked_ref()));

    worker.worker_handle().set_onerror(Some(on_error.as_ref().unchecked_ref()));

    let started = js_sys::Date::now();

    let result = async {
        worker.worker_handle().post_message(&format!("worxide-example:calculate:{index}").into())?;

        wasm_bindgen_futures::JsFuture::from(promise).await
    }
    .await;

    let request_ms = js_sys::Date::now() - started;

    worker.worker_handle().set_onmessage(None);
    worker.worker_handle().set_onerror(None);
    worker.terminate();

    let reply = result.map_err(|error| anyhow::anyhow!("{}", error.as_string().unwrap_or_else(|| format!("{error:?}")),))?;

    Ok(format!(
        "{}\nWorker startup: {startup_ms:.2} ms\nRequest round trip: {request_ms:.2} ms\nMemory ceiling: 256 MiB\nWorker \
         terminated.",
        reply.as_string().unwrap_or_default(),
    ))
}

async fn rayon_worker_example(index: usize) -> anyhow::Result<String> {
    let &(name, function, input, calls) = COMPARISONS.get(index).ok_or_else(|| anyhow::anyhow!("Unknown comparison"))?;

    let existing = EXAMPLE_RAYON_COORDINATOR.with(|slot| slot.borrow().clone());

    let coordinator = match existing {
        Some(worker) => worker,
        None => {
            let worker = std::rc::Rc::new(worxide::WorkerBuilder::default().glue_url(example_glue_path()).spawn().await?);

            EXAMPLE_RAYON_COORDINATOR.with(|slot| {
                *slot.borrow_mut() = Some(worker.clone());
            });

            worker
        }
    };

    coordinator
        .run(|| async {
            if EXAMPLE_RAYON_POOL.with(|slot| slot.borrow().is_some()) {
                return Ok::<(), anyhow::Error>(());
            }

            let pool = worxide::RayonPoolBuilder::new().spawn().await?;

            EXAMPLE_RAYON_POOL.with(|slot| {
                *slot.borrow_mut() = Some(pool);
            });

            Ok(())
        })
        .await??;

    let (sequential, parallel, sequential_ms, parallel_ms, threads) = coordinator
        .run_blocking(move || {
            EXAMPLE_RAYON_POOL.with(|slot| {
                let slot = slot.borrow();

                let pool = slot.as_ref().ok_or_else(|| anyhow::anyhow!("Rayon pool is not initialized"))?;

                let started = js_sys::Date::now();

                let sequential = run_batch(function, input, calls);

                let sequential_ms = js_sys::Date::now() - started;
                let started = js_sys::Date::now();

                let parallel = pool.install(|| {
                    (0..calls)
                        .into_par_iter()
                        .map(|_| {
                            let result = function(std::hint::black_box(input));

                            std::hint::black_box(result)
                        })
                        .reduce(|| 0u64, |left, right| left.wrapping_add(right))
                });

                let parallel_ms = js_sys::Date::now() - started;

                Ok::<_, anyhow::Error>((sequential, parallel, sequential_ms, parallel_ms, pool.current_num_threads()))
            })
        })
        .await??;

    anyhow::ensure!(sequential == parallel, "Sequential and parallel checksums differ",);

    let ratio = if parallel_ms > 0.0 { format!("{:.2}×", sequential_ms / parallel_ms) } else { "Not measurable".into() };

    Ok(format!(
        "PASS: checksums match\nWorkload: {calls} × {name}({input})\nRayon threads: {threads}\nSequential: \
         {sequential_ms:.2} ms\nParallel: {parallel_ms:.2} ms\nSequential/parallel time: {ratio}\nChecksum: \
         {parallel}\nCoordinator and pool retained.",
    ))
}

fn other_examples() -> dominator::Dom {
    let running = Mutable::new(false);

    #[allow(clippy::type_complexity)]
    let groups: [(&str, fn(usize) -> RunFuture); 3] = [
        ("Persistent shared workers", |index| {
            Box::pin(async move { persistent_worker_example(index).await.map_err(|error| format!("{error:#}")) })
        }),
        ("Independent memory", |index| {
            Box::pin(async move { isolated_worker_example(index).await.map_err(|error| format!("{error:#}")) })
        }),
        ("Rayon: sequential versus parallel", |index| {
            Box::pin(async move { rayon_worker_example(index).await.map_err(|error| format!("{error:#}")) })
        }),
    ];

    html_div()
        .children([
            html_p()
                .style("margin-bottom", "1rem")
                .text(
                    "Each category runs the same existing utilities. Timed computation excludes worker and pool startup. \
                     Run without concurrent jobs for cleaner comparisons.",
                )
                .into_dom(),
            html_div()
                .children(groups.into_iter().map(|(title, run)| {
                    html_div()
                        .children([
                            html_h3().class("subtitle").text(title).into_dom(),
                            html_div()
                                .class(["benchmark-grid"])
                                .children(COMPARISONS.iter().enumerate().map(|(index, &(name, _, input, calls))| {
                                    example_button(
                                        name,
                                        format!("{calls} calls to {name}({input})",),
                                        index,
                                        run,
                                        running.clone(),
                                    )
                                }))
                                .into_dom(),
                        ])
                        .into_dom()
                }))
                .into_dom(),
        ])
        .into_dom()
}

fn example_button(
    title: &'static str,
    description: String,
    index: usize,
    run: fn(usize) -> RunFuture,
    running: Mutable<bool>,
) -> dominator::Dom {
    let output = Mutable::new("Not run.".to_string());
    let click_running = running.clone();
    let click_output = output.clone();

    html_div()
        .class("box")
        .children([
            html_h3().class("subtitle").text(title).into_dom(),
            html_p().style("margin-bottom", "1rem").text(&description).into_dom(),
            html_button()
                .class(["button", "is-link"])
                .prop_signal("disabled", running.signal())
                .text("Run")
                .event(move |_: events::Click| {
                    if click_running.replace(true) {
                        return;
                    }

                    let running = click_running.clone();
                    let output = click_output.clone();

                    output.set("Running…".into());

                    wasm_bindgen_futures::spawn_local(async move {
                        output.set(match run(index).await {
                            Ok(message) => message,
                            Err(error) => format!("Failed: {error}"),
                        });

                        running.set(false);
                    });
                })
                .into_dom(),
            html_pre()
                .style("margin-top", "1rem")
                .style("white-space", "pre-wrap")
                .text_signal(output.signal_cloned())
                .into_dom(),
        ])
        .into_dom()
}

type Comparison = (&'static str, fn(u64) -> u64, u64, usize);

// Each configuration runs exactly the same calls in every execution mode.
//
// Enough independent calls are provided for the Rayon pool to distribute.
// Adjust inputs or call counts here to change the workload.
const COMPARISONS: [Comparison; 3] =
    [("fibonacci", fibonacci, 38, 16), ("collatz", collatz, 500_000, 32), ("prime_count", prime_count, 250_000, 32)];

fn run_batch(function: fn(u64) -> u64, input: u64, calls: usize) -> u64 {
    (0..calls).fold(0u64, |checksum, _| {
        let result = function(std::hint::black_box(input));

        checksum.wrapping_add(std::hint::black_box(result))
    })
}
