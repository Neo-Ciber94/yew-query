use crate::{fetcher::InfiniteFetcher, timeout::Timeout};

use super::{error::QueryError, fetcher::BoxFetcher};
use instant::Instant;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt::Debug,
    rc::Rc,
    time::Duration,
};

pub(crate) struct SingleQuery {
    data: Rc<dyn Any>,
    fetcher: BoxFetcher<Rc<dyn Any>>,
}

pub(crate) struct InfiniteQuery {
    data: Rc<RefCell<Box<dyn Any>>>,
    fetcher: InfiniteFetcher<Box<dyn Any>>,
}

pub(crate) enum QueryData {
    Single(SingleQuery),
    Infinite(InfiniteQuery),
}

pub struct Query {
    pub(crate) fetcher: BoxFetcher<Rc<dyn Any>>,
    pub(crate) value: Rc<dyn Any>,
    pub(crate) updated_at: Instant,
    pub(crate) timeout: Option<Timeout>,
}

impl Query {
    pub fn is_stale_by_time(&self, stale_time: Duration) -> bool {
        let now = Instant::now();
        (now - self.updated_at) >= stale_time
    }

    pub fn get_if_not_stale(&self, stale_time: Duration) -> Option<&Rc<dyn Any>> {
        if self.is_stale_by_time(stale_time) {
            None
        } else {
            Some(&self.value)
        }
    }

    pub(crate) fn set_value<T: 'static>(&mut self, value: T) -> Result<(), QueryError> {
        if self.value.type_id() != TypeId::of::<T>() {
            return Err(QueryError::type_mismatch::<T>());
        }

        self.value = Rc::new(value);
        self.updated_at = Instant::now();
        Ok(())
    }
}

impl Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query")
            .field("fetcher", &"Fetcher<_>")
            .field("cache_value", &"Rc<_>")
            .field("updated_at", &self.updated_at)
            .finish()
    }
}
