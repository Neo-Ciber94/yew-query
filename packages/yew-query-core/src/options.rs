use crate::retry::Retry;
use instant::Duration;

/// Options for a query.
#[derive(Debug, Default, Clone)]
pub struct QueryOptions {
    pub(crate) cache_time: Option<Duration>,
    pub(crate) refetch_time: Option<Duration>,
    pub(crate) retry: Option<Retry>,
}

impl QueryOptions {
    /// Constructs an empty `QueryOptions`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the cache time for a query.
    pub fn cache_time(mut self, duration: Duration) -> Self {
        self.cache_time = Some(duration);
        self
    }

    /// Sets the refetch time for a query.
    pub fn refetch_time(mut self, duration: Duration) -> Self {
        self.refetch_time = Some(duration);
        self
    }

    /// Sets a retry function for a query on failure.
    pub fn retry<F, I>(mut self, retry: F) -> Self
    where
        F: Fn() -> I + 'static,
        I: Iterator<Item = Duration> + 'static,
    {
        self.retry = Some(Retry::new(retry));
        self
    }
}
