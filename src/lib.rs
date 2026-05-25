//! worxide — tokio mpsc across shared wasm memory
//!
//! Main thread creates a tokio::sync::mpsc channel, sends 3 messages,
//! leaks the Receiver, and passes its pointer to a Worker.
//! Worker reconstructs the Receiver and drains the channel with try_recv.
//!
//! The channel queue, atomic state, and message slots all live in shared
//! linear memory — this proves the two wasm instances are truly sharing
//! the same address space.

use std::sync::Arc;
use tokio::sync::mpsc;
use wasm_bindgen::prelude::*;

// ── Message type ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct Message {
    pub seq: u32,
    pub text: Arc<str>,
}

// ── Handles supplied by JS ────────────────────────────────────────────────────

static mut WASM_MODULE: Option<JsValue> = None;
static mut WASM_MEMORY: Option<JsValue> = None;
static mut WORKER_URL: Option<String> = None;
static mut GLUE_URL: Option<String> = None;

#[wasm_bindgen]
pub fn set_wasm_handles(module: JsValue, memory: JsValue, worker_url: String, glue_url: String) {
    unsafe {
        WASM_MODULE = Some(module);
        WASM_MEMORY = Some(memory);
        WORKER_URL = Some(worker_url);
        GLUE_URL = Some(glue_url);
    }
}

fn get_wasm_handles() -> (JsValue, JsValue, String, String) {
    unsafe {
        let m = std::ptr::addr_of!(WASM_MODULE)
            .read()
            .expect("set_wasm_handles not called");
        let mem = std::ptr::addr_of!(WASM_MEMORY)
            .read()
            .expect("set_wasm_handles not called");
        let w_url = std::ptr::addr_of!(WORKER_URL)
            .read()
            .expect("set_wasm_handles not called");
        let g_url = std::ptr::addr_of!(GLUE_URL)
            .read()
            .expect("set_wasm_handles not called");
        (m, mem, w_url, g_url)
    }
}

// ── Main-thread entry point ───────────────────────────────────────────────────

#[wasm_bindgen]
pub fn run() {
    // Create a bounded channel with capacity 8.
    let (tx, rx) = mpsc::channel::<Message>(8);

    // Send 3 messages timestamped with the current time via js_sys::Date.
    for i in 0..3 {
        let now = js_sys::Date::now(); // milliseconds since epoch
        let msg = Message {
            seq: i,
            text: format!("message {i} at t={:.0}ms", now).into(),
        };
        console_log!("▶ [main] sending: {:?}", msg);
        tx.try_send(msg).expect("channel full");
    }

    // Drop sender so the worker's receiver sees channel closed after draining.
    drop(tx);
    console_log!("▶ [main] sender dropped, channel closed");

    // Leak the receiver and pass its pointer to the worker.
    let rx_ptr = Box::into_raw(Box::new(rx)) as usize;
    console_log!("▶ [main] rx_ptr = 0x{:x}", rx_ptr);

    spawn_worker(rx_ptr);
}

// ── Worker spawner ────────────────────────────────────────────────────────────

fn spawn_worker(rx_ptr: usize) {
    use js_sys::{Object, Reflect};
    use web_sys::{Worker, WorkerOptions, WorkerType};

    let (module, memory, worker_url, glue_url) = get_wasm_handles();
    let mut opts = WorkerOptions::new();
    opts.set_type(WorkerType::Module);

    let worker = match Worker::new_with_options(&worker_url, &opts) {
        Ok(w) => w,
        Err(e) => {
            console_log!("✗ Worker::new failed: {:?}", e);
            return;
        }
    };

    let msg = Object::new();
    Reflect::set(&msg, &"module".into(), &module).unwrap();
    Reflect::set(&msg, &"memory".into(), &memory).unwrap();
    Reflect::set(&msg, &"ptr".into(), &JsValue::from_f64(rx_ptr as f64)).unwrap();
    Reflect::set(&msg, &"glue_url".into(), &JsValue::from_str(&glue_url)).unwrap();

    worker.post_message(&msg).expect("postMessage failed");
    console_log!("▶ [main] worker spawned with rx_ptr");
}

// ── Worker entry point ────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn worker_entry(rx_ptr: usize) {
    // Reconstruct the Receiver from the pointer.
    // SAFETY: pointer came from Box::into_raw on the main thread.
    let mut rx = unsafe { *Box::from_raw(rx_ptr as *mut mpsc::Receiver<Message>) };

    console_log!("● [worker] receiver reconstructed, draining channel...");

    loop {
        match rx.try_recv() {
            Ok(msg) => console_log!("● [worker] recv: {:?}", msg),
            Err(mpsc::error::TryRecvError::Empty) => {
                console_log!("● [worker] channel empty (unexpected — sender dropped before us)");
                break;
            }
            Err(mpsc::error::TryRecvError::Disconnected) => {
                console_log!("● [worker] channel drained and closed ✓");
                break;
            }
        }
    }
}

// ── console_log! ──────────────────────────────────────────────────────────────

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => { web_sys::console::log_1(&format!($($t)*).into()) }
}
