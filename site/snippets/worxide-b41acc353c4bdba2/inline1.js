
export function resolve_url_relative_to_location(file_name) {
    const base = (typeof self !== "undefined" && self.location)
        ? self.location.href
        : location.href;
    return new URL(file_name, base).href;
}
