#!/usr/bin/env bash
set -euo pipefail

CRATE=worxide_example
PKG=worxide-example
WASM=target/wasm32-unknown-unknown/release/${CRATE}.wasm
OUT_DIR=${PKG}/site

# Build with threading flags (see .cargo/config.toml). lld emits a binary with
# imported shared memory and the TLS exports wasm-bindgen needs — so no post
# build patching is required.
cargo +nightly build --target wasm32-unknown-unknown --release -p "$PKG"

# wasm-bindgen runs its threading transform directly on the binary. With the
# TLS exports + imported memory present, this succeeds and emits proper
# per-thread init code (initSync + a thread entry that calls __wasm_init_tls).
wasm-bindgen "$WASM" --out-dir "$OUT_DIR" --target web --no-typescript

# Only remaining glue patch: TextDecoder can't decode a SharedArrayBuffer view
# directly, so copy bytes to a plain Uint8Array first.
perl -i -0pe '
  s{return cachedTextDecoder\.decode\(getUint8ArrayMemory0\(\)\.subarray\(ptr, ptr \+ len\)\);}
   {const _src = getUint8ArrayMemory0().subarray(ptr, ptr + len); const _dst = new Uint8Array(len); _dst.set(_src); return cachedTextDecoder.decode(_dst);}g;
' "$OUT_DIR/${CRATE}.js"

echo "✓ build complete → $OUT_DIR/"