// worxide worker bootstrap.
//
// Embedded into the crate via include_str! and served from a Blob URL at
// runtime. This code necessarily lives in JavaScript: it runs *before* the
// wasm module is initialized on this thread, and its whole job is to bootstrap
// wasm into existence — you can't write that in the thing it's booting.
// (For comparison, wasm-bindgen-rayon and the official wasm-bindgen threading
// examples likewise keep their worker bootstrap + dispatch in JS.)
//
// The build flags (imported shared memory + exported __wasm_init_tls / __tls_*
// / __heap_base) let wasm-bindgen's threading transform handle per-thread
// initialization: initSync({ module, memory }) attaches this worker to the
// shared memory and allocates this thread's own TLS block. We don't manage
// TLS, memory, or __wbindgen_start ordering by hand.
//
// `glue_url` is resolved on the main thread and passed in via postMessage. We
// re-seed it into this worker's own resolved-URL cache (see below) so that any
// *nested* spawn this worker performs reuses it instead of re-deriving from a
// crate name or a `globalThis.app_js_path` that only exists on the page.

// Surface worker-side errors in the page console for diagnostics. Async
// rejections reach the unhandledrejection listener on their own.
self.addEventListener('error', e => console.error('[worxide:worker]', e.message));
self.addEventListener('unhandledrejection', e => console.error('[worxide:worker]', e.reason));

self.onmessage = async ({ data: { kind, module, memory, ptr, glue_url } }) => {
    const glue = await import(glue_url);

    // Attach to the main thread's shared memory and initialize this worker
    // thread. wasm-bindgen's generated initSync handles TLS allocation and
    // per-thread __wbindgen_start for this instance.
    glue.initSync({ module, memory });

    // Plant the already-resolved glue URL in this worker's thread-local cache
    // BEFORE running the task. If the task makes nested spawns, they'll find
    // the cache populated and forward this same URL onward — workers can't see
    // the page's `globalThis.app_js_path`, so without this they'd fall back to
    // the (wrong) crate name. Must run after initSync and before the entry call.
    glue.__worxide_seed_glue_url(glue_url);

    const result_ptr = kind === 'sync'
        ? glue.__worxide_worker_entry(ptr)
        : await glue.__worxide_worker_entry_async(ptr);

    self.postMessage(result_ptr);
};