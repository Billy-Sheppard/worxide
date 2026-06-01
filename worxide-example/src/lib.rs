//! worxide-example

mod html_macros;
use std::{collections::HashSet, fmt::Debug, hash::Hash, ops::Not, sync::Arc};

use dominator::events;
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::{MutableVec, SignalVecExt},
};
use html_macros::*;

use wasm_bindgen::{self, prelude::*};

#[derive(Debug, Clone)]
struct Job<T, U> {
    name: Arc<str>,
    func: Arc<str>,
    f: fn(T) -> U,
    param: T,
    result: Mutable<Option<Result<U, String>>>,
}
impl<T: Debug, U> Job<T, U> {
    fn title(&self) -> String {
        format!("{}({:?})", self.func, self.param,)
    }
}
impl<T: Hash, U> Hash for Job<T, U> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.func.hash(state);
        self.f.hash(state);
        self.param.hash(state);
    }
}
impl<T: PartialEq, U> PartialEq for Job<T, U> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.func == other.func
            && std::ptr::fn_addr_eq(self.f, other.f)
            && self.param == other.param
    }
}
impl<T: Eq, U> Eq for Job<T, U> {}

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

#[wasm_bindgen]
pub async fn run_app() -> Result<(), JsValue> {
    #[allow(clippy::mutable_key_type)]
    let jobs = Mutable::new(HashSet::from([
        Job {
            name: Default::default(),
            func: "fibonacci".into(),
            f: fibonacci,
            param: 43,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "collatz".into(),
            f: collatz,
            param: 8000000,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "pi_leibniz".into(),
            f: pi_leibniz,
            param: 800000000,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "prime_count".into(),
            f: prime_count,
            param: 10000000,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "fibonacci".into(),
            f: fibonacci,
            param: 44,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "collatz".into(),
            f: collatz,
            param: 10000000,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "pi_leibniz".into(),
            f: pi_leibniz,
            param: 900000000,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "prime_count".into(),
            f: prime_count,
            param: 20000000,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "fibonacci".into(),
            f: fibonacci,
            param: 45,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "collatz".into(),
            f: collatz,
            param: 20000000,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "pi_leibniz".into(),
            f: pi_leibniz,
            param: 1000000000,
            result: Mutable::new(None),
        },
        Job {
            name: Default::default(),
            func: "prime_count".into(),
            f: prime_count,
            param: 30000000,
            result: Mutable::new(None),
        },
    ]));

    let mut names = HashSet::from([
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
    ]);

    let workers: MutableVec<Arc<Job<u64, u64>>> = MutableVec::new();

    let header = html_div()
        .class(["columns"])
        .child(
            html_div()
                .class(["column", "is-narrow"])
                .child(html_h1().class("title").text("worxide").into_dom())
                .into_dom(),
        )
        .child(
            html_div()
                .class(["column", "is-narrow"])
                .child(
                    html_button()
                        .class(["button", "fading-text"])
                        .style("height", "100%")
                        .text("main thread stays active")
                        .into_dom(),
                )
                .into_dom(),
        )
        .child(
            html_div()
                .class(["column", "is-narrow"])
                .child(
                    html_button()
                        .class(["button", "fading-text"])
                        .style("height", "100%")
                        .text("Main thread FPS: ")
                        .into_dom(),
                )
                .into_dom(),
        )
        .child(
            html_div()
                .class(["column", "is-narrow"])
                .child(
                    html_button()
                        .class(["button", "fading-text"])
                        .style("height", "100%")
                        .text("CPU Pressure")
                        .into_dom(),
                )
                .into_dom(),
        )
        .child(
            html_div()
                .class("column")
                .child(
                    html_button()
                        .class(["button", "is-loading"])
                        .visible_signal(
                            workers
                                .signal_vec_cloned()
                                .map_signal(|worker| worker.result.signal_cloned())
                                .to_signal_cloned()
                                .map(|vec: Vec<Option<Result<u64, String>>>| {
                                    vec.iter().any(Option::is_none)
                                }),
                        )
                        .style("height", "100%")
                        .into_dom(),
                )
                .into_dom(),
        )
        .into_dom();

    let dom = html_div()
        .class(["section"])
        .child(header)
        .child(
            html_button()
                .class(["button", "is-success"])
                .text("Spawn Worker")
                .visible_signal(jobs.signal_ref(|j| j.is_empty().not()))
                .event({
                    let workers = workers.clone();
                    let jobs = jobs.clone();
                    move |_: events::Click| {
                        let key = match names.iter().next().cloned().and_then(|k| names.take(k)) {
                            Some(k) => k,
                            None => return,
                        };

                        let job = {
                            let mut g = jobs.lock_mut();
                            let Some(k) = g.iter().next().cloned() else {
                                return;
                            };
                            g.take(&k)
                        };
                        let Some(mut job) = job else { return };

                        job.name = key.into();
                        let job = Arc::from(job);

                        workers.lock_mut().push_cloned(job.clone());

                        wasm_bindgen_futures::spawn_local(async move {
                            let f = job.f;
                            let param = job.param;
                            job.result.set(Some(
                                worxide::spawn_blocking!(f, param)
                                    .await
                                    .map_err(|e| e.to_string()),
                            ));
                        });
                    }
                })
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
                                        .child(html_h3().text(&worker.title()).into_dom())
                                        .child_signal(worker.result.signal_ref(|res| {
                                            html_button()
                                                .class("button")
                                                .style("width", "100%")
                                                .style("height", "40px")
                                                .apply_if(res.is_none(), |b| b.class("is-loading"))
                                                .apply_if(res.is_some(), |b| {
                                                    b.text(&match res.as_ref().unwrap() {
                                                        Ok(res) => format!("Res: {}", res),
                                                        Err(e) => e.to_string(),
                                                    })
                                                })
                                                .into_dom()
                                                .into()
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
