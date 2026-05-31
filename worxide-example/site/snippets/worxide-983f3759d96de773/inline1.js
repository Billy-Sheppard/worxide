
    export function worxide_glue_url(crate_name) {
        // Cargo replaces hyphens with underscores in library output filenames,
        // so a crate named "my-app" produces "my_app.js" / "my_app_bg.wasm".
        const file_name = crate_name.replace(/-/g, "_");
        return new URL("../../" + file_name + ".js", import.meta.url).href;
    }
