#!/usr/bin/env bash

cargo +nightly build --target wasm32-unknown-unknown --release -p worxide-example

wasm-bindgen target/wasm32-unknown-unknown/release/worxide_example.wasm --out-dir ./worxide-example/site --target web --no-typescript
