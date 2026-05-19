use std::marker::PhantomData;

use js_sys::{SharedArrayBuffer, Uint8Array};
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

#[derive(Debug)]
pub struct SharedData<T: Serialize + DeserializeOwned> {
    buffer: SharedArrayBuffer,
    marker: PhantomData<T>,
}
impl<T: Serialize + DeserializeOwned> SharedData<T> {
    pub fn new(data: &T) -> anyhow::Result<Self> {
        let bytes = serde_json::to_vec(data)?;
        let size = bytes.len();
        let buffer = create_shared_buffer(size);

        let array = Uint8Array::new(&buffer);
        array.copy_from(&bytes);

        Ok(Self {
            buffer,
            marker: PhantomData,
        })
    }

    pub fn write(&self, data: &T) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec(data)?;
        let array = Uint8Array::new(&self.buffer);
        array.copy_from(&bytes);
        Ok(())
    }

    pub fn read(&self) -> Result<T, serde_json::Error> {
        let bytes = Uint8Array::new(&self.buffer).to_vec();
        serde_json::from_slice(&bytes)
    }
}

#[wasm_bindgen(inline_js = r#"
export function create_shared_buffer(size) {
    return new SharedArrayBuffer(size);
}
"#)]
extern "C" {
    fn create_shared_buffer(size: usize) -> SharedArrayBuffer;
}

#[cfg(feature = "example")]
pub mod example {
    use rand::RngExt;
    use std::{ops::Add, sync::OnceLock};
    use wasm_bindgen::{JsCast, prelude::wasm_bindgen};
    use web_sys::Worker;

    use crate::SharedData;

    static SHARED_DATA: OnceLock<SharedData<u32>> = OnceLock::new();

    fn is_worker() -> bool {
    js_sys::global()
        .dyn_into::<web_sys::WorkerGlobalScope>()
        .is_ok()
}

    #[wasm_bindgen(start)]
    pub fn start() {
        console_error_panic_hook::set_once();
        if is_worker() {
            worker_js();
        } else {
            main_js();
        }
    }

    pub fn worker_js() {
        let sd = SHARED_DATA.get().unwrap();
        let add_ten = sd.read().unwrap().add(10);
        sd.write(&add_ten).unwrap();
    }

    pub fn main_js() {
        let mut rng = rand::rng();
        let rand = rng.random::<u32>();

        gloo::console::console_dbg!(format!("Generated `{}` on the main thread.", rand));

        let handle = SharedData::new(&rand).unwrap();
        SHARED_DATA.set(handle).unwrap();

        let worker = Worker::new("./worxide.js").unwrap();
        

        let rand = SHARED_DATA.get().unwrap().read().unwrap();
        gloo::console::console_dbg!(format!("Added 10 on a worker thread: `{}`", rand));
    }
}
