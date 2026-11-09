
    export function worxide_isolated_ready_timeout(promise) {
        let timer;
        const timeout = new Promise((_, reject) => {
            timer = setTimeout(() => {
                reject(new Error(
                    "worxide: isolated worker initialization timed out"
                ));
            }, 30000);
        });

        return Promise.race([promise, timeout])
            .finally(() => clearTimeout(timer));
    }
