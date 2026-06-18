// worxide worker bootstrap.
//
// Embedded into the crate via include_str! and served from a Blob URL at
// runtime. This code necessarily lives in JavaScript: it runs *before* the
// wasm module is initialized on this thread, and its whole job is to bootstrap
// wasm into existence — you can't write that in the thing it's booting.
// (For comparison, wasm-bindgen-rayon and the official wasm-bindgen threading
// examples likewise keep their worker bootstrap + dispatch in JS.)
//
// Two messaging shapes share this worker:
//
//   * Persistent handle (worxide::Worker) — typed frames:
//       main -> worker  { type: "init",  module, memory, glue_url }
//       worker -> main  { type: "ready" }
//       main -> worker  { type: "call",  id, kind, ptr }
//       worker -> main  { type: "result", id, result }   // success
//       worker -> main  { type: "result", id, error }    // task threw
//     init runs initSync ONCE; later `call` frames reuse the same instance.
//
//   * One-shot macros (spawn! / spawn_blocking!) — the untyped message { kind, module, memory, ptr, glue_url }  ->  postMessage(result_ptr)
//     It carries no `type`, so it falls through to the original path below and
//     is unchanged.
//
// The build flags (imported shared memory + exported __wasm_init_tls / __tls_*
// / __heap_base) let wasm-bindgen's threading transform handle per-thread
// initialization: initSync({ module, memory }) attaches this worker to the
// shared memory and allocates this thread's own TLS block.

// Surface worker-side errors in the page console for diagnostics. Async
// rejections reach the unhandledrejection listener on their own.
self.addEventListener('error', e => console.error('[worxide:worker]', e.message));
self.addEventListener('unhandledrejection', e => console.error('[worxide:worker]', e.reason));

self.addEventListener('message', async (ev) => {
    const msg = ev.data;

    // --- persistent handle: one-time init -------------------------------
    if (msg && msg.type === 'init') {
        const glue = await import(msg.glue_url);
        // Attach to the main thread's shared memory and initialize this worker
        // thread. wasm-bindgen's generated initSync handles TLS allocation and
        // per-thread __wbindgen_start for this instance. Done ONCE.
        glue.initSync({ module: msg.module, memory: msg.memory });
        // Plant the resolved glue URL so any nested spawn this worker performs
        // reuses it instead of re-deriving from a crate name / a global it
        // can't see.
        glue.__worxide_seed_glue_url(msg.glue_url);
        // Keep the initialized glue for subsequent `call` frames.
        self.__worxide_glue = glue;
        self.postMessage({ type: 'ready' });
        return;
    }

    // --- persistent handle: per-call dispatch ---------------------------
    if (msg && msg.type === 'call') {
        const glue = self.__worxide_glue;
        const { id, kind, ptr } = msg;
        try {
            // A Rust panic (panic=abort) traps here as a RuntimeError; catching
            // it rejects this one call's future and leaves the worker alive for
            // later calls.
            const result = kind === 'sync'
                ? glue.__worxide_worker_entry(ptr)
                : await glue.__worxide_worker_entry_async(ptr);
            self.postMessage({ type: 'result', id, result });
        } catch (e) {
            self.postMessage({ type: 'result', id, error: String((e && e.stack) || e) });
        }
        return;
    }

    // --- one-shot path (spawn! / spawn_blocking!) ----------------
    // Only genuine frames: no typed discriminator, but carrying the
    // one-shot payload (a glue_url to import). Anything else on this channel
    // belongs to a consumer sharing the worker via raw() (e.g. canvas/mouse
    // side-channels) — not ours, so ignore it rather than mis-handling it.
    if (msg && msg.type === undefined && msg.glue_url !== undefined) {
        const { kind, module, memory, ptr, glue_url } = msg;
        const glue = await import(glue_url);
        glue.initSync({ module, memory });
        glue.__worxide_seed_glue_url(glue_url);
        const result_ptr = kind === 'sync'
            ? glue.__worxide_worker_entry(ptr)
            : await glue.__worxide_worker_entry_async(ptr);
        self.postMessage(result_ptr);
    }
    // else: a frame we don't own — ignore.
});