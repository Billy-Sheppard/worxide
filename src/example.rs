use rand::RngExt;
use wasm_bindgen::prelude::wasm_bindgen;

async fn add_ten(n: u32) -> u32 {
    n + 10
}

#[wasm_bindgen(start)]
pub async fn main_js() {
    let mut rng = rand::rng();
    let rand = rng.random::<u32>();

    gloo::console::console_dbg!(format!("Generated `{}` on the main thread.", rand));

    let add_ten = crate::spawn!(add_ten, rand).await;

    gloo::console::console_dbg!(format!("Added 10 on a worker thread: `{}`", add_ten));
}
