// worxide worker bootstrap.
//
// Embedded into the crate via include_str! and served from a Blob URL at
// runtime. This code runs before the wasm module is initialized.
//
// Supported initialization messages:
//
//   Shared persistent worker:
//     { type: "init", module, memory, glue_url } + control port
//
//   Isolated persistent worker:
//     { type: "init_isolated", module, maximum_bytes?, glue_url }
//     + control port
//
//   Shared one-shot worker:
//     { kind, module, memory, ptr, glue_url }
//
// Persistent workers use a private MessagePort for worxide traffic.
// The default message channel remains available to downstream applications.
//
// Shared workers attach to the caller's memory. Isolated workers initialize
// a fresh memory using the generated glue's descriptor, optionally replacing
// its maximum.

// Surface worker-side errors in the page console for diagnostics.
self.addEventListener('error', e =>
    console.error('[worxide:worker]', e.message)
);

self.addEventListener('unhandledrejection', e =>
    console.error('[worxide:worker]', e.reason)
);

let isolatedStarted = false;

function initializeIsolated(glue, module, maximumBytes) {
    const OriginalMemory = WebAssembly.Memory;

    let capturedMemory = null;
    let capturedDescriptor = null;

    const replacement = new Proxy(OriginalMemory, {
        construct(target, args) {
            if (capturedMemory !== null) {
                return Reflect.construct(target, args);
            }

            const descriptor = { ...args[0] };

            if (maximumBytes !== undefined) {
                if (
                    !Number.isSafeInteger(maximumBytes) ||
                    maximumBytes <= 0 ||
                    maximumBytes % 65536 !== 0
                ) {
                    throw new Error(
                        "worxide: invalid maximum memory size"
                    );
                }

                const pages = maximumBytes / 65536;
                const initial =
                    descriptor.initial ?? descriptor.minimum;

                if (initial === undefined) {
                    throw new Error(
                        "worxide: memory descriptor has no initial size"
                    );
                }

                if (BigInt(pages) < BigInt(initial)) {
                    throw new Error(
                        "worxide: maximum is below initial memory size"
                    );
                }

                descriptor.maximum =
                    typeof initial === "bigint" ||
                        typeof descriptor.maximum === "bigint"
                        ? BigInt(pages)
                        : pages;
            }

            const memory = Reflect.construct(target, [descriptor]);

            capturedMemory = memory;
            capturedDescriptor = descriptor;

            return memory;
        },
    });

    let exports;

    try {
        WebAssembly.Memory = replacement;
        exports = glue.initSync({ module });
    } finally {
        WebAssembly.Memory = OriginalMemory;
    }

    const memory = exports && exports.memory;

    if (!(memory instanceof OriginalMemory)) {
        throw new Error(
            "worxide: initialized module did not export memory"
        );
    }

    if (maximumBytes !== undefined && capturedMemory !== memory) {
        throw new Error(
            "worxide: generated glue did not construct exported memory"
        );
    }

    const maximumPages =
        capturedMemory === memory
            ? capturedDescriptor?.maximum
            : undefined;

    return {
        initial_bytes: memory.buffer.byteLength,
        maximum_bytes:
            maximumPages === undefined
                ? null
                : Number(maximumPages) * 65536,
    };
}

self.addEventListener('message', async (ev) => {
    const msg = ev.data;

    // Isolated workers never enter the shared-pointer paths below.
    // Returning here affects only worxide's listener, not application
    // listeners installed on the default message channel.
    if (isolatedStarted) return;

    // --- isolated handle: independent memory + port adoption -----------
    if (msg && msg.type === 'init_isolated') {
        if (self.__worxide_glue) return;

        const port = ev.ports[0];
        if (!port) return;

        // Set before awaiting import to prevent duplicate isolated init.
        isolatedStarted = true;

        try {
            const glue = await import(msg.glue_url);

            const stats = initializeIsolated(
                glue,
                msg.module,
                msg.maximum_bytes,
            );

            glue.__worxide_seed_glue_url(msg.glue_url);
            self.__worxide_glue = glue;

            // Application dispatch stays on the default channel.
            // Reserve the private port for worxide.
            port.onmessage = () => { };

            port.postMessage({
                type: 'ready',
                ...stats,
            });
        } catch (e) {
            port.postMessage({
                type: 'init_error',
                error: String((e && e.stack) || e),
            });

            port.close();
            self.close();
        }

        return;
    }

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

        // Seed the resolved glue URL for nested spawns.
        glue.__worxide_seed_glue_url(msg.glue_url);

        // Prevent one-shot-shaped consumer messages from reinitializing
        // this persistent worker.
        self.__worxide_glue = glue;

        // Per-call dispatch, isolated on the private port.
        port.onmessage = async (pev) => {
            const m = pev.data;
            if (!m || m.type !== 'call') return;

            const { id, kind, ptr } = m;

            try {
                // Preserve the existing per-call error protocol.
                // Catching a trap does not guarantee the Rust runtime
                // remains usable afterward.
                const result = kind === 'sync'
                    ? glue.__worxide_worker_entry(ptr)
                    : await glue.__worxide_worker_entry_async(ptr);

                port.postMessage({
                    type: 'result',
                    id,
                    result,
                });
            } catch (e) {
                port.postMessage({
                    type: 'result',
                    id,
                    error: String((e && e.stack) || e),
                });
            }
        };

        // Assigning port.onmessage auto-starts the port.
        port.postMessage({ type: 'ready' });
        return;
    }

    // --- one-shot path (spawn! / spawn_blocking!) -----------------------
    // A persistent worker never takes this path once initialized.
    // Other default-channel messages belong to the consumer.
    if (
        !self.__worxide_glue &&
        msg &&
        msg.type === undefined &&
        msg.glue_url !== undefined
    ) {
        const { kind, module, memory, ptr, glue_url } = msg;
        const glue = await import(glue_url);

        glue.initSync({ module, memory });
        glue.__worxide_seed_glue_url(glue_url);

        const result_ptr = kind === 'sync'
            ? glue.__worxide_worker_entry(ptr)
            : await glue.__worxide_worker_entry_async(ptr);

        self.postMessage(result_ptr);
    }

    // Unrecognized envelopes belong to the application.
});