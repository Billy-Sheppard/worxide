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
//   * Persistent handle (worxide::Worker) — typed envelopes. The handshake's
//     `init` rides the worker's default channel and *transfers a MessagePort*;
//     every later envelope runs over that private port:
//       main   -> worker (default) { type: "init", module, memory, glue_url } + [port]
//       worker -> port             { type: "ready" }
//       main   -> port             { type: "call",  id, kind, ptr }
//       worker -> port             { type: "result", id, result }   // success
//       worker -> port             { type: "result", id, error }    // task threw
//     init runs initSync ONCE; later `call` envelopes reuse the same instance.
//     Because dispatch lives on the private port, the worker's default channel
//     is free for the consumer (Worker::raw) — their traffic can never reach it.
//
//   * One-shot macros (spawn! / spawn_blocking!) — the untyped envelope
//     { kind, module, memory, ptr, glue_url } -> postMessage(result_ptr), on the
//     default channel. One-shot workers are never shared, so there is no port.
//     The envelope carries no `type`, so it falls through to the path below.
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

    // --- persistent handle: one-time init + port adoption ---------------
    if (msg && msg.type === 'init') {
        // The control port was transferred alongside this envelope; adopt our
        // end. All worxide protocol runs over it from here on.
        const port = ev.ports[0];

        const glue = await import(msg.glue_url);
        // Attach to the main thread's shared memory and initialize this worker
        // thread. wasm-bindgen's generated initSync handles TLS allocation and
        // per-thread __wbindgen_start for this instance. Done ONCE.
        glue.initSync({ module: msg.module, memory: msg.memory });
        // Plant the resolved glue URL so any nested spawn this worker performs
        // reuses it instead of re-deriving from a crate name / a global it
        // can't see.
        glue.__worxide_seed_glue_url(msg.glue_url);
        // Marks this worker as persistent: gates the one-shot path below so a
        // consumer sharing the default channel can't re-trigger a re-init.
        self.__worxide_glue = glue;

        // Per-call dispatch, isolated on the private port.
        port.onmessage = async (pev) => {
            const m = pev.data;
            if (!m || m.type !== 'call') return; // nothing else on this port is ours
            const { id, kind, ptr } = m;
            try {
                // A Rust panic (panic=abort) traps here as a RuntimeError, and a
                // malformed payload throws from the entry point; either is caught
                // and reported as this one call's error, leaving the worker alive.
                const result = kind === 'sync'
                    ? glue.__worxide_worker_entry(ptr)
                    : await glue.__worxide_worker_entry_async(ptr);
                port.postMessage({ type: 'result', id, result });
            } catch (e) {
                port.postMessage({ type: 'result', id, error: String((e && e.stack) || e) });
            }
        };
        // Assigning port.onmessage auto-starts the port.
        port.postMessage({ type: 'ready' });
        return;
    }

    // --- one-shot path (spawn! / spawn_blocking!) ----------------
    // A persistent worker never takes this path; once initialised, ignore
    // one-shot-shaped envelopes so a consumer on the default channel cannot
    // re-import / re-init. Anything else on this channel (e.g. a consumer's
    // canvas/mouse side-channel) is theirs — let their own listener handle it.
    if (!self.__worxide_glue && msg && msg.type === undefined && msg.glue_url !== undefined) {
        const { kind, module, memory, ptr, glue_url } = msg;
        const glue = await import(glue_url);
        glue.initSync({ module, memory });
        glue.__worxide_seed_glue_url(glue_url);
        const result_ptr = kind === 'sync'
            ? glue.__worxide_worker_entry(ptr)
            : await glue.__worxide_worker_entry_async(ptr);
        self.postMessage(result_ptr);
    }
    // else: an envelope we don't own — ignore.
});