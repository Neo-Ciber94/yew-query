use futures::Future;
use prokio::spawn_local;
use std::{marker::PhantomData, rc::Rc};

use crate::{client::QueryClient, key::{QueryKey, Key}, state::QueryState, Error};

/// An event emitted when executing a query.
pub struct QueryEvent<T> {
    /// The state of a query.
    pub state: QueryState,

    /// Whether if is fetching the data.
    pub is_fetching: bool,

    /// The last value emitted.
    pub value: Option<Rc<T>>,
}

/// A mechanism for track the state of a query.
pub struct QueryObserver<T> {
    client: QueryClient,
    key: QueryKey,
    _marker: PhantomData<T>,
}

impl<T> QueryObserver<T>
where
    T: 'static,
{
    /// Constructs a new observer for the given key.
    pub fn new(client: QueryClient, key: Key) -> Self {
        let key = QueryKey::of::<T>(key);

        QueryObserver {
            client,
            key,
            _marker: PhantomData,
        }
    }

    /// Returns the last value emitted.
    pub fn get_last_value(&self) -> Option<Rc<T>> {
        let key = &self.key;
        let value = self.client.get_query_data(key);
        value.ok()
    }

    /// Adds a callback for observing the given query.
    pub fn observe<F, Fut, E, C>(&self, fetch: F, callback: C)
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        E: Into<Error> + 'static,
        C: Fn(QueryEvent<T>) + 'static,
    {
        let key = &self.key;

        {
            let client = self.client.clone();
            let state = client.get_query_state(key).unwrap_or(QueryState::Idle);
            let last_value = self.get_last_value();
            let is_fetching = client.is_fetching(key);

            // Set initial state
            callback(QueryEvent {
                state,
                is_fetching,
                value: last_value,
            });
        }

        let key = key.clone();
        let client = self.client.clone();

        spawn_local(async move {
            let mut client = client;
            let ret = client.fetch_query(key, fetch).await;

            match ret {
                Ok(value) => callback(QueryEvent {
                    state: QueryState::Ready,
                    is_fetching: false,
                    value: Some(value),
                }),
                Err(err) => callback(QueryEvent {
                    state: QueryState::Failed(err.into()),
                    is_fetching: false,
                    value: None,
                }),
            }
        });
    }
}
