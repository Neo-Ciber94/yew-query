use super::{
    cache::QueryCache, error::QueryError, fetcher::Fetcher, query::Query, retry::Retrier, Error,
};
use futures::TryFutureExt;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    future::Future,
    rc::Rc,
    time::{Duration},
};
use instant::Instant;
use yew::virtual_dom::Key;

pub struct QueryClient {
    cache: Box<dyn QueryCache>,
    stale_time: Option<Duration>,
    retry: Option<Retrier>,
}

impl QueryClient {
    pub fn builder() -> QueryClientBuilder {
        QueryClientBuilder::new()
    }

    pub fn is_stale(&self, key: &Key) -> bool {
        match (self.cache.get(key), self.stale_time) {
            (Some(query), Some(stale_time)) => query.is_stale_by_time(stale_time),
            _ => false,
        }
    }

    pub fn is_cached(&self, key: &Key) -> bool {
        match (self.cache.get(key), self.stale_time) {
            (Some(query), Some(stale_time)) => query.get_if_not_stale(stale_time).is_some(),
            _ => false,
        }
    }

    pub async fn fetch_query<F, Fut, T, E>(&mut self, key: Key, f: F) -> Result<Rc<T>, Error>
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        T: 'static,
        E: Into<Error> + 'static,
    {
        // Get value if cached
        if self.is_cached(&key) {
            if let Some(stale_time) = self.stale_time {
                log::trace!("Using cached data for: {key}");
                return Ok(self
                    .cache
                    .get(&key)
                    .and_then(|x| x.get_if_not_stale(stale_time).cloned())
                    .unwrap() // SAFETY: value is cached
                    .downcast::<T>()
                    .unwrap());
            }
        }

        log::trace!("fetching data for: {key}");
        let retrier = self.retry.as_ref();
        let fetcher = Fetcher::new(move || f().map_ok(|x| Rc::new(x) as Rc<dyn Any>));
        let cache_value = Some(do_fetch(&fetcher, retrier).await?);
        let cache_time = Instant::now();
        let type_id = TypeId::of::<T>();

        self.cache.set(
            key.clone(),
            Query {
                cache_value,
                updated_at: cache_time,
                fetcher,
                type_id,
            },
        );

        let ret = self
            .cache
            .get(&key)
            .and_then(|x| x.cache_value.clone())
            .map(|x| x.downcast::<T>().unwrap())
            .unwrap(); // SAFETY: The value is `T`

        Ok(ret)
    }

    pub async fn refetch_query<T: 'static>(&mut self, key: Key) -> Result<Rc<T>, Error> {
        if let Some(query) = self.cache.get_mut(&key) {
            let retrier = self.retry.as_ref();
            let fetcher = &query.fetcher;
            let cache_value = Some(do_fetch(&fetcher, retrier).await?);

            query.cache_value = cache_value;
            query.updated_at = Instant::now();

            let ret = self
                .cache
                .get(&key)
                .and_then(|x| x.cache_value.clone())
                .unwrap() // SAFETY: value was added to cache
                .downcast::<T>()
                .unwrap();

            Ok(ret)
        } else {
            Err(QueryError::key_not_found(&key).into())
        }
    }

    pub fn get_query_data<T: 'static>(&self, key: &Key) -> Result<Rc<T>, QueryError> {
        if let Some(stale_time) = self.stale_time {
            if let Some(query) = self
                .cache
                .get(key)
                .and_then(|x| x.get_if_not_stale(stale_time).cloned())
            {
                return query
                    .downcast::<T>()
                    .map_err(|_| QueryError::type_mismatch::<T>());
            }
        }

        Err(QueryError::NoCacheValue)
    }

    pub fn set_query_data<T: 'static>(&mut self, key: Key, value: T) -> Result<(), QueryError> {
        if let Some(entry) = self.cache.get_mut(&key) {
            entry.set_value(value)
        } else {
            Err(QueryError::key_not_found(&key))
        }
    }

    pub fn remove_query_data(&mut self, key: &Key) {
        self.cache.remove(key);
    }

    pub fn clear_queries(&mut self) {
        self.cache.clear();
    }

    pub fn invalidate_query_data(&mut self, key: &Key) {
        if let Some(entry) = self.cache.get_mut(key) {
            entry.cache_value = None;
        }
    }
}

impl Debug for QueryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryClient")
            .field("cache", &"QueryCache")
            .field("stale_time", &self.stale_time)
            .field("retry", &"Retry")
            .finish()
    }
}

#[derive(Default)]
pub struct QueryClientBuilder {
    stale_time: Option<Duration>,
    retry: Option<Retrier>,
}

impl QueryClientBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn stale_time(mut self, stale_time: Duration) -> Self {
        self.stale_time = Some(stale_time);
        self
    }

    pub fn retry<R, I>(mut self, retry: R) -> Self
    where
        R: Fn() -> I + 'static,
        I: Iterator<Item = Duration> + 'static,
    {
        self.retry = Some(Retrier::new(retry));
        self
    }

    pub fn build<C>(self, cache: C) -> QueryClient
    where
        C: QueryCache + 'static,
    {
        let Self { stale_time, retry } = self;
        let cache = Box::new(cache);

        QueryClient {
            cache,
            stale_time,
            retry,
        }
    }
}

async fn do_fetch<T: 'static>(fetcher: &Fetcher<T>, retrier: Option<&Retrier>) -> Result<T, Error> {
    let mut ret = fetcher.get().await;

    if ret.is_ok() {
        return ret;
    }

    if let Some(retrier) = retrier {
        let retry = retrier.get();
        for delay in retry {
            utils::sleep(delay).await;
            ret = fetcher.get().await;
        }
    }

    ret
}

mod utils {
    use std::{
        task::Poll,
        time::{Duration},
    };
    use instant::Instant;
    use futures::Future;

    pub async fn sleep(duration: Duration) {
        Sleep::new(duration).await
    }

    pub struct Sleep {
        start: Option<Instant>,
        duration: Duration,
        done: bool,
    }

    impl Sleep {
        pub fn new(duration: Duration) -> Self {
            Sleep {
                start: None,
                duration,
                done: false,
            }
        }
    }

    impl Future for Sleep {
        type Output = ();

        fn poll(
            mut self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            let mut this = self.as_mut();

            if this.done {
                panic!("attempting to poll future after done");
            }

            let start = this.start.get_or_insert_with(|| Instant::now());
            let now = Instant::now();
            let elapsed = now - *start;
            let duration = self.duration;

            if elapsed > duration {
                self.done = true;
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }
}
