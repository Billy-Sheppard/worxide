self.addEventListener('error', e => console.error('[worker] uncaught:', e.message, e));
self.addEventListener('unhandledrejection', e => console.error('[worker] rejection:', e.reason));

self.onmessage = async ({ data: { module, memory, ptr, glue_url } }) => {
    try {
        // Intercept WebAssembly.Memory and WebAssembly.Instance BEFORE importing
        // the glue so that:
        //   1. __wbg_shared_memory in the worker realm = main thread's memory
        //   2. wasm instantiation uses main thread's memory for env.memory
        // This ensures all string reads/writes and heap access go to the same
        // SharedArrayBuffer as the main thread.
        const RealMemory   = WebAssembly.Memory;
        const RealInstance = WebAssembly.Instance;

        WebAssembly.Memory = function() { return memory; };
        WebAssembly.Memory.prototype = RealMemory.prototype;

        WebAssembly.Instance = function(mod, imports) {
            if (!imports)     imports     = {};
            if (!imports.env) imports.env = {};
            imports.env.memory = memory;
            return new RealInstance(mod, imports);
        };
        WebAssembly.Instance.prototype = RealInstance.prototype;

        const { initSync, worker_entry } = await import(glue_url);

        WebAssembly.Memory   = RealMemory;
        WebAssembly.Instance = RealInstance;

        initSync({ module });
        worker_entry(ptr);
    } catch(e) {
        console.error('[worker] error:', e.message, e);
    }
};