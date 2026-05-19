#! /bin/bash

cargo build --target wasm32-unknown-unknown --release --features example
wasm-bindgen target/wasm32-unknown-unknown/release/worxide.wasm --out-dir site --target web --no-typescript