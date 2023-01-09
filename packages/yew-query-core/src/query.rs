use super::{error::QueryError, fetcher::BoxFetcher};
use crate::{
    client::fetch_with_retry, retry::Retryer, state::QueryState, time::interval::Interval, Error,
};
use futures::{
    future::{ready, LocalBoxFuture, Shared},
    Future, FutureExt, TryFutureExt,
};
use instant::Instant;
use prokio::spawn_local;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    rc::Rc,
    sync::{Arc, RwLock},
    time::Duration,
};

#[derive(Debug)]
struct Inner {
    fetcher: BoxFetcher<Rc<dyn Any>>,
    retrier: Option<Retryer>,
    cache_time: Option<Duration>,
    refetch_time: Option<Duration>,
    updated_at: Option<Instant>,
    last_value: Option<Rc<dyn Any>>,
    future_or_value: Shared<LocalBoxFuture<'static, Result<Rc<dyn Any>, Error>>>,
    interval: Option<Interval>,
    state: QueryState,
}

/// Represents a query.
#[derive(Debug, Clone)]
pub struct Query {
    type_id: TypeId,
    inner: Arc<RwLock<Inner>>,
}

impl Query {
    /// Constructs a new `Query`
    pub fn new<F, Fut, T, E>(
        f: F,
        retrier: Option<Retryer>,
        cache_time: Option<Duration>,
        refetch_time: Option<Duration>,
    ) -> Self
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        T: 'static,
        E: Into<Error> + 'static,
    {
        let type_id = TypeId::of::<T>();
        let fetcher = BoxFetcher::new(move || f().map_ok(|x| Rc::new(x) as Rc<dyn Any>));
        let future_or_value = fetch_with_retry(fetcher.clone(), retrier.clone())
            .boxed_local()
            .shared();

        let inner = Arc::new(RwLock::new(Inner {
            fetcher,
            retrier,
            cache_time,
            refetch_time,
            future_or_value,
            state: QueryState::Idle,
            last_value: None,
            updated_at: None,
            interval: None,
        }));

        Query { type_id, inner }
    }

    fn assert_type<T: 'static>(&self) -> Result<(), QueryError> {
        if self.type_id != TypeId::of::<T>() {
            return Err(QueryError::type_mismatch::<T>());
        }

        Ok(())
    }

    /// Returns the type if of this `Query`.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns the state of this query.
    pub fn state(&self) -> QueryState {
        self.inner.read().unwrap().state.clone()
    }

    /// Returns a future that resolve to this query value.
    pub async fn future<T: 'static>(&self) -> Result<Rc<T>, Error> {
        if self.type_id != TypeId::of::<T>() {
            return Err(Error::new(QueryError::type_mismatch::<T>()));
        }

        let fut = self
            .inner
            .read()
            .expect("failed to read query")
            .future_or_value
            .clone();

        let value = fut.await;
        match value {
            Ok(x) => {
                let ret = x
                    .downcast::<T>()
                    .map_err(|_| QueryError::type_mismatch::<T>().into());

                ret
            }
            Err(err) => Err(err),
        }
    }

    /// Returns `true` if the query is resolving a future.
    pub fn is_fetching(&self) -> bool {
        self.inner.read().unwrap().future_or_value.peek().is_none()
    }

    /// Return the last cache value of this query.
    pub fn last_value(&self) -> Option<Rc<dyn Any>> {
        self.inner.read().unwrap().last_value.clone()
    }

    /// Executes a future that resolves to a value.
    pub async fn fetch<T: 'static>(&mut self) -> Result<Rc<T>, Error> {
        self.assert_type::<T>()?;

        // Only when is empty will be loading, otherwise may use the cache last value.
        if self.last_value().is_none() {
            let mut inner = self.inner.write().expect("failed to write in query");
            inner.state = QueryState::Loading;
        }

        let fut = {
            let mut inner = self.inner.write().expect("failed to write in query");
            let fetcher = inner.fetcher.clone();
            let retrier = inner.retrier.clone();
            let fut = fetch_with_retry(fetcher, retrier.clone())
                .boxed_local()
                .shared();

            // Updates the inner future
            inner.future_or_value = fut.clone();
            fut
        };

        // Await and which updates the inner future
        let value = match fut.await {
            Ok(x) => x,
            Err(err) => {
                let mut inner = self.inner.write().expect("failed to write in query");
                inner.state = QueryState::Failed(err.clone());
                return Err(err);
            }
        };

        // refetch
        self.queue_refetch::<T>();

        let mut inner = self.inner.write().expect("failed to write in query");

        let ret = value
            .downcast::<T>()
            .map_err(|_| QueryError::type_mismatch::<T>())?;

        inner.last_value = Some(ret.clone());
        inner.updated_at = Some(Instant::now());
        inner.state = QueryState::Ready;

        Ok(ret)
    }

    /// Returns `true` if the value of the query is expired.
    pub fn is_stale(&self) -> bool {
        let inner = self.inner.read().unwrap();
        let updated_at = inner.updated_at.clone();
        let cache_time = inner.cache_time.clone();
        drop(inner);

        let Some(updated_at) = updated_at else {
            return false;
        };

        match cache_time {
            Some(cache_time) => {
                let now = Instant::now();
                (now - updated_at) >= cache_time
            }
            None => false,
        }
    }

    /// Sets the value of this query.
    pub fn set_value<T: 'static>(&mut self, value: T) -> Result<(), QueryError> {
        self.assert_type::<T>()?;

        let fut = ready(Ok(Rc::new(value) as Rc<dyn Any>))
            .boxed_local()
            .shared();

        // SAFETY: Value always is Ok(T)
        let value = futures::executor::block_on(fut.clone()).unwrap();

        {
            let mut inner = self.inner.write().expect("failed to write in query");
            inner.future_or_value = fut;
            inner.state = QueryState::Ready;
            inner.last_value = Some(value);
            inner.updated_at = Some(Instant::now());
        }

        // refetch
        self.queue_refetch::<T>();
        Ok(())
    }

    fn queue_refetch<T: 'static>(&self) {
        let mut inner = self.inner.write().unwrap();

        if let Some(refetch_time) = inner.refetch_time {
            if let Some(interval) = inner.interval.take() {
                interval.cancel();
            };

            drop(inner); // We don't need to hold the ownership anymore

            let this = self.clone();
            let interval = Interval::new(refetch_time, move || {
                let this = this.clone();

                spawn_local(async move {
                    // We fetch and ignore the errors, on failure the inner state will be updated
                    let mut this = this.clone();
                    this.fetch::<T>().await.ok();
                });
            });

            let mut inner = self.inner.write().unwrap();
            inner.interval = Some(interval);
        }
    }
}
