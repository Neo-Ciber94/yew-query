use super::{error::QueryError, fetcher::Fetcher};
use instant::Instant;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    rc::Rc,
    time::Duration,
};

pub struct Query {
    pub(crate) fetcher: Fetcher<Rc<dyn Any>>,
    pub(crate) cache_value: Option<Rc<dyn Any>>,
    pub(crate) updated_at: Instant,
    pub(crate) type_id: TypeId,
}

impl Query {
    pub fn fetcher(&self) -> &Fetcher<Rc<dyn Any>> {
        &self.fetcher
    }

    pub fn updated_at(&self) -> &Instant {
        &self.updated_at
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn is_stale_by_time(&self, stale_time: Duration) -> bool {
        let now = Instant::now();
        (now - self.updated_at) >= stale_time
    }

    pub fn get_if_not_stale(&self, stale_time: Duration) -> Option<&Rc<dyn Any>> {
        if self.is_stale_by_time(stale_time) {
            None
        } else {
            self.cache_value.as_ref()
        }
    }

    pub(crate) fn set_value<T: 'static>(&mut self, value: T) -> Result<(), QueryError> {
        if self.type_id != TypeId::of::<T>() {
            return Err(QueryError::type_mismatch::<T>());
        }

        self.cache_value = Some(Rc::new(value));
        self.updated_at = Instant::now();
        Ok(())
    }
}

impl Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cache_value = if self.cache_value.is_some() {
            "Some(..)"
        } else {
            "None"
        };
        
        f.debug_struct("Query")
            .field("fetcher", &"Fetcher<_>")
            .field("cache_value", &cache_value)
            .field("updated_at", &self.updated_at)
            .field("type_id", &self.type_id)
            .finish()
    }
}
