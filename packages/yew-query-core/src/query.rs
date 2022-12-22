use super::{error::QueryError, fetcher::BoxFetcher};
use instant::Instant;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    rc::Rc,
    time::Duration,
};

pub struct Query {
    pub(crate) fetcher: BoxFetcher<Rc<dyn Any>>,
    pub(crate) value: Rc<dyn Any>,
    pub(crate) updated_at: Instant,
    pub(crate) cache_time: Option<Duration>,
}

impl Query {
    pub fn is_stale(&self) -> bool {
        match self.cache_time {
            Some(cache_time) => {
                let now = Instant::now();
                (now - self.updated_at) >= cache_time
            }
            None => false,
        }
    }

    pub fn get(&self) -> Option<&Rc<dyn Any>> {
        if self.is_stale() {
            return None;
        }

        Some(&self.value)
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
            .field("cache_time", &self.cache_time)
            .finish()
    }
}
