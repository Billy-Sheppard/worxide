use std::{sync::Arc, time::Duration};

use crate as worxide;
use dominator::{Dom, html};
use futures_signals::signal::{Mutable, SignalExt};
use rand::RngExt;
use wasm_bindgen::{JsCast, prelude::wasm_bindgen};

#[worxide::worker_fn]
async fn add_ten(n: u32) -> u32 {
    n + 10
}

#[worxide::worker_fn]
async fn reverse_words(s: String) -> String {
    s.split_whitespace().rev().collect::<Vec<_>>().join(" ")
}

// Intentionally naive recursive fibonacci — CPU-bound, ~2.7B calls for n=45.
// This is the kind of work that would freeze the main thread; on a worker
// the UI stays responsive the entire time.
#[worxide::worker_fn]
async fn fibonacci(n: u64) -> u64 {
    fn fib(n: u64) -> u64 {
        match n {
            0 => 0,
            1 => 1,
            _ => fib(n - 1) + fib(n - 2),
        }
    }
    fib(n)
}

#[worxide::worker_fn]
async fn sleepy_time(secs: u64) {
    gloo::timers::future::sleep(Duration::from_secs(secs)).await;
}

// Hot spin — burns 100% of a CPU core for `secs` seconds.
// On the main thread this would freeze the browser entirely.
// On a worker the page stays fully responsive.
#[worxide::worker_fn]
async fn spin_lock(secs: u64) {
    let start = js_sys::Date::now();
    let end = start + (secs * 1000) as f64;
    while js_sys::Date::now() < end {
        std::hint::spin_loop();
    }
}

// Spawns another worker from inside a worker — tests nested spawning.
// chain_start receives n, spawns add_ten(n) on a new worker, returns the result.
#[worxide::worker_fn]
async fn chain_start(n: u32) -> u32 {
    worxide::spawn!(add_ten, n).await
}

#[derive(Clone)]
struct Demo {
    label: &'static str,
    status: Mutable<Status>,
}

#[derive(Clone, PartialEq)]
enum Status {
    Waiting,
    Running,
    Done(String),
}

impl Status {
    fn text(&self) -> String {
        match self {
            Status::Waiting => "waiting…".into(),
            Status::Running => "running…".into(),
            Status::Done(s) => s.clone(),
        }
    }
}

fn render_demo(demo: &Arc<Demo>) -> Dom {
    let status = demo.status.clone();
    html!("div", {
        .class("demo")
        .child(html!("div", {
            .class("label")
            .text(demo.label)
        }))
        .child(html!("div", {
            .class("status")
            .class_signal("waiting", status.signal_cloned().map(|s| s == Status::Waiting))
            .class_signal("running", status.signal_cloned().map(|s| s == Status::Running))
            .class_signal("done",    status.signal_cloned().map(|s| matches!(s, Status::Done(_))))
            .text_signal(status.signal_cloned().map(|s| s.text()))
        }))
    })
}

fn render_app(demos: &[Arc<Demo>]) -> Dom {
    html!("div", {
        .class("app")
        .child(html!("h1", { .text("worxide") }))
        .child(html!("p", { .text("Web Worker tasks running off the main thread. The page stays responsive throughout.") }))
        .child(html!("div", {
            .class("heartbeat-row")
            .child(html!("div", { .class("spinner") }))
            .child(html!("span", {
                .class("heartbeat-label")
                .text("main thread — ")
            }))
            .child(html!("span", {
                .class("heartbeat-label")
                .attr("id", "heartbeat-ms")
                .text("0ms")
            }))
        }))
        .children(demos.iter().map(render_demo))
    })
}

#[wasm_bindgen(start)]
pub async fn start() {
    // Set up panic hooks and any other global init here — runs on both
    // main thread and workers so everything is covered in one place.
    console_error_panic_hook::set_once();

    // Initialise worxide. Scans wasm exports for registered worker fns and
    // prepares the main thread for spawning. Override the glue file name with
    // .js_file("./custom.js") if your build output uses a non-default name.
    worxide::Config::init().await;

    // Worker threads return after init — everything below is main thread only.
    if worxide::is_worker() {
        return;
    }

    let add_ten_demo = Arc::new(Demo {
        label: "add_ten(rand u32)",
        status: Mutable::new(Status::Waiting),
    });
    let reverse_demo = Arc::new(Demo {
        label: "reverse_words(\"the quick brown fox\")",
        status: Mutable::new(Status::Waiting),
    });
    let fib_demo = Arc::new(Demo {
        label: "fibonacci(45) — naive recursive, ~2.7B calls",
        status: Mutable::new(Status::Waiting),
    });
    let concurrent_demo = Arc::new(Demo {
        label: "3 × sleep(2s) concurrently — all start together, finish in ~2s not ~6s",
        status: Mutable::new(Status::Waiting),
    });
    let spinlock_demo = Arc::new(Demo {
        label: "spin_lock(5s) — 100% CPU core, main thread free",
        status: Mutable::new(Status::Waiting),
    });
    let chain_demo = Arc::new(Demo {
        label: "chain_start(n) — worker spawns add_ten(n) on another worker",
        status: Mutable::new(Status::Waiting),
    });

    let demos = vec![
        add_ten_demo.clone(),
        reverse_demo.clone(),
        fib_demo.clone(),
        spinlock_demo.clone(),
        concurrent_demo.clone(),
        chain_demo.clone(),
    ];

    // Mount the UI.
    dominator::append_dom(&dominator::body(), render_app(&demos));
    inject_styles();

    // Kick off a rAF loop that updates a ms counter on the main thread.
    // If the spinner freezes or the counter stalls, the main thread is blocked.
    let heartbeat = start_heartbeat();

    let perf = web_sys::window().unwrap().performance().unwrap();
    let mut rng = rand::rng();
    let rand = rng.random::<u32>();

    // add_ten
    add_ten_demo.status.set(Status::Running);
    let t = perf.now();
    let result = worxide::spawn!(add_ten, rand).await;
    let elapsed = perf.now() - t;
    add_ten_demo.status.set(Status::Done(format!(
        "add_ten — {} + 10 = {} [took {:.3}s]",
        rand,
        result,
        elapsed / 1000.0
    )));

    // reverse_words
    reverse_demo.status.set(Status::Running);
    let t = perf.now();
    let reversed = worxide::spawn!(reverse_words, "the quick brown fox".to_string()).await;
    let elapsed = perf.now() - t;
    reverse_demo.status.set(Status::Done(format!(
        "reverse_words — \"the quick brown fox\" = \"{}\" [took {:.3}s]",
        reversed,
        elapsed / 1000.0
    )));

    // fibonacci(45) — CPU bound, runs alone so you can see it in task manager
    fib_demo.status.set(Status::Running);
    let t = perf.now();
    let fib_result = worxide::spawn!(fibonacci, 45u64).await;
    let elapsed = perf.now() - t;
    fib_demo.status.set(Status::Done(format!(
        "fibonacci — fib(45) = {} [took {:.3}s]",
        fib_result,
        elapsed / 1000.0
    )));

    // spin_lock — hot CPU spin, watch task manager for a worker thread at 100%
    spinlock_demo.status.set(Status::Running);
    let t = perf.now();
    worxide::spawn!(spin_lock, 5u64).await;
    let elapsed = perf.now() - t;
    spinlock_demo.status.set(Status::Done(format!(
        "spin_lock — spun 5s [took {:.3}s]",
        elapsed / 1000.0
    )));

    // 3 concurrent sleeps — spawned together, all finish in ~2s not ~6s
    concurrent_demo.status.set(Status::Running);
    let t = perf.now();
    futures::join!(
        worxide::spawn!(sleepy_time, 2u64),
        worxide::spawn!(sleepy_time, 2u64),
        worxide::spawn!(sleepy_time, 2u64),
    );
    let elapsed = perf.now() - t;
    concurrent_demo.status.set(Status::Done(format!(
        "sleepy_time — 3 × sleep(2s) [took {:.3}s]",
        elapsed / 1000.0
    )));

    // worker-from-worker — chain_start spawns add_ten on a nested worker
    chain_demo.status.set(Status::Running);
    let t = perf.now();
    let chain_result = worxide::spawn!(chain_start, rand).await;
    let elapsed = perf.now() - t;
    chain_demo.status.set(Status::Done(format!(
        "chain_start — worker spawned add_ten({}) = {} [took {:.3}s]",
        rand,
        chain_result,
        elapsed / 1000.0
    )));

    heartbeat.set(false);

    // Stop the spinner animation
    let document = web_sys::window().unwrap().document().unwrap();
    if let Some(el) = document.query_selector(".spinner").unwrap() {
        el.class_list().remove_1("spinner").unwrap();
        el.class_list().add_1("spinner-done").unwrap();
    }
}

#[allow(clippy::type_complexity)]
fn start_heartbeat() -> std::rc::Rc<std::cell::Cell<bool>> {
    use wasm_bindgen::closure::Closure;
    use web_sys::window;

    let running = std::rc::Rc::new(std::cell::Cell::new(true));
    let running_clone = running.clone();

    let win = window().unwrap();
    let perf = win.performance().unwrap();
    let t0 = perf.now();

    let cb: std::rc::Rc<std::cell::RefCell<Option<Closure<dyn FnMut()>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let cb_clone = cb.clone();

    *cb.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if !running_clone.get() {
            return; // stopped — don't reschedule
        }
        let elapsed = perf.now() - t0;
        let document = web_sys::window().unwrap().document().unwrap();
        if let Some(el) = document.get_element_by_id("heartbeat-ms") {
            el.set_text_content(Some(&format!("{:.0}ms", elapsed)));
        }
        web_sys::window()
            .unwrap()
            .request_animation_frame(cb_clone.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .unwrap();
    }) as Box<dyn FnMut()>));

    win.request_animation_frame(cb.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .unwrap();

    std::mem::forget(cb);
    running
}

fn inject_styles() {
    let style = r#"
        * { box-sizing: border-box; margin: 0; padding: 0; }

        body {
            background: #0f1117;
            color: #e2e8f0;
            font-family: 'Segoe UI', system-ui, sans-serif;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }

        .app {
            width: 100%;
            max-width: 680px;
            padding: 2rem;
        }

        h1 {
            font-size: 2rem;
            font-weight: 700;
            color: #a78bfa;
            margin-bottom: 0.25rem;
            letter-spacing: -0.5px;
        }

        p {
            color: #94a3b8;
            font-size: 0.9rem;
            margin-bottom: 2rem;
        }

        .demo {
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
            padding: 0.875rem 1.25rem;
            margin-bottom: 0.75rem;
            background: #1e2130;
            border-radius: 10px;
            border: 1px solid #2d3148;
        }

        .label {
            font-family: 'Cascadia Code', 'Fira Code', monospace;
            font-size: 0.82rem;
            color: #94a3b8;
        }

        .status {
            font-size: 0.82rem;
            font-weight: 500;
            padding: 0.4rem 0.75rem;
            border-radius: 6px;
            width: 100%;
        }

        .status.waiting {
            background: #1e293b;
            color: #475569;
        }

        .status.running {
            background: #1e3a5f;
            color: #60a5fa;
            animation: pulse 1.2s ease-in-out infinite;
        }

        .status.done {
            background: #14532d;
            color: #4ade80;
        }

        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50%       { opacity: 0.5; }
        }

        .heartbeat-row {
            display: flex;
            align-items: center;
            gap: 0.5rem;
            margin-bottom: 1.5rem;
            padding: 0.6rem 1rem;
            background: #1e2130;
            border-radius: 8px;
            border: 1px solid #2d3148;
        }

        .heartbeat-label {
            font-size: 0.8rem;
            color: #94a3b8;
            font-family: 'Cascadia Code', 'Fira Code', monospace;
        }

        .spinner {
            width: 14px;
            height: 14px;
            border: 2px solid #2d3148;
            border-top-color: #a78bfa;
            border-radius: 50%;
            animation: spin 0.7s linear infinite;
            flex-shrink: 0;
        }

        @keyframes spin {
            to { transform: rotate(360deg); }
        }

        .spinner-done {
            width: 14px;
            height: 14px;
            border: 2px solid #14532d;
            border-radius: 50%;
            background: #4ade80;
            flex-shrink: 0;
        }
    "#;

    let document = web_sys::window().unwrap().document().unwrap();
    let el = document.create_element("style").unwrap();
    el.set_text_content(Some(style));
    document.head().unwrap().append_child(&el).unwrap();
}
