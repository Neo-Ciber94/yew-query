use super::{cache::QueryCache, error::QueryError, query::Query, retry::Retryer, Error};
use crate::{fetcher::Fetch, key::QueryKey, state::QueryState};
use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell},
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
    cache_time: Option<Duration>,
    refetch_time: Option<Duration>,
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
            Some(query) => !query.is_fetching(),
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
        // If is fetching for the query still fresh in cache
        {
            let cache = self.cache.borrow();
            if let Some(query) = cache.get(&key).cloned() {
                // This prevent borrow errors
                drop(cache);

                if !query.is_stale() && query.last_value().is_some() {
                    let last_value = query.last_value().clone().unwrap();
                    let ret = last_value
                        .downcast::<T>()
                        .map_err(|_| QueryError::type_mismatch::<T>().into());

                    return ret;
                } else if query.is_fetching() {
                    let ret = query.future::<T>().await;
                    return ret;
                }
            }
        }

        let can_cache = self.cache_time.is_some();
        let retrier = self.retry.clone();

        // Only store the result in the cache if had stale time
        if !can_cache {
            let ret = fetch_with_retry(f, retrier).await?;
            return Ok(Rc::new(ret));
        }

        let mut query = {
            let mut cache = self.cache.borrow_mut();
            match cache.get(&key).cloned() {
                Some(x) => x,
                None => {
                    let query = Query::new(f, retrier.clone(), self.cache_time, self.refetch_time);
                    cache.set(key.clone(), query.clone());

                    query
                }
            }
        };

        // Await the value what will update the copy in the cache
        let value = query.fetch::<T>().await?;

        Ok(value)
    }

    /// Executes the query with the given key, then cache and return the result.
    pub async fn refetch_query<T: 'static>(&mut self, key: QueryKey) -> Result<Rc<T>, Error> {
        let cache = self.cache.borrow_mut();
        let query = cache.get(&key).cloned();

        // We drop ownership to prevent borrow errors
        drop(cache);

        let Some(mut query) = query else {
            return Err(Error::new(QueryError::key_not_found(&key)));
        };

        let ret = query.fetch().await?;
        Ok(ret)
    }

    /// Returns the query associated with the given key.
    pub fn get_query(&self, key: &QueryKey) -> Option<Ref<'_, Query>> {
        let cache = self.cache.borrow();
        if !cache.has(key) {
            return None;
        }

        let ret = Ref::map(cache, |x| &*x.get(key).unwrap());
        Some(ret)
    }

    /// Returns `true` if there is a query associated with the given key.
    pub fn contains_query(&self, key: &QueryKey) -> bool {
        let cache = self.cache.borrow();
        return cache.has(key);
    }

    /// Returns `true` if there is cached data associated with the given key.
    pub fn has_query_data(&self, key: &QueryKey) -> bool {
        self.get_query(key).map(|q| !q.is_stale()).unwrap_or(false)
    }

    /// Returns the cache query data associated with the given key.
    ///
    /// # Returns
    /// - `Ok(Rc(T))` if the data is fresh in cache.
    /// - `Err(QueryError::KeyNotFound)` if there is not query associated with the given key.
    /// - `Err(QueryError::StaleValue)` if the query exists but is stale.
    /// - `Err(QueryError::TypeMismatch)` if the key don't match the given type or
    /// if the query value cannot be converted to the given type.
    pub fn get_query_data<T: 'static>(&self, key: &QueryKey) -> Result<Rc<T>, QueryError> {
        if !key.is_type::<T>() {
            return Err(QueryError::type_mismatch::<T>());
        }

        let cache = self.cache.borrow();
        cache
            .get(key)
            .ok_or_else(|| QueryError::key_not_found(key))
            .and_then(|q| {
                if q.is_stale() {
                    Err(QueryError::StaleValue)
                } else {
                    Ok(q)
                }
            })
            .and_then(|query| {
                query
                    .last_value()
                    .clone()
                    .ok_or_else(|| QueryError::NotReady)
                    .map(|x| x.downcast::<T>().unwrap())
                    .map_err(|_| QueryError::type_mismatch::<T>())
            })
    }

    /// Returns the state of the query with the given key.
    ///
    /// # Returns
    /// - `Some(QueryState)`: with the state of the query.
    /// - `None`: if the query do not exists.
    pub fn get_query_state(&self, key: &QueryKey) -> Option<QueryState> {
        self.cache
            .borrow()
            .get(key)
            .filter(|q| !q.is_stale())
            .clone()
            .map(|x| x.state())
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

                query.set_value(value);
            }
            None => {
                return Err(QueryError::type_mismatch::<T>());
            }
        }

        Ok(())
    }

    /// Removes the query with the given key from the cache.
    pub fn remove_query_data(&mut self, key: &QueryKey) -> bool {
        let mut cache = self.cache.borrow_mut();
        cache.remove(key).is_some()
    }

    /// Removes all the query data from the cache.
    pub fn clear_queries(&mut self) {
        let mut cache = self.cache.borrow_mut();
        cache.clear();
    }
}

impl Debug for QueryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryClient")
            .field("cache", &self.cache)
            .field("cache_time", &self.cache_time)
            .field("refetch_time", &self.refetch_time)
            .field("retry", {
                if self.retry.is_none() {
                    &"None"
                } else {
                    &"Some(Retryer)"
                }
            })
            .finish()
    }
}

/// A builder for creating a `QueryClient`.
#[derive(Default)]
pub struct QueryClientBuilder {
    cache: Option<Rc<RefCell<dyn QueryCache>>>,
    cache_time: Option<Duration>,
    refetch_time: Option<Duration>,
    retry: Option<Retryer>,
}

impl QueryClientBuilder {
    /// Constructs an empty `QueryClientBuilder`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the time a query can be reused from cache.
    pub fn cache_time(mut self, cache_time: Duration) -> Self {
        self.cache_time = Some(cache_time);
        self
    }

    /// Sets the interval at which the data will be refetched.
    pub fn refetch_time(mut self, refetch_time: Duration) -> Self {
        self.refetch_time = Some(refetch_time);
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
            cache_time: stale_time,
            retry,
            cache,
            refetch_time,
        } = self;

        let cache = cache
            .or_else(|| Some(Rc::new(RefCell::new(HashMap::new()))))
            .unwrap();

        QueryClient {
            cache,
            cache_time: stale_time,
            refetch_time,
            retry,
        }
    }
}

pub(crate) async fn fetch_with_retry<F, T>(fetcher: F, retrier: Option<Retryer>) -> Result<T, Error>
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
            prokio::time::sleep(delay).await;
            ret = fetcher.get().await;
        }
    }

    ret
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use futures::Future;
    use instant::Duration;
    use tokio::task::LocalSet;

    use crate::{error::QueryError, QueryClient, QueryKey};

    #[tokio::test]
    async fn fetch_and_cache_query_test() {
        #[derive(Debug, PartialEq)]
        struct Item {
            name: String,
        }

        run_local(async {
            let mut client = QueryClient::builder()
                .cache_time(Duration::from_millis(400))
                .build();

            let key = QueryKey::of::<Item>("sword");

            assert!(!client.contains_query(&key));

            let ret = client
                .fetch_query(key.clone(), || async {
                    Ok::<_, Infallible>(Item {
                        name: "Fire Sword".to_owned(),
                    })
                })
                .await;

            assert_eq!(
                ret.ok().as_deref(),
                Some(&Item {
                    name: "Fire Sword".to_owned()
                })
            );

            assert!(!client.is_stale(&key));
            assert_eq!(
                client.get_query_data::<Item>(&key).ok().as_deref(),
                Some(&Item {
                    name: "Fire Sword".to_owned()
                })
            );

            // Let the data expire
            tokio::time::sleep(Duration::from_millis(500)).await;

            assert!(client.is_stale(&key));
            assert!(matches!(
                client.get_query_data::<Item>(&key),
                Err(QueryError::StaleValue)
            ));
        })
        .await;
    }

    async fn run_local<Fut>(future: Fut) -> Fut::Output
    where
        Fut: Future,
    {
        let local_set = LocalSet::new();
        local_set.run_until(future).await
    }
}
