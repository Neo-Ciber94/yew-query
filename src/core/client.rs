use super::{cache::QueryCache, error::Error, fetcher::Fetcher, query::Query, retry::Retrier};
use futures::TryFutureExt;
use std::{
    any::{Any, TypeId},
    future::Future,
    time::{Duration, Instant},
};
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

    pub async fn fetch_query<F, Fut, T, E>(&mut self, key: Key, f: F) -> Result<&T, Error>
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        T: 'static,
        E: Into<Error> + 'static,
    {
        // Get value if cached
        if let Some(stale_time) = self.stale_time {
            let ret = self
                .cache
                .get(&key)
                .and_then(|x| x.get_if_not_stale(stale_time))
                .and_then(|x| x.downcast_ref::<T>())
                .unwrap();

            return Ok(ret);
        }

        let retrier = self.retry.as_ref();
        let fetcher = Fetcher::new(move || f().map_ok(|x| Box::new(x) as Box<dyn Any>));
        let cache_value = Some(fetch_boxed(&fetcher, retrier).await?);
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
            .and_then(|x| x.cache_value.as_ref())
            .and_then(|x| x.downcast_ref::<T>())
            .unwrap();

        Ok(ret)
    }

    pub async fn refetch_query<T: 'static>(&mut self, key: Key) -> Result<&T, Error> {
        if let Some(query) = self.cache.get_mut(&key) {
            let retrier = self.retry.as_ref();
            let fetcher = &query.fetcher;
            let cache_value = Some(fetch_boxed(&fetcher, retrier).await?);

            query.cache_value = cache_value;
            query.updated_at = Instant::now();

            let ret = self
                .cache
                .get(&key)
                .and_then(|x| x.cache_value.as_ref())
                .and_then(|x| x.downcast_ref::<T>())
                .unwrap();

            Ok(ret)
        } else {
            todo!()
        }
    }

    pub fn get_query_data<T: 'static>(&self, key: &Key) -> Option<&T> {
        let stale_time = self.stale_time?;

        self.cache
            .get(key)
            .and_then(|x| x.get_if_not_stale(stale_time))
            .and_then(|x| x.downcast_ref::<T>())
    }

    pub fn set_query_data<T: 'static>(&mut self, key: Key, value: T) -> Result<(), Error> {
        if let Some(entry) = self.cache.get_mut(&key) {
            entry.set_value(value)
        } else {
            panic!("key not found");
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

async fn fetch_boxed<T: 'static>(
    fetcher: &Fetcher<T>,
    retrier: Option<&Retrier>,
) -> Result<T, Error> {
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
        time::{Duration, Instant},
    };

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
