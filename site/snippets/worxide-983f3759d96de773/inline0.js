
export function spawn_worker(url, memory) {
    const worker = new Worker(url, { type: "module" });
    worker.postMessage(memory); // ONLY memory, nothing else
    return worker;
}
