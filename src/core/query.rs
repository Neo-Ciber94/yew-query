use std::{
    any::{Any, TypeId},
    time::{Duration, Instant},
};

use super::{error::QueryClientError, fetcher::Fetcher};

pub struct Query {
    pub(crate) fetcher: Fetcher<Box<dyn Any>>,
    pub(crate) cache_value: Option<Box<dyn Any>>,
    pub(crate) updated_at: Instant,
    pub(crate) type_id: TypeId,
}

impl Query {
    pub fn fetcher(&self) -> &Fetcher<Box<dyn Any>> {
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

    pub fn get_if_not_stale(&self, stale_time: Duration) -> Option<&Box<dyn Any>> {
        if self.is_stale_by_time(stale_time) {
            None
        } else {
            self.cache_value.as_ref()
        }
    }

    pub(crate) fn set_value<T: 'static>(&mut self, value: T) -> Result<(), QueryClientError> {
        if self.type_id != TypeId::of::<T>() {
            return Err(QueryClientError::type_mismatch::<T>());
        }

        self.cache_value = Some(Box::new(value));
        self.updated_at = Instant::now();
        Ok(())
    }
}
