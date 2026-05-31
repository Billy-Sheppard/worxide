// worxide worker bootstrap.
// Embedded into the Rust crate via include_str! and served at runtime from a
// Blob URL. Receives the wasm module, shared memory, task ptr, kind, and glue
// URL from the main thread.
//
// With wasm-bindgen's threading transform (enabled by the rayon-style build
// flags: imported shared memory + exported __wasm_init_tls / __tls_* /
// __heap_base), initSync({ module, memory }) attaches this worker to the
// shared memory AND performs correct per-thread initialization, allocating
// this thread's own TLS block via the wasm's exported __wasm_init_tls. We no
// longer manage TLS, memory interception, or __wbindgen_start ordering by hand.

self.addEventListener('error', e => console.error('[worxide:worker]', e.message, e));
self.addEventListener('unhandledrejection', e => console.error('[worxide:worker] rejection:', e.reason));

self.onmessage = async ({ data: { kind, module, memory, ptr, glue_url } }) => {
    try {
        const glue = await import(glue_url);

        // Attach to the main thread's shared memory and initialize this
        // worker thread. wasm-bindgen's generated initSync handles TLS
        // allocation and per-thread __wbindgen_start for this instance.
        glue.initSync({ module, memory });

        let result_ptr;
        if (kind === 'sync') {
            result_ptr = glue.__worxide_worker_entry(ptr);
        } else {
            result_ptr = await glue.__worxide_worker_entry_async(ptr);
        }
        self.postMessage(result_ptr);
    } catch (e) {
        throw e;
    }
};