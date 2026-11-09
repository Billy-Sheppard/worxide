use {
    crate::private::WORKER_JS,
    anyhow::{Result, anyhow},
    js_sys::{Promise, Reflect},
    wasm_bindgen::{closure::Closure, prelude::*},
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        Blob, BlobPropertyBag, MessageChannel, MessageEvent, MessagePort, Url, Worker as WebWorker, WorkerOptions,
        WorkerType,
    },
};

#[wasm_bindgen(inline_js = r#"
    export function worxide_isolated_ready_timeout(promise) {
        let timer;
        const timeout = new Promise((_, reject) => {
            timer = setTimeout(() => {
                reject(new Error(
                    "worxide: isolated worker initialization timed out"
                ));
            }, 30000);
        });

        return Promise.race([promise, timeout])
            .finally(() => clearTimeout(timer));
    }
"#)]
extern "C" {
    fn worxide_isolated_ready_timeout(promise: &Promise) -> Promise;
}

/// A persistent worker with memory independent of the caller.
///
/// This bootstrap-only handle does not accept shared-memory closures.
/// Registered isolated jobs will be added separately.
///
/// Dropping the handle terminates the worker.
pub struct IsolatedWorker {
    inner: WebWorker,
    port: MessagePort,
    initial_memory_bytes: u64,
    maximum_memory_bytes: Option<u64>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(JsValue)>,
}

impl IsolatedWorker {
    pub(crate) async fn boot(glue_url: String, maximum_bytes: Option<u64>) -> Result<Self> {
        let channel = MessageChannel::new().map_err(|error| js_error("MessageChannel construction", error))?;

        let port = channel.port1();
        let worker_port = channel.port2();

        let blob_url = bootstrap_url()?;

        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);

        let constructed = WebWorker::new_with_options(&blob_url, &options);

        Url::revoke_object_url(&blob_url).ok();

        let inner = constructed.map_err(|error| js_error("Worker construction", error))?;

        let mut resolve_slot = None;
        let mut reject_slot = None;

        let ready = Promise::new(&mut |resolve, reject| {
            resolve_slot = Some(resolve);
            reject_slot = Some(reject);
        });

        let resolve = resolve_slot.ok_or_else(|| anyhow!("Promise did not provide resolve"))?;

        let reject = reject_slot.ok_or_else(|| anyhow!("Promise did not provide reject"))?;

        let message_reject = reject.clone();

        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let data = event.data();

            let kind = Reflect::get(&data, &JsValue::from_str("type")).ok().and_then(|value| value.as_string());

            match kind.as_deref() {
                Some("ready") => {
                    resolve.call1(&JsValue::NULL, &data).ok();
                }
                Some("init_error") => {
                    let error = Reflect::get(&data, &JsValue::from_str("error"))
                        .unwrap_or_else(|_| JsValue::from_str("Isolated worker initialization failed"));

                    message_reject.call1(&JsValue::NULL, &error).ok();
                }
                _ => {}
            }
        });

        let on_error = Closure::<dyn FnMut(JsValue)>::new(move |error: JsValue| {
            reject.call1(&JsValue::NULL, &error).ok();
        });

        let mut handle = Self {
            inner,
            port,
            initial_memory_bytes: 0,
            maximum_memory_bytes: None,
            _on_message: on_message,
            _on_error: on_error,
        };

        handle.port.set_onmessage(Some(handle._on_message.as_ref().unchecked_ref()));

        handle.inner.set_onerror(Some(handle._on_error.as_ref().unchecked_ref()));

        let message = js_sys::Object::new();

        set(&message, "type", &JsValue::from_str("init_isolated"))?;
        set(&message, "module", &wasm_bindgen::module())?;
        set(&message, "glue_url", &JsValue::from_str(&glue_url))?;

        if let Some(bytes) = maximum_bytes {
            set(&message, "maximum_bytes", &JsValue::from_f64(bytes as f64))?;
        }

        let transfer = js_sys::Array::of1(&worker_port);

        handle
            .inner
            .post_message_with_transfer(&message, &transfer)
            .map_err(|error| js_error("postMessage(init_isolated)", error))?;

        let reply = JsFuture::from(worxide_isolated_ready_timeout(&ready))
            .await
            .map_err(|error| js_error("Isolated initialization", error))?;

        handle.initial_memory_bytes =
            number_field(&reply, "initial_bytes").ok_or_else(|| anyhow!("Missing initial memory size"))?;

        handle.maximum_memory_bytes = number_field(&reply, "maximum_bytes");

        Ok(handle)
    }

    /// Memory size observed immediately after initialization.
    ///
    /// This is a snapshot, not live telemetry.
    pub fn initial_memory_bytes(&self) -> u64 { self.initial_memory_bytes }

    /// Configured maximum, when captured from the generated descriptor.
    ///
    /// None means no maximum was observed, not unlimited available RAM.
    pub fn maximum_memory_bytes(&self) -> Option<u64> { self.maximum_memory_bytes }

    /// Consumer messaging uses the worker's default channel.
    ///
    /// Worxide reserves its separate private control port.
    pub fn worker_handle(&self) -> &WebWorker { &self.inner }

    /// Terminate immediately. Also called on drop.
    pub fn terminate(&self) {
        self.port.set_onmessage(None);
        self.port.close();
        self.inner.set_onerror(None);
        self.inner.terminate();
    }
}

impl Drop for IsolatedWorker {
    fn drop(&mut self) { self.terminate(); }
}

fn bootstrap_url() -> Result<String> {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(WORKER_JS));

    let options = BlobPropertyBag::new();
    options.set_type("application/javascript");

    let blob =
        Blob::new_with_str_sequence_and_options(&parts, &options).map_err(|error| js_error("Blob construction", error))?;

    Url::create_object_url_with_blob(&blob).map_err(|error| js_error("Blob URL construction", error))
}

fn set(object: &JsValue, key: &str, value: &JsValue) -> Result<()> {
    Reflect::set(object, &JsValue::from_str(key), value).map_err(|error| js_error("Setting initialization field", error))?;

    Ok(())
}

fn number_field(object: &JsValue, key: &str) -> Option<u64> {
    let number = Reflect::get(object, &JsValue::from_str(key)).ok()?.as_f64()?;

    if !number.is_finite() || number < 0.0 {
        return None;
    }

    Some(number as u64)
}

fn js_error(context: &str, value: JsValue) -> anyhow::Error {
    let message = value
        .as_string()
        .or_else(|| Reflect::get(&value, &JsValue::from_str("message")).ok().and_then(|message| message.as_string()));

    anyhow!("{context}: {}", message.unwrap_or_else(|| format!("{value:?}")))
}
