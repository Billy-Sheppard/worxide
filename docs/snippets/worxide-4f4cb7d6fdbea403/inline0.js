
    export function worxide_glue_url(crate_name) {
        // Cargo replaces hyphens with underscores in generated library filenames.
        // Strip a trailing ".js" so callers may pass either a crate-ish name or
        // an already-js-looking filename stem.
        const file = crate_name.replace(/-/g, "_").replace(/\.js$/, "");
        return new URL("../../" + file + ".js", import.meta.url).href;
    }
    export function worxide_glue_url_from_path(path) {
        // Resolve explicit consumer paths/URLs against the page base. Do not
        // mangle these the way crate names are mangled.
        return new URL(path, document.baseURI).href;
    }
    export function worxide_app_js_path() {
        // Optional consumer-set global, e.g.:
        //   globalThis.app_js_path = "my_app.js";
        //   globalThis.app_js_path = "/static/my_app.js";
        const p = globalThis.app_js_path;
        return (typeof p === "string" && p.length > 0) ? p : null;
    }
