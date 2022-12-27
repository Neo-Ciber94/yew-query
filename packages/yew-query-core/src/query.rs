use self::x::CacheFuture;

use super::{error::QueryError, fetcher::BoxFetcher};
use crate::{query::x::CacheFutureExt, Error};
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

pub type QueryCacheFuture = CacheFuture<LocalBoxFuture<'static, Result<Rc<dyn Any>, Error>>>;

/// Represents a query.
pub struct Query {
    fetcher: BoxFetcher<Rc<dyn Any>>,
    updated_at: Instant,
    cache_time: Option<Duration>,
    pub(crate) future_or_value: QueryCacheFuture,
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
            .field("is_stale", &self.is_stale())
            .finish()
    }
}

pub(crate) mod x {
    use futures::{future::Shared, Future, FutureExt};
    use pin_project_lite::pin_project;
    use std::{cell::RefCell, rc::Rc};

    pub trait CacheFutureExt: Future {
        fn cached(self) -> CacheFuture<Self>
        where
            Self: Sized,
            Self::Output: Clone,
        {
            CacheFuture::new(self)
        }

        fn cached_with_initial(self, initial_value: Option<Self::Output>) -> CacheFuture<Self>
        where
            Self: Sized,
            Self::Output: Clone,
        {
            CacheFuture::with_initial(self, initial_value)
        }
    }

    impl<F> CacheFutureExt for F where F: Future {}

    pin_project! {
        pub struct CacheFuture<Fut>
        where
            Fut: Future,
        {
            last_value: Rc<RefCell<Option<Fut::Output>>>,

            #[pin]
            future_or_output: Shared<Fut>,
        }
    }

    impl<Fut> CacheFuture<Fut>
    where
        Fut: Future,
        Fut::Output: Clone,
    {
        pub fn new(fut: Fut) -> Self {
            Self::with_initial(fut, None)
        }

        pub fn with_initial(fut: Fut, initial_value: Option<Fut::Output>) -> Self {
            CacheFuture {
                future_or_output: fut.shared(),
                last_value: Rc::new(RefCell::new(initial_value)),
            }
        }

        pub fn last_value(&self) -> Option<Fut::Output> {
            match self.last_value.borrow().as_ref() {
                Some(x) => Some(x.clone()),
                None => None,
            }
        }

        pub fn is_resolved(&self) -> bool {
            self.future_or_output.peek().is_some()
        }
    }

    impl<Fut> Clone for CacheFuture<Fut>
    where
        Fut: Future,
    {
        fn clone(&self) -> Self {
            Self {
                last_value: self.last_value.clone(),
                future_or_output: self.future_or_output.clone(),
            }
        }
    }

    impl<Fut> Future for CacheFuture<Fut>
    where
        Fut: Future,
        Fut::Output: Clone,
    {
        type Output = Fut::Output;

        fn poll(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            let this = self.project();
            match this.future_or_output.poll(cx) {
                std::task::Poll::Ready(x) => {
                    *this.last_value.borrow_mut() = Some(x.clone());
                    std::task::Poll::Ready(x)
                }
                std::task::Poll::Pending => std::task::Poll::Pending,
            }
        }
    }
}
