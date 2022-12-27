use super::{error::QueryError, fetcher::BoxFetcher};
use crate::Error;
use futures::{
    future::{ready, LocalBoxFuture, Shared},
    FutureExt,
};
use instant::Instant;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    rc::Rc,
    time::Duration,
};

pub type QuerySharedFuture = Shared<LocalBoxFuture<'static, Result<Rc<dyn Any>, Error>>>;

/// Represents a query.
pub struct Query {
    fetcher: BoxFetcher<Rc<dyn Any>>,
    updated_at: Instant,
    cache_time: Option<Duration>,
    future_or_value: QuerySharedFuture,
    type_id: TypeId,
}

impl Query {
    pub(crate) fn new<T: 'static>(
        future_or_value: QuerySharedFuture,
        fetcher: BoxFetcher<Rc<dyn Any>>,
        cache_time: Option<Duration>,
    ) -> Self {
        let updated_at = Instant::now();
        let type_id = TypeId::of::<T>();

        Query {
            future_or_value,
            fetcher,
            cache_time,
            updated_at,
            type_id,
        }
    }

    /// Returns the `TypeId` associated to this query value.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns the fetcher used to fetch the value of this query.
    pub fn fetcher(&self) -> &BoxFetcher<Rc<dyn Any>> {
        &self.fetcher
    }

    /// Returns the result of this query.
    ///
    /// # Returns
    /// - `Some(Ok(..))` if the query resolved with a successful result.
    /// - `Some(Err(..))` if the query resolved with an error.
    /// - `None` if the query had not resolved yet.
    pub fn value(&self) -> Option<Rc<dyn Any>> {
        match self.future_or_value.peek() {
            Some(x) => {
                let value = x.clone().expect("the query returned an error");
                Some(value)
            }
            _ => None,
        }
    }

    /// Returns a future that resolve to this query value.
    pub async fn resolve<T: 'static>(&self) -> Result<Rc<T>, Error> {
        if self.type_id != TypeId::of::<T>() {
            return Err(Error::new(QueryError::type_mismatch::<T>()));
        }

        let value = self.future_or_value.clone().await?;
        let ret = value.downcast::<T>().unwrap();
        Ok(ret)
    }

    /// Returns `true` if the future of this query had resolved.
    pub fn is_resolved(&self) -> bool {
        self.future_or_value.peek().is_none()
    }

    /// Returns `true` if the value of the query is expired.
    pub fn is_stale(&self) -> bool {
        if !self.is_resolved() {
            return false;
        }

        match self.cache_time {
            Some(cache_time) => {
                let now = Instant::now();
                (now - self.updated_at) >= cache_time
            }
            None => false,
        }
    }

    pub(crate) fn set_future(&mut self, fut: QuerySharedFuture) {
        self.future_or_value = fut;
    }

    pub(crate) fn set_value(&mut self, value: Rc<dyn Any>) {
        assert!(self.type_id == value.type_id());

        let fut = ready(Ok(Rc::new(value) as Rc<dyn Any>))
            .boxed_local()
            .shared();
        let _ = futures::executor::block_on(fut.clone());

        debug_assert!(fut.peek().is_some());

        self.future_or_value = fut;
        self.updated_at = Instant::now();
    }
}

impl Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query")
            .field("fetcher", &"Fetcher<_>")
            .field("future_or_value", &"Shared<_>")
            .field("updated_at", &self.updated_at)
            .field("cache_time", &self.cache_time)
            .finish()
    }
}
