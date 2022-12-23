use super::{
    cache::QueryCache, error::QueryError, fetcher::BoxFetcher, query::Query, retry::Retryer, Error,
};
use crate::fetcher::Fetch;
use futures::TryFutureExt;
use instant::Instant;
use std::{
    any::Any, cell::RefCell, collections::HashMap, fmt::Debug, future::Future, rc::Rc,
    time::Duration,
};
use yew::virtual_dom::Key;

/// Mechanism used for fetching and caching queries.
#[derive(Clone)]
pub struct QueryClient {
    cache: Rc<RefCell<dyn QueryCache>>,
    stale_time: Option<Duration>,
    retry: Option<Retryer>,
}

impl QueryClient {
    /// Returns a builder for a `QueryClient`.
    pub fn builder() -> QueryClientBuilder {
        QueryClientBuilder::new()
    }

    /// Returns `true` if the value for the given key not expired.
    pub fn is_stale(&self, key: &Key) -> bool {
        let cache = self.cache.borrow();
        if let Some(query) = cache.get(key) {
            query.is_stale()
        } else {
            false
        }
    }

    /// Executes the future, cache and returns the result.
    pub async fn fetch_query<F, Fut, T, E>(&mut self, key: Key, f: F) -> Result<Rc<T>, Error>
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        T: 'static,
        E: Into<Error> + 'static,
    {
        // Get value if cached
        if !self.is_stale(&key) {
            let cache = self.cache.borrow();
            if let Some(query) = cache.get(&key) {
                return Ok(query.value.clone().downcast::<T>().unwrap());
            }
        }

        let retrier = self.retry.as_ref();

        if self.stale_time.is_none() {
            let ret = fetch_with_retry(&f, retrier).await;
            return ret.map(|x| Rc::new(x));
        }

        let fetcher = BoxFetcher::new(move || f().map_ok(|x| Rc::new(x) as Rc<dyn Any>));
        let value = fetch_with_retry(&fetcher, retrier).await?;
        let updated_at = Instant::now();

        let mut cache = self.cache.borrow_mut();

        cache.set(
            key.clone(),
            Query {
                value,
                updated_at,
                fetcher,
                cache_time: self.stale_time.clone(),
            },
        );

        let ret = cache
            .get(&key)
            .map(|x| x.value.clone())
            .map(|x| x.downcast::<T>().unwrap())
            .unwrap(); // SAFETY: The value is `T`

        Ok(ret)
    }

    /// Executes the query with the given key, then cache and return the result.
    pub async fn refetch_query<T: 'static>(&mut self, key: Key) -> Result<Rc<T>, Error> {
        let mut cache = self.cache.borrow_mut();
        let query = cache.get_mut(&key);

        let Some(query) = query else {
            return Err(QueryError::key_not_found(&key).into());
        };

        // FIXME: We still have the cache borrowed while still refetching
        // this may lead to a borrow error if other thread attempt to read the cache

        let retrier = self.retry.as_ref();
        let fetcher = &query.fetcher;
        let value = fetch_with_retry(fetcher, retrier).await?;

        query.value = value;
        query.updated_at = Instant::now();

        let cache = self.cache.borrow();

        let ret = cache
            .get(&key)
            .map(|x| x.value.clone())
            .unwrap() // SAFETY: value was added to cache
            .downcast::<T>()
            .unwrap();

        Ok(ret)
    }

    /// Returns `true` if the given key exists in the cache.
    pub fn contains_query(&self, key: &Key) -> bool {
        let cache = self.cache.borrow();
        return cache.has(key);
    }

    /// Returns the query value associated with the given key.
    /// 
    /// # Remarks
    /// This don't checks if the value is not stale.
    pub fn get_query_data<T: 'static>(&self, key: &Key) -> Result<Rc<T>, QueryError> {
        let cache = self.cache.borrow();
        cache
            .get(key)
            .ok_or(QueryError::NoCacheValue)
            .and_then(|query| {
                query
                    .value
                    .clone()
                    .downcast::<T>()
                    .map_err(|_| QueryError::type_mismatch::<T>())
            })
    }

    /// Sets cache value for given key.
    pub fn set_query_data<T: 'static>(&mut self, key: Key, value: T) -> Result<(), QueryError> {
        let mut cache = self.cache.borrow_mut();
        if let Some(query) = cache.get_mut(&key) {
            let ret = query.set_value(value);
            ret
        } else {
            Err(QueryError::key_not_found(&key))
        }
    }

    /// Removes the query with the given key from the cache.
    pub fn remove_query_data(&mut self, key: &Key) {
        let mut cache = self.cache.borrow_mut();
        cache.remove(key);
    }

    /// Removes all the query data from the cache.
    pub fn clear_queries(&mut self) {
        let mut cache = self.cache.borrow_mut();
        cache.clear();
    }
}

impl PartialEq for QueryClient {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.cache, &other.cache)
    }
}

impl Debug for QueryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryClient")
            .field("cache", &self.cache)
            .field("stale_time", &self.stale_time)
            .field("retry", &"Retryer")
            .finish()
    }
}

/// A builder for creating a `QueryClient`.
#[derive(Default)]
pub struct QueryClientBuilder {
    cache: Option<Rc<RefCell<dyn QueryCache>>>,
    stale_time: Option<Duration>,
    retry: Option<Retryer>,
}

impl QueryClientBuilder {
    /// Constructs an empty `QueryClientBuilder`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the max time a query can be reused from cache.
    pub fn stale_time(mut self, stale_time: Duration) -> Self {
        self.stale_time = Some(stale_time);
        self
    }

    /// Sets a function used to retry a failed execution.
    pub fn retry<R, I>(mut self, retry: R) -> Self
    where
        R: Fn() -> I + 'static,
        I: Iterator<Item = Duration> + 'static,
    {
        self.retry = Some(Retryer::new(retry));
        self
    }

    /// Sets the cache implementation used for the client.
    pub fn cache<C>(mut self, cache: C) -> Self
    where
        C: QueryCache + 'static,
    {
        self.cache = Some(Rc::new(RefCell::new(cache)));
        self
    }

    /// Returns the `QueryClient` using this builder options.
    pub fn build(self) -> QueryClient {
        let Self {
            stale_time,
            retry,
            cache,
        } = self;

        let cache = cache
            .or_else(|| Some(Rc::new(RefCell::new(HashMap::new()))))
            .unwrap();

        QueryClient {
            cache,
            stale_time,
            retry,
        }
    }
}

async fn fetch_with_retry<F, T>(fetcher: &F, retrier: Option<&Retryer>) -> Result<T, Error>
where
    F: Fetch<T> + 'static,
    T: 'static,
{
    let mut ret = fetcher.get().await;

    if ret.is_ok() {
        return ret;
    }

    if let Some(retrier) = retrier {
        let retry = retrier.get();
        for delay in retry {
            utils::sleep(delay).await;
            //yew::platform::time::sleep(delay).await;
            ret = fetcher.get().await;
        }
    }

    ret
}

mod utils {
    use futures::Future;
    use instant::Instant;
    use std::{task::Poll, time::Duration};

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
