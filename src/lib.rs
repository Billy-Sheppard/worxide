use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Mutex, OnceLock},
};

use js_sys::{Array, Object, Promise, Reflect, SharedArrayBuffer, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

pub use worxide_macros::worker_fn;

#[cfg(feature = "example")]
mod example;

pub type ErasedFn = fn(Vec<u8>) -> Pin<Box<dyn Future<Output = Vec<u8>>>>;

static REGISTRY: OnceLock<Mutex<HashMap<&'static str, ErasedFn>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<&'static str, ErasedFn>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a function by name. Called by the `__register_*` shims that
/// `#[worker_fn]` generates — do not call directly.
pub fn register(name: &'static str, f: ErasedFn) {
    registry().lock().unwrap().entry(name).or_insert(f);
}

struct State {
    module: JsValue,
    glue_url: String,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

static STATE: OnceLock<State> = OnceLock::new();

/// Builder for initialising worxide.
///
/// ```rust
/// worxide::Config::init().await;                         // auto-derives glue URL
/// worxide::Config::init().js_file("./custom.js").await;  // override
/// ```
pub struct Config {
    js_file: Option<String>,
}

impl Config {
    pub fn init() -> Self {
        Self { js_file: None }
    }

    /// Override the wasm-bindgen glue JS filename.
    /// Defaults to `"{crate_name}.js"` resolved relative to `location.href`.
    pub fn js_file(mut self, name: &str) -> Self {
        self.js_file = Some(name.to_string());
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::init()
    }
}

impl std::future::IntoFuture for Config {
    type Output = ();
    type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            // On the worker, glue_url comes via STATE set by setup_worker_listener.
            // On the main thread, derive it from the crate name / override.
            if is_worker() {
                // Worker path: STATE.glue_url was set when the task message arrived.
                // Just register shims and set up the listener.
                if let Some(state) = STATE.get() {
                    let url = state.glue_url.clone();
                    call_register_shims(&url).await;
                    setup_worker_listener();
                }
                return;
            }
            let glue_url = resolve_glue_url(self.js_file.as_deref());
            init_inner(glue_url).await;
        })
    }
}

fn resolve_glue_url(override_name: Option<&str>) -> String {
    let file_name = override_name.unwrap_or(concat!(env!("CARGO_PKG_NAME"), ".js"));
    resolve_url_relative_to_location(file_name)
}

async fn init_inner(glue_url: String) {
    call_register_shims(&glue_url).await;

    if is_worker() {
        setup_worker_listener();
        return;
    }

    STATE
        .set(State {
            module: wasm_bindgen::module(),
            glue_url,
        })
        .ok();
}

/// Spawn `f(input)` on a dedicated Web Worker and await the result.
/// Call via the `spawn!` macro.
pub async fn spawn_erased(fn_name: &'static str, input_bytes: Vec<u8>) -> Vec<u8> {
    let state = STATE.get().expect("call init() first");

    let input_sab = alloc_sab(input_bytes.len());
    write_bytes(&input_sab, &input_bytes);

    let worker = spawn_blob_worker();

    let msg = Object::new();
    Reflect::set(&msg, &"fn_name".into(), &JsValue::from_str(fn_name)).unwrap();
    Reflect::set(&msg, &"input_sab".into(), &input_sab).unwrap();
    Reflect::set(&msg, &"module".into(), &state.module).unwrap();
    Reflect::set(
        &msg,
        &"glue_url".into(),
        &JsValue::from_str(&state.glue_url),
    )
    .unwrap();

    worker.post_message(&msg).unwrap();

    let output_sab: SharedArrayBuffer = await_worker_result(&worker)
        .await
        .unwrap()
        .dyn_into()
        .unwrap();

    read_bytes(&output_sab)
}

pub fn is_worker() -> bool {
    js_sys::global()
        .dyn_into::<web_sys::WorkerGlobalScope>()
        .is_ok()
}

#[macro_export]
macro_rules! spawn {
    ($f:ident, $input:expr) => {{
        let bytes = serde_json::to_vec(&$input).unwrap();
        async move {
            let out = $crate::spawn_erased(stringify!($f), bytes).await;
            #[allow(unreachable_code)]
            if false {
                return $f($input).await;
            }
            serde_json::from_slice(&out).unwrap()
        }
    }};
}

/// Called by the blob worker JS after importing the glue module.
/// Bypasses Config so we don't need location.href on the worker.
#[wasm_bindgen]
pub async fn worker_init(glue_url: String) {
    call_register_shims(&glue_url).await;
    STATE
        .set(State {
            module: wasm_bindgen::module(),
            glue_url,
        })
        .ok();
    setup_worker_listener();
}

fn setup_worker_listener() {
    let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        let data = event.data();

        let fn_name: String = Reflect::get(&data, &"fn_name".into())
            .unwrap()
            .as_string()
            .unwrap();
        let input_sab: SharedArrayBuffer = Reflect::get(&data, &"input_sab".into())
            .unwrap()
            .dyn_into()
            .unwrap();
        let f = *registry()
            .lock()
            .unwrap()
            .get(fn_name.as_str())
            .unwrap_or_else(|| panic!("no worker fn registered as '{fn_name}'"));

        let input_bytes = read_bytes(&input_sab);
        drop(input_sab);

        let fut = f(input_bytes);

        wasm_bindgen_futures::spawn_local(async move {
            let output_bytes = fut.await;

            let output_sab = alloc_sab(output_bytes.len());
            write_bytes(&output_sab, &output_bytes);

            let reply = Object::new();
            Reflect::set(&reply, &"output_sab".into(), &output_sab).unwrap();

            let global: web_sys::DedicatedWorkerGlobalScope = js_sys::global().dyn_into().unwrap();
            global.post_message(&reply).unwrap();
        });
    }) as Box<dyn FnMut(web_sys::MessageEvent)>);

    let global = js_sys::global();
    Reflect::set(
        &global,
        &"onmessage".into(),
        onmessage.as_ref().unchecked_ref(),
    )
    .unwrap();
    onmessage.forget();
}

fn spawn_blob_worker() -> web_sys::Worker {
    let script = r#"
self.onmessage = async (event) => {
    const { module, glue_url, fn_name, input_sab } = event.data;

    const glue = await import(glue_url);
    // initSync sets up the wasm instance without triggering #[wasm_bindgen(start)]
    glue.initSync({ module });
    // worker_init registers shims and sets up the message listener
    await glue.worker_init(glue_url);

    self.dispatchEvent(new MessageEvent('message', {
        data: { fn_name, input_sab }
    }));
};
"#;

    let parts = Array::new();
    parts.push(&JsValue::from_str(script));

    let bag = web_sys::BlobPropertyBag::new();
    bag.set_type("text/javascript");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &bag).unwrap();
    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

    let opts = web_sys::WorkerOptions::new();
    opts.set_type(web_sys::WorkerType::Module);

    web_sys::Worker::new_with_options(&url, &opts).unwrap()
}

fn await_worker_result(worker: &web_sys::Worker) -> JsFuture {
    let worker = worker.clone();

    let promise = Promise::new(&mut |resolve, _reject| {
        let worker2 = worker.clone();
        let onmessage = Closure::once(move |event: web_sys::MessageEvent| {
            let sab = Reflect::get(&event.data(), &"output_sab".into()).unwrap();
            resolve.call1(&JsValue::UNDEFINED, &sab).unwrap();
            worker2.terminate();
        });

        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
    });

    JsFuture::from(promise)
}

// Layout: [0..4] u32 payload_len | [4..] payload bytes

fn alloc_sab(payload_len: usize) -> SharedArrayBuffer {
    create_shared_buffer((4 + payload_len) as u32)
}

fn write_bytes(sab: &SharedArrayBuffer, bytes: &[u8]) {
    let len = bytes.len() as u32;
    let view = Uint8Array::new(sab);
    view.set_index(0, (len & 0xFF) as u8);
    view.set_index(1, ((len >> 8) & 0xFF) as u8);
    view.set_index(2, ((len >> 16) & 0xFF) as u8);
    view.set_index(3, ((len >> 24) & 0xFF) as u8);
    view.set(&Uint8Array::from(bytes), 4);
}

fn read_bytes(sab: &SharedArrayBuffer) -> Vec<u8> {
    let view = Uint8Array::new(sab);
    let len = (view.get_index(0) as u32)
        | ((view.get_index(1) as u32) << 8)
        | ((view.get_index(2) as u32) << 16)
        | ((view.get_index(3) as u32) << 24);
    let mut out = vec![0u8; len as usize];
    view.slice(4, 4 + len).copy_to(&mut out);
    out
}

#[wasm_bindgen(inline_js = r#"
export function create_shared_buffer(size) {
    return new SharedArrayBuffer(size);
}

export async function call_register_shims_js(module, glue_url) {
    const glue = await import(glue_url);
    const exports = WebAssembly.Module.exports(module);
    for (const { name } of exports) {
        if (name.startsWith("__register_") && typeof glue[name] === "function") {
            glue[name]();
        }
    }
}
"#)]
extern "C" {
    fn create_shared_buffer(size: u32) -> SharedArrayBuffer;
    fn call_register_shims_js(module: &JsValue, glue_url: &str) -> js_sys::Promise;
}

#[wasm_bindgen(inline_js = r#"
export function resolve_url_relative_to_location(file_name) {
    const base = (typeof self !== "undefined" && self.location)
        ? self.location.href
        : location.href;
    return new URL(file_name, base).href;
}
"#)]
extern "C" {
    fn resolve_url_relative_to_location(file_name: &str) -> String;
}

async fn call_register_shims(glue_url: &str) {
    let module = wasm_bindgen::module();
    JsFuture::from(call_register_shims_js(&module, glue_url))
        .await
        .unwrap();
}
