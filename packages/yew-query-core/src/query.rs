use super::{error::QueryError, fetcher::BoxFetcher};
use crate::{client::fetch_with_retry, retry::Retryer, Error};
use futures::{
    future::{ready, LocalBoxFuture, Shared},
    Future, FutureExt, TryFutureExt,
};
use instant::Instant;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    rc::Rc,
    sync::{Arc, RwLock},
    time::Duration,
};

struct Inner {
    fetcher: BoxFetcher<Rc<dyn Any>>,
    cache_time: Option<Duration>,
    updated_at: Option<Instant>,
    last_value: Option<Rc<dyn Any>>,
    future_or_value: Shared<LocalBoxFuture<'static, Result<Rc<dyn Any>, Error>>>,
}

impl Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inner")
            .field("fetcher", &self.fetcher)
            .field("cache_time", &self.cache_time)
            .field("updated_at", &self.updated_at)
            .field("last_value", &self.last_value)
            .field("future_or_value", &self.future_or_value)
            .finish()
    }
}

/// Represents a query.
#[derive(Debug, Clone)]
pub struct Query {
    type_id: TypeId,
    inner: Arc<RwLock<Inner>>,
}

impl Query {
    pub fn new<F, Fut, T, E>(f: F, retrier: Option<Retryer>, cache_time: Option<Duration>) -> Self
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        T: 'static,
        E: Into<Error> + 'static,
    {
        let type_id = TypeId::of::<T>();
        let fetcher = BoxFetcher::new(move || f().map_ok(|x| Rc::new(x) as Rc<dyn Any>));
        let future_or_value = fetch_with_retry(fetcher.clone(), retrier)
            .boxed_local()
            .shared();

        let inner = Arc::new(RwLock::new(Inner {
            fetcher,
            cache_time,
            future_or_value,
            last_value: Default::default(),
            updated_at: Default::default(),
        }));

        Query { type_id, inner }
    }

    fn assert_type<T: 'static>(&self) {
        assert!(self.type_id == TypeId::of::<T>(), "type mismatch");
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

    pub fn is_fetching(&self) -> bool {
        self.inner.read().unwrap().future_or_value.peek().is_none()
    }

    pub fn last_value(&self) -> Option<Rc<dyn Any>> {
        self.inner.read().unwrap().last_value.clone()
    }

    pub async fn fetch<T: 'static>(&mut self, retrier: Option<Retryer>) -> Result<Rc<T>, Error> {
        self.assert_type::<T>();

        let fut = {
            let mut inner = self.inner.write().expect("failed to write in query");
            let fetcher = inner.fetcher.clone();
            let fut = fetch_with_retry(fetcher, retrier).boxed_local().shared();

            // Updates the inner future
            inner.future_or_value = fut.clone();
            fut
        };
        
        // Await and which updates the inner future
        let value = fut.await?;

        let ret = value
            .downcast::<T>()
            .map_err(|_| QueryError::type_mismatch::<T>())?;

        let mut inner = self.inner.write().expect("failed to write in query");
        inner.last_value = Some(ret.clone());
        inner.updated_at = Some(Instant::now());

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

    pub(crate) fn set_value(&mut self, value: Rc<dyn Any>) {
        assert!(self.type_id == value.type_id());

        let fut = ready(Ok(Rc::new(value) as Rc<dyn Any>))
            .boxed_local()
            .shared();

        // SAFETY: Value always is Ok(T)
        let value = futures::executor::block_on(fut.clone()).unwrap();
        debug_assert!(fut.peek().is_some());

        let mut inner = self.inner.write().expect("failed to write in query");
        inner.future_or_value = fut;
        inner.last_value = Some(value);
        inner.updated_at = Some(Instant::now());
    }
}
