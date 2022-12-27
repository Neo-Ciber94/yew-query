use super::{
    cache::QueryCache, error::QueryError, fetcher::BoxFetcher, query::Query, retry::Retryer, Error,
};
use crate::{fetcher::Fetch, futures::cache::CacheFutureExt, key::QueryKey};
use futures::{FutureExt, TryFutureExt};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    future::Future,
    rc::Rc,
    time::Duration,
};

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
    pub fn is_stale(&self, key: &QueryKey) -> bool {
        let cache = self.cache.borrow();
        if let Some(query) = cache.get(&key) {
            query.is_stale()
        } else {
            false
        }
    }

    /// Returns `true` if is fetching the given key.
    pub fn is_fetching(&self, key: &QueryKey) -> bool {
        match self.cache.borrow().get(key) {
            Some(query) => !query.is_resolved(),
            None => false,
        }
    }

    /// Executes the future, cache and returns the result.
    pub async fn fetch_query<F, Fut, T, E>(&mut self, key: QueryKey, f: F) -> Result<Rc<T>, Error>
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        T: 'static,
        E: Into<Error> + 'static,
    {
        {
            let cache = self.cache.borrow();
            if let Some(query) = cache.get(&key) {
                if !query.is_resolved() {
                    // FIXME: We should not access the prop directly
                    let fut = query.future_or_value.clone();
                    drop(cache);

                    if let Some(last_value) = fut.last_value() {
                        let ret = last_value?.downcast::<T>().unwrap();
                        return Ok(ret);
                    }

                    let value = fut.await?;
                    let ret = value.downcast::<T>().unwrap();
                    return Ok(ret);
                }

                if !query.is_stale() && query.is_resolved() {
                    let value = query.value().expect("query had not resolved");
                    let ret = value
                        .downcast::<T>()
                        .map_err(|_| QueryError::type_mismatch::<T>().into());
                    return ret;
                }
            }
        }

        // We store the future with the last emitted value.
        let last_value = {
            let cache = self.cache.borrow();
            cache.get(&key).map(|x| x.value()).flatten().map(|x| Ok(x))
        };

        let retrier = self.retry.clone();
        let fetcher = BoxFetcher::new(move || f().map_ok(|x| Rc::new(x) as Rc<dyn Any>));
        let fut = fetch_with_retry(fetcher.clone(), retrier)
            .boxed_local()
            .cached_with_initial(last_value);

        // Only store the result in the cache if had stale time
        if self.stale_time.is_some() {
            let mut cache = self.cache.borrow_mut();
            let cache_time = self.stale_time.clone();
            let query = Query::new::<T>(fut.clone(), fetcher, cache_time);
            cache.set(key.clone(), query);
        }

        // Await the value what will update the copy in the cache
        let _ = fut.await?;
        let cache = self.cache.borrow_mut();

        assert!(cache.get(&key).unwrap().is_resolved());

        let ret = cache
            .get(&key)
            .map(|x| x.value().expect("query had not resolved"))
            .map(|x| x.downcast::<T>().unwrap())
            .unwrap(); // SAFETY: The value is `T`

        Ok(ret)
    }

    /// Executes the query with the given key, then cache and return the result.
    pub async fn refetch_query<T: 'static>(&mut self, key: QueryKey) -> Result<Rc<T>, Error> {
        let mut cache = self.cache.borrow_mut();
        let query = cache.get_mut(&key);

        let Some(query) = query else {
            return Err(Error::new(QueryError::key_not_found(&key)));
        };

        // FIXME: We still have the cache borrowed while still refetching
        // this may lead to a borrow error if other thread attempt to read the cache

        let last_value = { query.value().map(|x| Ok(x)) };

        let retrier = self.retry.clone();
        let fetcher = query.fetcher().clone();
        let fut = fetch_with_retry(fetcher, retrier)
            .boxed_local()
            .cached_with_initial(last_value);

        // Update the future
        query.set_future(fut);
        let _ = query.resolve::<T>().await;

        let cache = self.cache.borrow();

        let ret = cache
            .get(&key)
            .map(|x| x.value().unwrap())
            .unwrap() // SAFETY: value was added to cache
            .downcast::<T>()
            .unwrap();

        Ok(ret)
    }

    /// Returns `true` if the given key exists in the cache.
    pub fn contains_query(&self, key: &QueryKey) -> bool {
        let cache = self.cache.borrow();
        return cache.has(key);
    }

    /// Returns the query value associated with the given key.
    ///
    /// # Remarks
    /// This don't checks if the value is not stale.
    pub fn get_query_data<T: 'static>(&self, key: &QueryKey) -> Result<Rc<T>, QueryError> {
        if !key.is_type::<T>() {
            return Err(QueryError::type_mismatch::<T>());
        }

        let cache = self.cache.borrow();
        cache
            .get(key)
            .ok_or(QueryError::NoCacheValue)
            .and_then(|query| {
                query
                    .value()
                    .clone()
                    .ok_or_else(|| QueryError::NotReady)
                    .map(|x| x.downcast::<T>().unwrap())
                    .map_err(|_| QueryError::type_mismatch::<T>())
            })
    }

    /// Sets cache value for given key.
    pub fn set_query_data<T: 'static>(
        &mut self,
        key: QueryKey,
        value: T,
    ) -> Result<(), QueryError> {
        if !key.is_type::<T>() {
            return Err(QueryError::type_mismatch::<T>());
        }

        let mut cache = self.cache.borrow_mut();

        // This check should be done in the commented code below but the borrow checker doesn't allow it
        if let Some(query) = cache.get(&key) {
            if query.type_id() != TypeId::of::<T>() {
                return Err(QueryError::type_mismatch::<T>());
            }
        }

        match cache.get_mut(&key) {
            Some(query) => {
                // For some reason the borrow checker complains about this
                // if query.type_id() != TypeId::of::<T>() {
                //     return Err(QueryError::type_mismatch::<T>());
                // }

                query.set_value(Rc::new(value) as Rc<dyn Any>);
            }
            None => {
                return Err(QueryError::type_mismatch::<T>());
            }
        }

        Ok(())
    }

    /// Removes the query with the given key from the cache.
    pub fn remove_query_data(&mut self, key: &QueryKey) {
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

async fn fetch_with_retry<F, T>(fetcher: F, retrier: Option<Retryer>) -> Result<T, Error>
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
