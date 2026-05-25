#!/usr/bin/env bash
set -euo pipefail

cargo +nightly build --target wasm32-unknown-unknown --release

WASM=target/wasm32-unknown-unknown/release/worxide.wasm
PATCHED=/tmp/worxide.wasm
cp "$WASM" "$PATCHED"

# 1. Strip shared memory flag so wasm-bindgen skips threading transform.
OFFSET=$(perl -e '
  local $/;
  open(F, "<:raw", "'"$WASM"'") or die $!;
  my $b = <F>;
  close F;
  if ($b =~ /\x05.\x01\x03/g) { printf "%d\n", pos($b) - 1; }
')
[ -z "$OFFSET" ] && { echo "error: shared flag not found" >&2; exit 1; }
printf '\x01' | dd of="$PATCHED" bs=1 seek="$OFFSET" count=1 conv=notrunc 2>/dev/null
echo "▶ patched shared flag at offset $OFFSET"

wasm-bindgen "$PATCHED" --out-dir site --target web --no-typescript

# 2. Convert internal memory definition to an import in worxide_bg.wasm.
echo "▶ converting memory to import in worxide_bg.wasm..."
wasm2wat site/worxide_bg.wasm -o /tmp/worxide_bg.wat --enable-threads
perl -i -0pe '
  my ($init, $max) = (17, 1024);
  s{\(memory \(;0;\) (\d+) (\d+)\)}{($init, $max) = ($1, $2); ""}e;
  s{\s*\(export "memory" \(memory 0\)\)}{}g;
  s{\(module[^\n]*\n}{"(module\n  (import \"env\" \"memory\" (memory " . $init . " " . $max . " shared))\n"}e;
' /tmp/worxide_bg.wat
wat2wasm /tmp/worxide_bg.wat -o site/worxide_bg.wasm --enable-threads
echo "▶ worxide_bg.wasm: memory is now an imported shared memory"

# 3. Patch worxide.js for SharedArrayBuffer compatibility.
perl -i -0pe '
  # Export shared memory and inject into wasm imports.
  s{^}{export const __wbg_shared_memory = new WebAssembly.Memory({ initial: 256, maximum: 1024, shared: true });\n};
  s{(const imports = __wbg_get_imports\(\);)}{$1\n    if (!imports.env) imports.env = {}; imports.env.memory = __wbg_shared_memory;}g;
  # wasm.memory no longer exists; replace with __wbg_shared_memory.
  s/\bwasm\.memory\b/__wbg_shared_memory/g;
  # TextDecoder rejects SAB-backed views; copy bytes into plain Uint8Array first.
  s{return cachedTextDecoder\.decode\(getUint8ArrayMemory0\(\)\.subarray\(ptr, ptr \+ len\)\);}
   {const _src = getUint8ArrayMemory0().subarray(ptr, ptr + len); const _dst = new Uint8Array(len); _dst.set(_src); return cachedTextDecoder.decode(_dst);}g;
  # NOTE: __wbindgen_start runs on ALL threads — it initialises the externref
  # table (needed for JS interop) and copies static data segments (idempotent).
  # Do NOT guard it; workers need it for console_log! and other JS calls.
' site/worxide.js

echo "✓ build complete → site/"