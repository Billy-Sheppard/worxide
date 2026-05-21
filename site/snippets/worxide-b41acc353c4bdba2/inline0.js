
export function create_shared_buffer(size) {
    return new SharedArrayBuffer(size);
}

export async function call_register_shims_js(module, glue_url) {
    const glue = await import(glue_url);
    const exports = WebAssembly.Module.exports(module);
    for (const { name } of exports) {
        if (name.startsWith("__register_") && typeof glue[name] === "function") {
            glue[name]();
        }
    }
}
