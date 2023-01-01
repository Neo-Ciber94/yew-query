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
    time::Duration, cell::{Cell, RefCell},
};

// pub(crate) type QueryCacheFuture = Cached<LocalBoxFuture<'static, Result<Rc<dyn Any>, Error>>>;

/// Represents a query.
#[derive(Clone)]
pub struct Query {
    type_id: TypeId,
    fetcher: BoxFetcher<Rc<dyn Any>>,
    cache_time: Option<Duration>,
    updated_at: Rc<Cell<Option<Instant>>>,
    last_value: Rc<RefCell<Option<Rc<dyn Any>>>>,
    future_or_value: Shared<LocalBoxFuture<'static, Result<Rc<dyn Any>, Error>>>,
}

impl Query {
    pub fn new<F, Fut, T, E>(f: F, retrier: Option<Retryer>, cache_time: Option<Duration>) -> Self
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        T: 'static,
        E: Into<Error> + 'static,
    {
        let fetcher = BoxFetcher::new(move || f().map_ok(|x| Rc::new(x) as Rc<dyn Any>));
        let future_or_value = fetch_with_retry(fetcher.clone(), retrier).boxed_local().shared();
        let type_id = TypeId::of::<T>();

        Query {
            fetcher,
            type_id,
            cache_time,
            future_or_value,
            last_value: Default::default(),
            updated_at: Default::default(),
        }
    }

    fn assert_type<T: 'static>(&self) {
        assert!(self.type_id == TypeId::of::<T>(), "type mismatch");
    }

    /// Returns a future that resolve to this query value.
    pub async fn future<T: 'static>(&self) -> Result<Rc<T>, Error> {
        if self.type_id != TypeId::of::<T>() {
            return Err(Error::new(QueryError::type_mismatch::<T>()));
        }

        let fut = self.future_or_value.clone();
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
        self.future_or_value.peek().is_none()
    }

    pub fn last_value(&self) -> Option<Rc<dyn Any>> {
        self.last_value.borrow().clone()
    }

    pub async fn fetch<T: 'static>(&mut self, retrier: Option<Retryer>) -> Result<Rc<T>, Error> {
        self.assert_type::<T>();

        let fetcher = self.fetcher.clone();
        let fut = fetch_with_retry(fetcher, retrier).boxed_local().shared();

        // Updates the inner future
        self.future_or_value = fut.clone();

        log::trace!("Query::fetch() START");

        // Await and which updates the inner future
        let value = fut.await?;

        log::trace!("Query::fetch() END");

        let ret = value
            .downcast::<T>()
            .map_err(|_| QueryError::type_mismatch::<T>())?;

        *self.last_value.borrow_mut() = Some(ret.clone());
        self.updated_at.set(Some(Instant::now()));

        Ok(ret)
    }

    /// Returns `true` if the value of the query is expired.
    pub fn is_stale(&self) -> bool {
        let Some(updated_at) = self.updated_at.get() else {
            return false;
        };

        match self.cache_time {
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

        self.future_or_value = fut;
        *self.last_value.borrow_mut() = Some(value);
        self.updated_at.set(Some(Instant::now()));
    }
}

impl Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query")
            .field("fetcher", &"Fetcher<_>")
            .field("future_or_value", &"Shared<_>")
            .field("updated_at", &self.updated_at)
            .field("cache_time", &self.cache_time)
            .field("type_id", &self.type_id)
            .field("is_stale", &self.is_stale())
            .finish()
    }
}
