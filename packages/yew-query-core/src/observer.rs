use futures::Future;
use prokio::spawn_local;
use std::{marker::PhantomData, rc::Rc};

use crate::{
    client::QueryClient,
    key::{Key, QueryKey},
    state::QueryState,
    Error, QueryChanged, QueryOptions,
};

/// An event emitted when executing a query.
pub struct QueryChangeEvent<T> {
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
    options: Option<QueryOptions>,
    key: QueryKey,
    _marker: PhantomData<T>,
}

impl<T> QueryObserver<T>
where
    T: 'static,
{
    /// Constructs a new observer for the given key.
    pub fn new(client: QueryClient, key: Key) -> Self {
        Self::with_options(client, key, None)
    }

    /// Constructs a new observer for the given key and `QueryOptions`.
    pub fn with_options(client: QueryClient, key: Key, options: Option<QueryOptions>) -> Self {
        let key = QueryKey::of::<T>(key);

        QueryObserver {
            client,
            key,
            options,
            _marker: PhantomData,
        }
    }

    /// Returns `true` if is fetching.
    pub fn is_fetching(&self) -> bool {
        let key = &self.key;
        self.client.is_fetching(key)
    }

    /// Returns `true` if the query is stale.
    pub fn is_stale(&self) -> bool {
        let key = &self.key;
        self.client.is_stale(key)
    }

    /// Returns the last value emitted.
    pub fn last_value(&self) -> Option<Rc<T>> {
        let key = &self.key;
        let value = self
            .client
            .get_query(key)
            .and_then(|x| x.last_value())
            .clone()
            .and_then(|x| x.downcast::<T>().ok());

        value
    }

    /// Returns the last state.
    pub fn last_state(&self) -> Option<QueryState> {
        let key = &self.key;
        let state = self.client.get_query(key).map(|q| q.state());
        state
    }

    /// Adds a callback for observing the given query.
    pub fn observe<F, Fut, E, C>(&self, fetch: F, callback: C)
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        E: Into<Error> + 'static,
        C: Fn(QueryChangeEvent<T>) + Clone + 'static,
    {
        let key = &self.key;

        {
            let client = self.client.clone();
            let state = client.get_query_state(key).unwrap_or(QueryState::Idle);
            let last_value = self.last_value();
            let is_fetching = client.is_fetching(key);

            // Set initial state
            callback(QueryChangeEvent {
                state,
                is_fetching,
                value: last_value,
            });
        }

        let key = key.clone();
        let client = self.client.clone();
        let options = self.options.clone();

        spawn_local(async move {
            let mut client = client;

            let on_change = {
                let callback = callback.clone();
                move |event: QueryChanged| {
                    let value = event.value.map(|x| x.downcast::<T>().unwrap());
                    callback(QueryChangeEvent {
                        state: event.state,
                        is_fetching: event.is_fetching,
                        value,
                    });
                }
            };

            let is_cached = !client.is_stale(&key);
            let ret = client
                .fetch_query_with_options_and_observe(
                    key,
                    fetch,
                    options.as_ref(),
                    Some(Rc::new(on_change)),
                )
                .await;

            // The `Query` will notify each state change, but while cache we will not receive any updates,
            // in that cache we notify the current state of the query from the observer
            if is_cached {
                match ret {
                    Ok(value) => callback(QueryChangeEvent {
                        state: QueryState::Ready,
                        is_fetching: false,
                        value: Some(value),
                    }),
                    Err(err) => callback(QueryChangeEvent {
                        state: QueryState::Failed(err.into()),
                        is_fetching: false,
                        value: None,
                    }),
                }
            }
        });
    }
}
