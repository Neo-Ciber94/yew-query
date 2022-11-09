use super::query::Query;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use yew::virtual_dom::Key;

/// Provides a way to cache data.
pub trait QueryCache: Debug {
    /// Returns the cache entry with the given key.
    fn get(&self, key: &Key) -> Option<&Query>;

    /// Returns a mutable reference to the cache entry with the given key.
    fn get_mut(&mut self, key: &Key) -> Option<&mut Query>;

    /// Sets a cache entry with the given key.
    fn set(&mut self, key: Key, entry: Query);

    /// Removes and returns the cache entry with the given key.
    fn remove(&mut self, key: &Key) -> Option<Query>;

    /// Returns `true` if the given key is in the cache.
    fn has(&self, key: &Key) -> bool;

    /// Removes all the cache entries.
    fn clear(&mut self);
}

impl QueryCache for HashMap<Key, Query> {
    fn get(&self, key: &Key) -> Option<&Query> {
        self.get(&key)
    }

    fn get_mut(&mut self, key: &Key) -> Option<&mut Query> {
        self.get_mut(&key)
    }

    fn set(&mut self, key: Key, entry: Query) {
        self.insert(key, entry);
    }

    fn remove(&mut self, key: &Key) -> Option<Query> {
        self.remove(key)
    }

    fn has(&self, key: &Key) -> bool {
        self.contains_key(key)
    }

    fn clear(&mut self) {
        self.clear()
    }
}

impl QueryCache for BTreeMap<Key, Query> {
    fn get(&self, key: &Key) -> Option<&Query> {
        self.get(&key)
    }

    fn get_mut(&mut self, key: &Key) -> Option<&mut Query> {
        self.get_mut(&key)
    }

    fn set(&mut self, key: Key, entry: Query) {
        self.insert(key, entry);
    }

    fn remove(&mut self, key: &Key) -> Option<Query> {
        self.remove(key)
    }

    fn has(&self, key: &Key) -> bool {
        self.contains_key(key)
    }

    fn clear(&mut self) {
        self.clear()
    }
}