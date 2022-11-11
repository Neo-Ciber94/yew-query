use crate::timeout::Timeout;

use super::{
    cache::QueryCache, error::QueryError, fetcher::Fetcher, query::Query, retry::Retryer, Error,
};
use futures::TryFutureExt;
use instant::Instant;
use std::{any::Any, cell::RefCell, fmt::Debug, future::Future, rc::Rc, time::Duration};
use wasm_bindgen::JsValue;
use yew::virtual_dom::Key;

pub struct QueryClient {
    cache: Rc<RefCell<Box<dyn QueryCache>>>,
    stale_time: Option<Duration>,
    retry: Option<Retryer>,
}

impl QueryClient {
    pub fn builder() -> QueryClientBuilder {
        QueryClientBuilder::new()
    }

    pub fn is_stale(&self, key: &Key) -> bool {
        let cache = self.cache.borrow();
        match (cache.get(key), self.stale_time) {
            (Some(query), Some(stale_time)) => query.is_stale_by_time(stale_time),
            _ => false,
        }
    }

    pub fn is_cached(&self, key: &Key) -> bool {
        let cache = self.cache.borrow();
        match (cache.get(key), self.stale_time) {
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
        let mut cache = self.cache.borrow_mut();

        // Get value if cached
        if let Some(query) = cache.get(&key) {
            return Ok(query.cache_value.clone().downcast::<T>().unwrap());
        }

        let retrier = self.retry.as_ref();
        let fetcher = Fetcher::new(move || f().map_ok(|x| Rc::new(x) as Rc<dyn Any>));
        let cache_value = do_fetch(&fetcher, retrier).await?;
        let updated_at = Instant::now();
        let timeout = self.schedule_delete(&key);

        cache.set(
            key.clone(),
            Query {
                cache_value,
                updated_at,
                fetcher,
                timeout,
            },
        );

        let ret = cache
            .get(&key)
            .map(|x| x.cache_value.clone())
            .map(|x| x.downcast::<T>().unwrap())
            .unwrap(); // SAFETY: The value is `T`

        Ok(ret)
    }

    pub async fn refetch_query<T: 'static>(&mut self, key: Key) -> Result<Rc<T>, Error> {
        let mut cache = self.cache.borrow_mut();
        if let Some(query) = cache.get_mut(&key) {
            let retrier = self.retry.as_ref();
            let fetcher = &query.fetcher;
            let cache_value = do_fetch(&fetcher, retrier).await?;

            query.cache_value = cache_value;
            query.updated_at = Instant::now();
            self.schedule_delete(&key);

            let ret = cache
                .get(&key)
                .map(|x| x.cache_value.clone())
                .unwrap() // SAFETY: value was added to cache
                .downcast::<T>()
                .unwrap();

            Ok(ret)
        } else {
            Err(QueryError::key_not_found(&key).into())
        }
    }

    pub async fn fetch_infinite_query<F, Fut, T, E>(
        &mut self,
        key: Key,
        param: usize,
        f: F,
    ) -> Result<Rc<Vec<T>>, Error>
    where
        F: Fn(usize) -> Fut + 'static,
        Fut: Future<Output = Option<Result<T, E>>> + 'static,
        T: 'static,
        E: Into<Error> + 'static,
    {
        todo!()
    }

    pub async fn refetch_infinite_query<T>(
        &mut self,
        key: Key,
        param: usize,
    ) -> Result<Rc<Vec<T>>, Error>
    where
        T: 'static,
    {
        todo!()
    }

    pub fn get_query_data<T: 'static>(&self, key: &Key) -> Result<Rc<T>, QueryError> {
        let cache = self.cache.borrow();

        cache
            .get(key)
            .ok_or(QueryError::NoCacheValue)
            .and_then(|query| {
                query
                    .cache_value
                    .clone()
                    .downcast::<T>()
                    .map_err(|_| QueryError::type_mismatch::<T>())
            })
    }

    pub fn set_query_data<T: 'static>(&mut self, key: Key, value: T) -> Result<(), QueryError> {
        let mut cache = self.cache.borrow_mut();
        if let Some(query) = cache.get_mut(&key) {
            let ret = query.set_value(value);
            self.schedule_delete(&key);
            ret
        } else {
            Err(QueryError::key_not_found(&key))
        }
    }

    pub fn remove_query_data(&mut self, key: &Key) {
        self.cache.borrow_mut().remove(key);
    }

    pub fn clear_queries(&mut self) {
        self.cache.borrow_mut().clear();
    }

    fn schedule_delete(&self, key: &Key) -> Option<Timeout> {
        // Clear existing timeout
        {
            let mut cache = self.cache.borrow_mut();
            let query = cache.get_mut(key)?;
            if let Some(timeout) = query.timeout.take() {
                timeout.clear();
            }
        }

        let cache = self.cache.clone();
        let stale_time = self.stale_time.as_ref()?;

        // Duration millis cannot go past i32 due js limits
        let millis = stale_time.as_millis().min(i32::MAX as u128) as i32;
        let key = key.clone();

        Some(Timeout::new(millis, move || {
            let mut cache = cache.borrow_mut();
            cache.remove(&key);

            let s = JsValue::from_str("Removed");
            web_sys::console::log_1(&s);
        }))
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

#[derive(Default)]
pub struct QueryClientBuilder {
    stale_time: Option<Duration>,
    retry: Option<Retryer>,
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
        self.retry = Some(Retryer::new(retry));
        self
    }

    pub fn build<C>(self, cache: C) -> QueryClient
    where
        C: QueryCache + 'static,
    {
        let Self { stale_time, retry } = self;
        let cache = Box::new(cache) as Box<dyn QueryCache>;
        let cache = Rc::new(RefCell::new(cache));

        QueryClient {
            cache,
            stale_time,
            retry,
        }
    }
}

async fn do_fetch<T: 'static>(fetcher: &Fetcher<T>, retrier: Option<&Retryer>) -> Result<T, Error> {
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
