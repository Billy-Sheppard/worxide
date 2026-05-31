//! worxide-example — a live dashboard demonstrating non-blocking compute.
//!
//! On load we kick off several CPU-heavy fibonacci jobs concurrently on
//! Web Workers via worxide. The main thread stays free: an always-spinning
//! element keeps animating smoothly, a frame-time meter shows the main
//! thread isn't janking, and (where supported) the Compute Pressure API
//! reports real CPU pressure. Each job is a card showing a random word
//! label, its status, the fibonacci input, the result, and wall-clock time.
//!
//! All markup AND styling are defined here via dominator; index.html is a
//! bare shell.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use dominator::{Dom, class, clone, html, stylesheet};
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

// ── CPU-heavy work that runs on a worker ─────────────────────────────────────

// ── CPU-heavy tasks ──────────────────────────────────────────────────────────
//
// A spread of workloads with different computational shapes: recursive,
// iterative-numeric, floating-point, bitwise/crypto, and memory-churning.
// Each runs to completion on its own worker.

/// What kind of work a job does. Carries its input parameter.
#[derive(Clone, Copy)]
enum Task {
    /// Naive recursive fibonacci — exponential recursion, call-stack heavy.
    Fib(u64),
    /// Count primes below N via trial division — tight integer loops.
    PrimeCount(u64),
    /// Mandelbrot escape-time over an NxN grid — dense floating point.
    Mandelbrot(u32),
    /// Iterated SHA-256 (hash the previous digest N times) — bitwise/crypto.
    HashRounds(u32),
    /// Sort N pseudo-random u64s — memory churn + comparisons.
    Sort(usize),
    /// Collatz: sum the stopping times for all starts 1..=N — branchy integer.
    Collatz(u64),
    /// Leibniz series for π to N terms — sequential floating-point accumulation.
    PiLeibniz(u64),
    /// Count words in a procedurally generated text of N tokens — string churn.
    WordCount(usize),
}

impl Task {
    /// Short task-type name for the card.
    fn kind(&self) -> &'static str {
        match self {
            Task::Fib(_) => "fibonacci",
            Task::PrimeCount(_) => "prime-count",
            Task::Mandelbrot(_) => "mandelbrot",
            Task::HashRounds(_) => "sha256-rounds",
            Task::Sort(_) => "sort",
            Task::Collatz(_) => "collatz",
            Task::PiLeibniz(_) => "pi-leibniz",
            Task::WordCount(_) => "word-count",
        }
    }

    /// Human-readable input description for the card (e.g. "n = 40").
    fn input_desc(&self) -> String {
        match self {
            Task::Fib(n) => format!("n = {n}"),
            Task::PrimeCount(n) => format!("N = {n}"),
            Task::Mandelbrot(n) => format!("{n}×{n} grid"),
            Task::HashRounds(n) => format!("{n} rounds"),
            Task::Sort(n) => format!("{n} items"),
            Task::Collatz(n) => format!("1..={n}"),
            Task::PiLeibniz(n) => format!("{n} terms"),
            Task::WordCount(n) => format!("{n} tokens"),
        }
    }
}

/// Naive recursive fibonacci.
fn fib(n: u64) -> u64 {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

/// Count primes strictly below `limit` by trial division.
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

/// Mandelbrot escape-time: count points (in an NxN grid over the classic
/// view) that stay bounded after a fixed iteration cap. Returns that count.
fn mandelbrot(grid: u32) -> u64 {
    const MAX_ITER: u32 = 1000;
    let mut inside = 0u64;
    for py in 0..grid {
        for px in 0..grid {
            let x0 = (px as f64 / grid as f64) * 3.5 - 2.5;
            let y0 = (py as f64 / grid as f64) * 2.0 - 1.0;
            let (mut x, mut y) = (0.0f64, 0.0f64);
            let mut iter = 0u32;
            while x * x + y * y <= 4.0 && iter < MAX_ITER {
                let xt = x * x - y * y + x0;
                y = 2.0 * x * y + y0;
                x = xt;
                iter += 1;
            }
            if iter == MAX_ITER {
                inside += 1;
            }
        }
    }
    inside
}

/// Iterated SHA-256: start from a seed and hash the digest `rounds` times.
/// Returns the first 8 bytes of the final digest as a u64. Self-contained
/// SHA-256 (no deps) so the worker stays dependency-light.
fn hash_rounds(rounds: u32) -> u64 {
    let mut digest = sha256(b"worxide");
    for _ in 0..rounds {
        digest = sha256(&digest);
    }
    let mut out = 0u64;
    for &b in digest.iter().take(8) {
        out = (out << 8) | b as u64;
    }
    out
}

/// Minimal SHA-256 over a byte slice. Returns the 32-byte digest.
fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    // Pad.
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..i * 4 + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

/// Sort `n` pseudo-random u64s (xorshift-generated) and return a checksum
/// (sum of every 1000th element) so the work can't be optimized away.
fn sort_checksum(n: usize) -> u64 {
    let mut state = 0x9e3779b97f4a7c15u64;
    let mut v: Vec<u64> = (0..n)
        .map(|_| {
            // xorshift64
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        })
        .collect();
    v.sort_unstable();
    v.iter()
        .step_by(1000)
        .fold(0u64, |acc, &x| acc.wrapping_add(x))
}

/// Sum the Collatz stopping times for every start value 1..=limit.
/// Heavily branchy integer work with unpredictable iteration counts.
fn collatz_total(limit: u64) -> u64 {
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

/// Approximate π via the Leibniz series to `terms` terms, returning the result
/// scaled to an integer (×1e9). Sequential floating-point accumulation that
/// resists vectorization due to the running sign + denominator.
fn pi_leibniz(terms: u64) -> u64 {
    let mut sum = 0.0f64;
    let mut sign = 1.0f64;
    for k in 0..terms {
        sum += sign / (2 * k + 1) as f64;
        sign = -sign;
    }
    ((sum * 4.0) * 1e9) as u64
}

/// Generate `tokens` pseudo-random lowercase words, then count distinct words
/// by inserting into a hash map. String allocation + hashing churn.
fn word_count(tokens: usize) -> u64 {
    use std::collections::HashMap;
    let mut state = 0xdeadbeefcafef00du64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut counts: HashMap<String, u32> = HashMap::new();
    let mut buf = String::with_capacity(8);
    for _ in 0..tokens {
        buf.clear();
        // 3–8 letter word from the RNG.
        let len = 3 + (next() % 6) as usize;
        for _ in 0..len {
            let c = (b'a' + (next() % 26) as u8) as char;
            buf.push(c);
        }
        *counts.entry(buf.clone()).or_insert(0) += 1;
    }
    counts.len() as u64
}

/// Run a task to completion. Returns (result_value, millis_spent_in_worker).
/// The result is a u64 "signature" of the work (count/checksum/digest), enough
/// to display and to prevent dead-code elimination.
fn run_task(task: Task) -> (u64, f64) {
    let start = now_ms();
    let result = match task {
        Task::Fib(n) => fib(n),
        Task::PrimeCount(n) => prime_count(n),
        Task::Mandelbrot(n) => mandelbrot(n),
        Task::HashRounds(n) => hash_rounds(n),
        Task::Sort(n) => sort_checksum(n),
        Task::Collatz(n) => collatz_total(n),
        Task::PiLeibniz(n) => pi_leibniz(n),
        Task::WordCount(n) => word_count(n),
    };
    (result, now_ms() - start)
}

// ── Per-job reactive state ───────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Running,
    Done,
}

struct JobState {
    label: String,
    kind: &'static str,
    input: String,
    status: Mutable<Status>,
    result: Mutable<Option<u64>>,
    elapsed: Mutable<f64>,           // wall-clock, ticks live while running
    worker_ms: Mutable<Option<f64>>, // worker-measured compute time
}

impl JobState {
    fn new(label: String, task: Task) -> Arc<Self> {
        Arc::new(Self {
            label,
            kind: task.kind(),
            input: task.input_desc(),
            status: Mutable::new(Status::Running),
            result: Mutable::new(None),
            elapsed: Mutable::new(0.0),
            worker_ms: Mutable::new(None),
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn random_word() -> String {
    const WORDS: &[&str] = &[
        "falcon", "ember", "quartz", "willow", "cobalt", "nimbus", "thistle", "harbor", "vortex",
        "cedar", "marble", "zephyr", "lumen", "onyx", "saffron", "drift", "pinecone", "basalt",
        "comet", "tundra",
    ];
    let idx = (js_sys::Math::random() * WORDS.len() as f64) as usize;
    WORDS[idx.min(WORDS.len() - 1)].to_owned()
}

/// Current high-resolution time in ms. Works on the main thread (Window) AND
/// inside workers (WorkerGlobalScope) — both expose `performance.now()`, but
/// via different globals. Workers have no `window`, so the old window-only
/// version returned 0.0 there, which is why worker "compute" times read 0 ms.
fn now_ms() -> f64 {
    let global = js_sys::global();
    // Both WorkerGlobalScope and Window have a `performance` property.
    if let Ok(perf) = js_sys::Reflect::get(&global, &JsValue::from_str("performance"))
        && !perf.is_undefined()
        && let Ok(now) = js_sys::Reflect::get(&perf, &JsValue::from_str("now"))
        && let Ok(now_fn) = now.dyn_into::<js_sys::Function>()
        && let Ok(v) = now_fn.call0(&perf)
    {
        return v.as_f64().unwrap_or(0.0);
    }
    0.0
}

/// Yield to the event loop via setTimeout, allowing a paint between ticks.
/// Used by the live timer loop so it updates visibly without starving render.
async fn yield_to_event_loop() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
                resolve.unchecked_ref(),
                33, // ~30Hz update is plenty for a timer readout
            );
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

// ── Main-thread health meter (requestAnimationFrame delta) ───────────────────

type FrameCb = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

fn start_frame_meter(frame_ms: Mutable<f64>) {
    let last = Rc::new(RefCell::new(now_ms()));
    let cb: FrameCb = Rc::new(RefCell::new(None));
    let cb2 = cb.clone();
    *cb.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let t = now_ms();
        let dt = t - *last.borrow();
        *last.borrow_mut() = t;
        let prev = frame_ms.get();
        frame_ms.set(prev * 0.8 + dt * 0.2);
        if let Some(w) = web_sys::window()
            && let Some(c) = cb2.borrow().as_ref()
        {
            let _ = w.request_animation_frame(c.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut()>));
    if let Some(w) = web_sys::window()
        && let Some(c) = cb.borrow().as_ref()
    {
        let _ = w.request_animation_frame(c.as_ref().unchecked_ref());
    }
}

// ── Compute Pressure API (real CPU pressure where supported) ─────────────────

fn start_pressure_observer(state: Mutable<Option<String>>) {
    let global = js_sys::global();
    let ctor = match js_sys::Reflect::get(&global, &JsValue::from_str("PressureObserver")) {
        Ok(v) if !v.is_undefined() => v,
        _ => return,
    };
    let cb = Closure::wrap(Box::new(clone!(state => move |records: JsValue| {
        if let Ok(arr) = records.dyn_into::<js_sys::Array>() {
            let len = arr.length();
            if len > 0
                && let Ok(s) = js_sys::Reflect::get(&arr.get(len - 1), &JsValue::from_str("state"))
                && let Some(s) = s.as_string()
            {
                state.set(Some(s));
            }
        }
    })) as Box<dyn FnMut(JsValue)>);
    let ctor_fn: js_sys::Function = ctor.unchecked_into();
    let observer = match js_sys::Reflect::construct(
        &ctor_fn,
        &js_sys::Array::of1(cb.as_ref().unchecked_ref()),
    ) {
        Ok(o) => o,
        Err(_) => {
            cb.forget();
            return;
        }
    };
    cb.forget();
    if let Ok(observe) = js_sys::Reflect::get(&observer, &JsValue::from_str("observe"))
        && let Ok(observe_fn) = observe.dyn_into::<js_sys::Function>()
    {
        let opts = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &opts,
            &JsValue::from_str("sampleInterval"),
            &JsValue::from_f64(1000.0),
        );
        let _ = observe_fn.call2(&observer, &JsValue::from_str("cpu"), &opts);
    }
}

// ── Styles (all defined in Rust via dominator) ───────────────────────────────

fn install_global_styles() {
    stylesheet!("html, body", {
        .style("margin", "0")
        .style("padding", "0")
    });
    stylesheet!("body", {
        .style("background", "#0b0e14")
        .style("color", "#e6edf3")
        .style("font", "14px/1.5 ui-monospace, 'SF Mono', Menlo, monospace")
        .style("min-height", "100vh")
    });
    // Keyframes can't be expressed via stylesheet! property calls, so inject
    // them once as a raw <style> rule.
    inject_raw_css(
        "
        @keyframes worxide-spin { to { transform: rotate(360deg); } }
        @keyframes worxide-pulse {
            0%   { box-shadow: 0 0 0 0 rgba(122,162,247,0.5); }
            100% { box-shadow: 0 0 0 8px rgba(122,162,247,0); }
        }
    ",
    );
}

/// Append a raw CSS string in a <style> tag (for @keyframes etc).
fn inject_raw_css(css: &str) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document())
        && let Ok(el) = doc.create_element("style")
    {
        el.set_text_content(Some(css));
        if let Some(head) = doc.head() {
            let _ = head.append_child(&el);
        }
    }
}

fn app() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("max-width", "1100px")
            .style("margin", "0 auto")
            .style("padding", "32px 24px 64px")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn header() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("display", "flex")
            .style("align-items", "center")
            .style("justify-content", "space-between")
            .style("gap", "24px")
            .style("flex-wrap", "wrap")
            .style("padding", "20px 24px")
            .style("margin-bottom", "28px")
            .style("background", "linear-gradient(180deg, #141925, #1b2230)")
            .style("border", "1px solid #232b3a")
            .style("border-radius", "14px")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn brand() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("display", "flex")
            .style("align-items", "center")
            .style("gap", "14px")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn spinner() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("width", "28px")
            .style("height", "28px")
            .style("border-radius", "50%")
            .style("border", "3px solid #232b3a")
            .style("border-top-color", "#5ad1c7")
            .style("border-right-color", "#7aa2f7")
            .style("animation", "worxide-spin 0.7s linear infinite")
            .style("flex", "none")
            .style("display", "flex")
            .style("align-items", "center")
            .style("justify-content", "center")
            .style("font-size", "15px")
            .style("font-weight", "700")
            .style("color", "#9ece6a")
            .style("transition", "border-color 0.3s ease")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

/// Applied to the spinner once all jobs finish: stops spinning and shows a
/// solid green ring around the checkmark.
fn spinner_done() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("animation", "none")
            .style("border-color", "#9ece6a")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn title() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("font-size", "20px")
            .style("margin", "0")
            .style("letter-spacing", "0.5px")
            .style("background", "linear-gradient(90deg, #5ad1c7, #7aa2f7)")
            .style("-webkit-background-clip", "text")
            .style("background-clip", "text")
            .style("color", "transparent")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn tagline() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("color", "#8b98a9")
            .style("font-size", "12px")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn meters() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("display", "flex")
            .style("gap", "20px")
            .style("flex-wrap", "wrap")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn meter() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("min-width", "220px")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn meter_label() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("display", "block")
            .style("color", "#8b98a9")
            .style("font-size", "11px")
            .style("text-transform", "uppercase")
            .style("letter-spacing", "1px")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn meter_value() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("font-size", "13px")
            .style("color", "#e6edf3")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn bar() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("margin-top", "6px")
            .style("height", "6px")
            .style("border-radius", "4px")
            .style("background", "#1b2230")
            .style("overflow", "hidden")
            .style("border", "1px solid #232b3a")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn bar_fill() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("height", "100%")
            .style("width", "0%")
            .style("background", "linear-gradient(90deg, #9ece6a, #5ad1c7)")
            .style("transition", "width 0.15s ease")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn grid() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("display", "grid")
            .style("gap", "14px")
            .style("grid-template-columns", "repeat(auto-fill, minmax(240px, 1fr))")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn card() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("background", "#141925")
            .style("border", "1px solid #232b3a")
            .style("border-radius", "12px")
            .style("overflow", "hidden")
            .style("transition", "border-color 0.3s ease")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn card_head() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("display", "flex")
            .style("align-items", "center")
            .style("gap", "10px")
            .style("padding", "12px 14px")
            .style("border-bottom", "1px solid #232b3a")
            .style("background", "#1b2230")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn dot() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("width", "9px")
            .style("height", "9px")
            .style("border-radius", "50%")
            .style("flex", "none")
            .style("background", "#7aa2f7")
            .style("animation", "worxide-pulse 1.1s ease-out infinite")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn dot_done() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("background", "#9ece6a")
            .style("animation", "none")
            .style("box-shadow", "none")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn label() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("font-weight", "600")
            .style("color", "#e6edf3")
            .style("flex", "1")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn status() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("font-size", "11px")
            .style("color", "#8b98a9")
            .style("text-transform", "uppercase")
            .style("letter-spacing", "1px")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn card_body() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("padding", "12px 14px")
            .style("display", "flex")
            .style("flex-direction", "column")
            .style("gap", "8px")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn row() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("display", "flex")
            .style("align-items", "baseline")
            .style("justify-content", "space-between")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn k() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("color", "#8b98a9")
            .style("font-size", "12px")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn v() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("color", "#e6edf3")
            .style("font-variant-numeric", "tabular-nums")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

fn badge() -> String {
    thread_local! { static C: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
    C.with(|c| {
        if let Some(s) = c.borrow().as_ref() {
            return s.clone();
        }
        let s = class! {
            .style("font-size", "10px")
            .style("text-transform", "uppercase")
            .style("letter-spacing", "0.5px")
            .style("color", "#5ad1c7")
            .style("background", "#10302d")
            .style("border", "1px solid #1d4b46")
            .style("border-radius", "5px")
            .style("padding", "2px 7px")
            .style("flex", "none")
        };
        *c.borrow_mut() = Some(s.clone());
        s
    })
}

// ── UI rendering ─────────────────────────────────────────────────────────────

fn render_job(job: Arc<JobState>) -> Dom {
    html!("div", {
        .class(&card())
        .children(&mut [
            html!("div", {
                .class(&card_head())
                .children(&mut [
                    html!("span", {
                        .class(&dot())
                        .class_signal(dot_done(), job.status.signal().map(|s| s == Status::Done))
                    }),
                    html!("span", { .class(&label()).text(&job.label) }),
                    html!("span", { .class(&badge()).text(job.kind) }),
                    html!("span", {
                        .class(&status())
                        .text_signal(job.status.signal().map(|s| match s {
                            Status::Running => "running",
                            Status::Done    => "done",
                        }))
                    }),
                ])
            }),
            html!("div", {
                .class(&card_body())
                .children(&mut [
                    job_row("task", job.kind),
                    job_row("input", &job.input),
                    html!("div", {
                        .class(&row())
                        .children(&mut [
                            html!("span", { .class(&k()).text("result") }),
                            html!("span", {
                                .class(&v())
                                .text_signal(job.result.signal().map(|r| match r {
                                    Some(v) => v.to_string(),
                                    None    => "…".to_owned(),
                                }))
                            }),
                        ])
                    }),
                    html!("div", {
                        .class(&row())
                        .children(&mut [
                            html!("span", { .class(&k()).text("wall") }),
                            html!("span", {
                                .class(&v())
                                .text_signal(job.elapsed.signal().map(|e| {
                                    if e > 0.0 { format!("{:.0} ms", e) } else { "—".to_owned() }
                                }))
                            }),
                        ])
                    }),
                    html!("div", {
                        .class(&row())
                        .children(&mut [
                            html!("span", { .class(&k()).text("compute") }),
                            html!("span", {
                                .class(&v())
                                .text_signal(job.worker_ms.signal().map(|w| match w {
                                    Some(ms) => format!("{:.0} ms", ms),
                                    None     => "…".to_owned(),
                                }))
                            }),
                        ])
                    }),
                ])
            }),
        ])
    })
}

fn job_row(key: &str, val: &str) -> Dom {
    html!("div", {
        .class(&row())
        .children(&mut [
            html!("span", { .class(&k()).text(key) }),
            html!("span", { .class(&v()).text(val) }),
        ])
    })
}

fn render_app(
    jobs: MutableVec<Arc<JobState>>,
    frame_ms: Mutable<f64>,
    pressure: Mutable<Option<String>>,
    done_count: Mutable<usize>,
    total: usize,
) -> Dom {
    // True once every job has finished.
    let all_done = move || done_count.signal().map(move |d| d >= total);

    html!("div", {
        .class(&app())
        .children(&mut [
            html!("div", {
                .class(&header())
                .children(&mut [
                    html!("div", {
                        .class(&brand())
                        .children(&mut [
                            // Spinner while work is in flight; swaps to a
                            // checkmark badge once all jobs complete.
                            html!("div", {
                                .class(&spinner())
                                .class_signal(spinner_done(), all_done())
                                .text_signal(all_done().map(|d| if d { "✓" } else { "" }))
                            }),
                            html!("h1", { .class(&title()).text("worxide") }),
                            html!("span", {
                                .class(&tagline())
                                .text("Rust on Web Workers — main thread stays free")
                            }),
                        ])
                    }),
                    html!("div", {
                        .class(&meters())
                        .children(&mut [
                            html!("div", {
                                .class(&meter())
                                .children(&mut [
                                    html!("span", { .class(&meter_label()).text("main thread") }),
                                    html!("span", {
                                        .class(&meter_value())
                                        .text_signal(frame_ms.signal().map(|ms| {
                                            let fps = if ms > 0.0 { 1000.0 / ms } else { 0.0 };
                                            format!("{:.0} fps ({:.1} ms/frame)", fps, ms)
                                        }))
                                    }),
                                    html!("div", {
                                        .class(&bar())
                                        .children(&mut [
                                            html!("div", {
                                                .class(&bar_fill())
                                                .style_signal("width", frame_ms.signal().map(|ms| {
                                                    format!("{:.0}%", (ms / 16.7 * 100.0).min(100.0))
                                                }))
                                            }),
                                        ])
                                    }),
                                ])
                            }),
                            html!("div", {
                                .class(&meter())
                                .visible_signal(pressure.signal_ref(|p| p.is_some()))
                                .children(&mut [
                                    html!("span", { .class(&meter_label()).text("CPU pressure") }),
                                    html!("span", {
                                        .class(&meter_value())
                                        .style("text-transform", "capitalize")
                                        .text_signal(pressure.signal_cloned()
                                            .map(|p| p.unwrap_or_else(|| "—".to_owned())))
                                    }),
                                ])
                            }),
                        ])
                    }),
                ])
            }),
            html!("div", {
                .class(&grid())
                .children_signal_vec(jobs.signal_vec_cloned().map(render_job))
            }),
        ])
    })
}

// ── Entry point ──────────────────────────────────────────────────────────────

/// Main-thread entry point. Called explicitly from index.html via init+run.
/// NOT a `#[wasm_bindgen(start)]` — that would run on every wasm instantiation
/// including each worker, where there is no DOM (and dominator's globals would
/// panic and poison shared state). Workers never call this; they only run
/// worxide's worker entry points.
#[wasm_bindgen]
pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("panic: {info}").into());
    }));

    web_sys::console::log_1(&"[worxide-example] run() on main thread".into());
    install_global_styles();

    // CRITICAL: this start function runs on EVERY instantiation of the wasm
    // module — including each Web Worker that worxide spins up. Workers have
    // no DOM (no `window`, no `document`), so any UI work there would panic
    // and poison our style statics. Only build the UI on the main thread.
    //
    web_sys::console::log_1(&"[worxide-example] main: styles installed".into());

    let jobs: MutableVec<Arc<JobState>> = MutableVec::new();
    let frame_ms = Mutable::new(16.7);
    let pressure: Mutable<Option<String>> = Mutable::new(None);
    let done_count = Mutable::new(0usize);

    start_frame_meter(frame_ms.clone());
    start_pressure_observer(pressure.clone());

    // Kick off a varied set of CPU-heavy jobs concurrently, each on its own
    // worker. Inputs are tuned HEAVY — each task should grind for several
    // seconds, fully saturating its core, so the spread really exercises the
    // CPU while the main thread stays smooth.
    let tasks = [
        Task::Fib(44),
        Task::PrimeCount(10_000_000),
        Task::Mandelbrot(2200),
        Task::HashRounds(6_000_000),
        Task::Sort(25_000_000),
        Task::Collatz(8_000_000),
        Task::PiLeibniz(800_000_000),
        Task::WordCount(15_000_000),
        Task::Fib(45),
        Task::PrimeCount(14_000_000),
        Task::Mandelbrot(2600),
        Task::Collatz(12_000_000),
    ];
    let total = tasks.len();

    dominator::append_dom(
        &dominator::body(),
        render_app(
            jobs.clone(),
            frame_ms.clone(),
            pressure.clone(),
            done_count.clone(),
            total,
        ),
    );

    for &task in tasks.iter() {
        let job = JobState::new(random_word(), task);
        jobs.lock_mut().push_cloned(job.clone());

        wasm_bindgen_futures::spawn_local(clone!(job, done_count => async move {
            let wall_start = now_ms();

            // Tick a live elapsed timer on the main thread while the worker
            // grinds, so the card shows time climbing. Stops when status flips
            // to Done. This loop does a tiny bit of work per frame, so it never
            // blocks — and visibly proves the main thread stays responsive
            // while the workers are saturated.
            wasm_bindgen_futures::spawn_local(clone!(job => async move {
                while job.status.get() == Status::Running {
                    job.elapsed.set(now_ms() - wall_start);
                    yield_to_event_loop().await;
                }
            }));

            match worxide::spawn_blocking!(run_task, task).await {
                Ok((result, worker_ms)) => {
                    job.result.set(Some(result));
                    job.elapsed.set(now_ms() - wall_start);
                    job.worker_ms.set(Some(worker_ms));
                    job.status.set(Status::Done);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("job {} failed: {e}", job.label).into());
                    job.status.set(Status::Done);
                }
            }
            // Mark this job complete for the header's all-done indicator.
            done_count.set(done_count.get() + 1);
        }));
    }
}
