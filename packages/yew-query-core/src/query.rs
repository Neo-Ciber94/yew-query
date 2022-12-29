use super::{error::QueryError, fetcher::BoxFetcher};
use crate::{
    futures::cache::{CacheFutureExt, Cached},
    Error,
};
use futures::{
    future::{ready, LocalBoxFuture},
    FutureExt,
};
use instant::Instant;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    rc::Rc,
    time::Duration,
};

pub(crate) type QueryCacheFuture = Cached<LocalBoxFuture<'static, Result<Rc<dyn Any>, Error>>>;

/// Represents a query.
pub struct Query {
    pub(crate) future_or_value: QueryCacheFuture,
    fetcher: BoxFetcher<Rc<dyn Any>>,
    updated_at: Instant,
    cache_time: Option<Duration>,
    type_id: TypeId,
}

impl Query {
    pub(crate) fn new<T: 'static>(
        future_or_value: QueryCacheFuture,
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
        match self.future_or_value.last_value() {
            Some(x) => {
                let value = x.expect("the query returned an error");
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
        self.future_or_value.is_resolved()
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

    pub(crate) fn set_future(&mut self, fut: QueryCacheFuture) {
        self.future_or_value = fut;
    }

    pub(crate) fn set_value(&mut self, value: Rc<dyn Any>) {
        assert!(self.type_id == value.type_id());

        let fut = ready(Ok(Rc::new(value) as Rc<dyn Any>))
            .boxed_local()
            .cached();
        let _ = futures::executor::block_on(fut.clone());

        debug_assert!(fut.last_value().is_some());

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
            .field("type_id", &self.type_id)
            .finish()
    }
}

mod x {
    use crate::{
        client::fetch_with_retry, error::QueryError, fetcher::BoxFetcher, retry::Retryer, Error,
    };
    use futures::{
        future::{LocalBoxFuture, Shared},
        Future, FutureExt, TryFutureExt,
    };
    use instant::{Duration, Instant};
    use std::{
        any::{Any, TypeId},
        rc::Rc,
    };

    struct Query {
        type_id: TypeId,
        fetcher: BoxFetcher<Rc<dyn Any>>,
        cache_time: Option<Duration>,
        updated_at: Option<Instant>,
        last_value: Option<Rc<dyn Any>>,
        future_or_value: Option<Shared<LocalBoxFuture<'static, Result<Rc<dyn Any>, Error>>>>,
    }

    impl Query {
        pub fn new<F, Fut, T, E>(f: F, cache_time: Option<Duration>) -> Self
        where
            F: Fn() -> Fut + 'static,
            Fut: Future<Output = Result<T, E>> + 'static,
            T: 'static,
            E: Into<Error> + 'static,
        {
            let fetcher = BoxFetcher::new(move || f().map_ok(|x| Rc::new(x) as Rc<dyn Any>));
            let type_id = TypeId::of::<T>();

            Query {
                fetcher,
                type_id,
                cache_time,
                last_value: None,
                future_or_value: None,
                updated_at: None
            }
        }

        fn assert_type<T: 'static>(&self) {
            assert!(self.type_id == TypeId::of::<T>(), "type mismatch");
        }

        pub fn is_fetching(&self) -> bool {
            match &self.future_or_value {
                Some(x) => x.peek().is_none(),
                None => false,
            }
        }

        pub fn last_value<T: 'static>(&self) -> Option<Rc<T>> {
            self.assert_type::<T>();
            
            match &self.last_value {
                Some(x) => x.clone().downcast::<T>().ok(),
                None => None,
            }
        }

        pub async fn fetch<T: 'static>(&mut self, retrier: Option<Retryer>) -> Result<Rc<T>, Error> {
            self.assert_type::<T>();

            let fetcher = self.fetcher.clone();
            let fut = fetch_with_retry(fetcher, retrier).boxed_local().shared();
            let value = fut.await?;
            let ret = value
                .downcast::<T>()
                .map_err(|_| QueryError::type_mismatch::<T>())?;

            self.last_value = Some(ret.clone());
            self.updated_at = Some(Instant::now());

            Ok(ret)
        }
    }
}
