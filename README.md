# worxide

*This library was written with the help of Claude, it has been reviewed by me and the example, README and lib.rs have been heavily re-written by me.*

[![crates.io](https://img.shields.io/crates/v/worxide.svg)](https://crates.io/crates/worxide)
[![docs.rs](https://docs.rs/worxide/badge.svg)](https://docs.rs/worxide)

<p align="center">
  <img src="https://github.com/Billy-Sheppard/worxide/raw/main/docs/worxide.png" alt="icon" width="500">
</p>

Spawn a Rust function on a Web Worker and `await` its result - passing **any `T`** in and out with **no serialization, no deserialization, and no copying/cloning**.

The API is just two macros, `spawn!` and `spawn_blocking!`. Both return `anyhow::Result<R>` where `R` is the function's inferred return type.

## Why use `worxide`? - Move any `T` across threads for the cost of a pointer

Other ways of getting work onto a Web Worker make you pay some sort of transfer cost on your data - 
`postMessage` based approaches (gloo-worker, wasm-mt) **serializes** your data to bytes, clones it across the worker boundary, and **deserializes** it on the other side, and then again on the way back.
The bigger and more deeply-structured your `T`, the more that costs, and puts a trait bound on your data and result.

`worxide` doesn't move data across the boundary at all. It moves the *pointer* to the data.

Every worker shares one `SharedArrayBuffer`-backed memory slab, your value already lives somewhere both threads can see. `worxide` boxes it, hands the worker the resulting pointer, and the worker reads the value back in place. 
A `Vec<HashMap<String, Vec<Record>>>` crosses threads exactly as fast as a `u8` does - because in both cases the only thing that actually crosses is one integer.

## How it works

`worxide` compiles your crate to `wasm32-unknown-unknown` with shared linear memory. When you `spawn!` a function:

1. The data is boxed, its pointer is handed to a freshly created Web Worker.
2. The worker instantiates the same module in the *same* shared memory, reads the boxed data back through the pointer, and runs your function.
3. The result is boxed in shared memory; its pointer is posted back to the caller, which un-boxes it into a typed `R`.

The only values that ever cross the `postMessage` boundary are pointers. Your arguments and results stay put in shared memory the entire time - no `JsValue` round-tripping, no encoding, no copies.


### Pros

- **Tiny API.** Two macros.
- **Fully Typed.** Arguments and return values are ordinary Rust values, not `JsValue`.
- **Zero-copy hand-off.** Data passes by pointer through shared memory rather than being serialized through `postMessage`.
- **Sync and async.** `spawn_blocking!` for CPU-bound functions, `spawn!` for `async fn`s - the worker drives the future to completion itself.
- **No build-time worker path.** The worker bootstrap is embedded in the crate and served from a Blob URL.

### Cons

- **One worker per call (oneshot style).** `worxide` spawns and terminates a worker for each `spawn!`.
- **Cross-origin isolation required.** Shared memory needs `SharedArrayBuffer`, which needs the page to be cross-origin isolated (COOP + COEP headers). This is a hard browser requirement, not a `worxide` choice.
- **Nightly Rust + `build-std`.** WASM threads aren't stable yet, so the nightly toolchain is required. (Same constraint as other wasm-threading crates)
- **`--target web` only.** Requires `WebAssembly.Module`/`Memory` access and JS snippets, which are only available on wasm-bindgen's `web` target.
- **Arguments must be `'static`.** The data is moved into the worker, captured data must be owned.

## Usage

`worxide` needs nightly Rust with the `wasm32-unknown-unknown` target and the `rust-src` component (for `build-std`). The easiest way is a `rust-toolchain.toml` in your project root, which makes `rustup` fetch and usethe right toolchain automatically.

```toml
[toolchain]
channel = "nightly"
targets = ["wasm32-unknown-unknown"]
components = ["rust-src"]
```

```toml
[dependencies]
worxide = "0.0.1"
```

```toml
[target.wasm32-unknown-unknown]
rustflags = [
    "-C", "target-feature=+atomics,+bulk-memory,+mutable-globals",
    "-C", "link-arg=--shared-memory",
    "-C", "link-arg=--max-memory=1073741824",
    "-C", "link-arg=--import-memory",
    "-C", "link-arg=--export=__wasm_init_tls",
    "-C", "link-arg=--export=__tls_size",
    "-C", "link-arg=--export=__tls_align",
    "-C", "link-arg=--export=__tls_base",
    "-C", "link-arg=--export=__heap_base",
    "-C", "link-arg=--export=__data_end",
]

[unstable]
build-std = ["panic_abort", "std"]
```

- `+atomics,+bulk-memory,+mutable-globals` + `--shared-memory` - enable WASM threads and make the linear memory shareable.
- `--import-memory` - the memory becomes an *import* so the main thread can create it once and every worker can attach to it
- `--max-memory=1073741824` - shared memory must declare a maximum (e.g. 1 GiB - tune to your needs).
- `--export=__wasm_init_tls / __tls_size / __tls_align / __tls_base` - expose the thread-local-storage machinery so each worker gets its own TLS block
- `--export=__heap_base / __data_end` - wasm-bindgen's threading transform needs these to inject per-thread state; lld only emits them when asked.

These flags are similar to what's used by `wasm-bindgen-rayon` and `wasm-bindgen-spawn`.

#### Cross-origin isolation (COOP + COEP)

`SharedArrayBuffer` is only available when the page is **cross-origin
isolated**, which requires two response headers:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
``` 

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8"/>
  <title>your app</title>
  <script>
    // If you can't set COOP/COEP headers from your webserver you can use the following to allow for `SharedArrayBuffer` use
    // Register the COOP/COEP service worker, then reload so the page loads cross-origin isolated. Skips straight through once isolation is active.
    (async () => {
      if (self.crossOriginIsolated) return;
      if (!('serviceWorker' in navigator)) return;
      await navigator.serviceWorker.register('./sw.js', { scope: './' }); // there is a `sw.js` provided in this repo at `docs/sw.js`
      await navigator.serviceWorker.ready;
      location.reload();
    })();
  </script>
</head>
<body>
</body>
<script type="module">
  // If you need to override the calculated path for your apps javascript do so like this
  globalThis.app_js_path = './your_app.js';
  
  import init, { run_app } from './your_app.js';
  if (self.crossOriginIsolated) {

    await init();
    await run_app();
  }
</script>
</html>
```

Then call the macros from main-thread code:

```rust
use wasm_bindgen::prelude::*;

fn primes_below(n: u64) -> u64 {
    (2..n).filter(|&k| (2..k).all(|d| k % d != 0)).count() as u64
}

async fn request_data(url: &str) -> MyData {
  ...
}

#[wasm_bindgen]
pub fn run() {
    // run sync code on a worker
    wasm_bindgen_futures::spawn_local(async {
        match worxide::spawn_blocking!(primes_below, 100_000).await {
            Ok(count) => web_sys::console::log_1(&format!("{count} primes").into()),
            Err(e)    => web_sys::console::error_1(&format!("{e}").into()),
        }
    });

    // run async code on a worker
    wasm_bindgen_futures::spawn_local(async {
        match worxide::spawn!(request_data, "https://example.com").await {
          ..
        }
    });
}
```

#### Example Build Script:
```sh
#!/usr/bin/env bash

cargo +nightly build --target wasm32-unknown-unknown --release -p worxide-example

wasm-bindgen target/wasm32-unknown-unknown/release/worxide_example.wasm --out-dir ./worxide-example/site --target web --no-typescript
```

### Detecting the worker thread: `is_worker()`

```rust
fn foo() {
  if worxide::is_worker() {
    ...
  }
  else {
    ...
  }
}
```

There is an exported function offered if you wish to detect whether your code is running on a worker or not.

## Requirements

- Nightly Rust with `rust-src`
- `wasm-bindgen-cli` (matching your `wasm-bindgen` dependency version)
- `--target web`
- A cross-origin-isolated page (COOP + COEP, via server headers or the service worker workaround above)

## License

MIT