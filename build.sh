#!/usr/bin/env bash

rm -r docs/snippets

cargo +nightly build --target wasm32-unknown-unknown --release -p worxide-example
 
wasm-bindgen target/wasm32-unknown-unknown/release/worxide_example.wasm --out-dir ./docs --target web --no-typescript