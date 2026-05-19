use std::{collections::HashMap, future::Future, pin::Pin, sync::OnceLock};

use js_sys::{Array, Object, Promise, Reflect, SharedArrayBuffer, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[cfg(feature = "example")]
mod example;

pub type ErasedFn = fn(Vec<u8>) -> Pin<Box<dyn Future<Output = Vec<u8>>>>;

// linkme requires an extern static with no body for distributed_slice.
#[linkme::distributed_slice]
pub static REGISTRY_ENTRIES: [(&str, ErasedFn)];

static REGISTRY: OnceLock<HashMap<&'static str, ErasedFn>> = OnceLock::new();

fn registry() -> &'static HashMap<&'static str, ErasedFn> {
    REGISTRY.get().expect("call init() first")
}

struct State {
    module: JsValue,
    memory: JsValue,
    glue_url: String,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

static STATE: OnceLock<State> = OnceLock::new();

/// Call once at startup (inside `#[wasm_bindgen(start)]`).
pub fn init() {
    REGISTRY
        .set(REGISTRY_ENTRIES.iter().copied().collect())
        .ok();

    if is_worker() {
        setup_worker_listener();
        return;
    }

    STATE
        .set(State {
            module: wasm_bindgen::module(),
            memory: wasm_bindgen::memory(),
            glue_url: get_script_url(),
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
    Reflect::set(&msg, &"memory".into(), &state.memory).unwrap();
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

#[macro_export]
macro_rules! spawn {
    ($f:ident, $input:expr) => {{
        #[linkme::distributed_slice($crate::REGISTRY_ENTRIES)]
        static _ENTRY: (&str, $crate::ErasedFn) = (stringify!($f), |bytes| {
            Box::pin(async move {
                let input = serde_json::from_slice(&bytes).unwrap();
                let output = $f(input).await;
                serde_json::to_vec(&output).unwrap()
            })
        });

        let bytes = serde_json::to_vec(&$input).unwrap();
        async move {
            type Output = dyn Sized;
            let out = $crate::spawn_erased(stringify!($f), bytes).await;
            if false {
                return $f($input).await;
            } // forces U inference
            serde_json::from_slice(&out).unwrap()
        }
    }};
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
    const { module, memory, glue_url, fn_name, input_sab } = event.data;

    const glue = await import(glue_url);
    glue.initSync({ module, memory });

    self.dispatchEvent(new MessageEvent('message', {
        data: { fn_name, input_sab }
    }));
};
"#;

    let parts = Array::new();
    parts.push(&JsValue::from_str(script));

    let blob = web_sys::Blob::new_with_str_sequence(&parts).unwrap();
    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

    let opts = web_sys::WorkerOptions::new();
    opts.set_type(web_sys::WorkerType::Module);

    web_sys::Worker::new_with_options(&url, &opts).unwrap()
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

fn is_worker() -> bool {
    js_sys::global()
        .dyn_into::<web_sys::WorkerGlobalScope>()
        .is_ok()
}

#[wasm_bindgen(inline_js = r#"
export function create_shared_buffer(size) {
    return new SharedArrayBuffer(size);
}
export function get_script_url() {
    return import.meta.url;
}
"#)]
extern "C" {
    fn create_shared_buffer(size: u32) -> SharedArrayBuffer;
    fn get_script_url() -> String;
}
