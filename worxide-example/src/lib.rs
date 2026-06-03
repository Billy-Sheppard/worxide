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
                    Mode::Worker => worxide::spawn_blocking!(f, param)
                        .await
                        .map(|r| format!("Res: {r}"))
                        .map_err(|e| e.to_string()),
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

    let worker = Arc::new(Worker {
        name: key.into(),
        title: Arc::from(task.title.as_str()),
        result: Mutable::new(None),
    });
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
                                    html_button()
                                        .class(["button", "is-loading"])
                                        .style("height", "100%")
                                        .into_dom(),
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
                                .text_signal(pressure.signal_cloned().map(|p| {
                                    format!("CPU Pressure: {}", p.unwrap_or_else(|| "—".into()))
                                }))
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
                                .class_signal(
                                    "is-warning",
                                    mode.signal().map(|m| m == Mode::MainThread),
                                )
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
                                                while spawn_job(&names, &pool, &workers, mode.get())
                                                {
                                                }
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
                                        .child(
                                            html_h2()
                                                .class("subtitle")
                                                .text(worker.name.as_ref())
                                                .into_dom(),
                                        )
                                        .child(html_h3().text(worker.title.as_ref()).into_dom())
                                        .child_signal(worker.result.signal_cloned().map(|res| {
                                            Some(
                                                html_button()
                                                    .class("button")
                                                    .style("width", "100%")
                                                    .style("height", "40px")
                                                    .apply_if(res.is_none(), |b| {
                                                        b.class("is-loading")
                                                    })
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
            w.request_animation_frame(c.as_ref().unchecked_ref())
                .unwrap();
        }
    }) as Box<dyn FnMut()>)));

    if let Some(w) = web_sys::window()
        && let Some(c) = callback.lock_ref().as_ref()
    {
        w.request_animation_frame(c.as_ref().unchecked_ref())
            .unwrap();
    }
}

fn start_pressure_observer(pressure: Mutable<Option<String>>) {
    let constructor = if let Ok(v) =
        js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("PressureObserver"))
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
                && let Ok(s) =
                    js_sys::Reflect::get(&arr.get(len.sub(1)), &JsValue::from_str("state"))
                && let Some(s) = s.as_string()
            {
                pressure.set(Some(s));
            }
        }
    }) as Box<dyn FnMut(JsValue)>);

    let observer = if let Ok(o) = js_sys::Reflect::construct(
        &constructor,
        &js_sys::Array::of1(cb.as_ref().unchecked_ref()),
    ) {
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
        js_sys::Reflect::set(
            &opts,
            &JsValue::from_str("sampleInterval"),
            &JsValue::from_f64(500.0),
        )
        .unwrap();
        observe_fn
            .call2(&observer, &JsValue::from_str("cpu"), &opts)
            .unwrap();
    }
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}
