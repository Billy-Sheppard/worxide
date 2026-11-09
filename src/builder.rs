use {
    crate::{isolated::IsolatedWorker, Worker},
    anyhow::Result,
};

#[derive(Clone, Debug, Default)]
#[must_use = "call .spawn().await to construct the worker"]
pub struct WorkerBuilder {
    glue_url: Option<String>,
}

impl WorkerBuilder {
    /// Override the application glue URL.
    pub fn glue_url(mut self, url: impl Into<String>) -> Self {
        self.glue_url = Some(url.into());
        self
    }

    /// Create an independent memory instead of joining the caller's memory.
    pub fn isolated(self) -> IsolatedWorkerBuilder { IsolatedWorkerBuilder { glue_url: self.glue_url, maximum_bytes: None } }

    /// Spawn a persistent worker sharing the caller's memory.
    ///
    /// URL discovery follows `Worker::new()` when no override is supplied.
    pub async fn spawn(self) -> Result<Worker> {
        match self.glue_url {
            Some(url) => Worker::with_glue(&url).await,
            None => Worker::new().await,
        }
    }
}

#[derive(Clone, Debug)]
#[must_use = "call .spawn().await to construct the worker"]
pub struct IsolatedWorkerBuilder {
    glue_url: Option<String>,
    maximum_bytes: Option<u64>,
}

impl IsolatedWorkerBuilder {
    pub fn glue_url(mut self, url: impl Into<String>) -> Self {
        self.glue_url = Some(url.into());
        self
    }

    pub fn max_256mib(self) -> Self { self.max_memory(256 * 1024 * 1024) }

    pub fn max_512mib(self) -> Self { self.max_memory(512 * 1024 * 1024) }

    pub fn max_1gib(self) -> Self { self.max_memory(1024 * 1024 * 1024) }

    pub fn max_2gib(self) -> Self { self.max_memory(2 * 1024 * 1024 * 1024) }

    pub fn max_4gib(self) -> Self { self.max_memory(4 * 1024 * 1024 * 1024) }

    pub fn max_6gib(self) -> Self { self.max_memory(4 * 1024 * 1024 * 1024) }

    pub fn max_8gib(self) -> Self { self.max_memory(8 * 1024 * 1024 * 1024) }

    pub fn max_10gib(self) -> Self { self.max_memory(8 * 1024 * 1024 * 1024) }

    pub fn max_12gib(self) -> Self { self.max_memory(8 * 1024 * 1024 * 1024) }

    pub fn max_14gib(self) -> Self { self.max_memory(8 * 1024 * 1024 * 1024) }

    pub fn max_16gib(self) -> Self { self.max_memory(16 * 1024 * 1024 * 1024) }

    pub fn max_memory(mut self, bytes: u64) -> Self {
        self.maximum_bytes = Some(bytes);
        self
    }

    /// Spawn a worker with an independent heap.
    ///
    /// Without a preset, use the generated glue's default memory limits.
    ///
    /// A preset is a growth ceiling, not an immediate allocation.
    pub async fn spawn(self) -> Result<IsolatedWorker> {
        let glue_url = match self.glue_url {
            Some(url) => crate::private::glue_url_explicit(&url),
            None => crate::private::glue_url_via_app_path()?,
        };

        IsolatedWorker::boot(glue_url, self.maximum_bytes).await
    }
}
