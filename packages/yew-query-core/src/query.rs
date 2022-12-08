use super::{error::QueryError, fetcher::BoxFetcher};
use crate::fetcher::InfiniteFetcher;
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

mod x {
    use std::{any::Any, cell::RefCell, marker::PhantomData, rc::Rc};

    use futures::Future;
    use wasm_bindgen_futures::spawn_local;
    use yew::virtual_dom::Key;
    use crate::{client::QueryClient, Error};

    enum QueryState {
        Idle,
        Loading,
        Ready,
        Error(Box<Error>),
    }

    struct QueryStatus<T> {
        state: QueryState,
        value: Option<Rc<T>>,
        is_fetching: bool,
    }

    impl<T> QueryStatus<T> {
        fn new(state: QueryState, is_fetching: bool, value: Option<Rc<T>>) -> Self {
            QueryStatus {
                state,
                value,
                is_fetching,
            }
        }
    }

    struct QueryObserver<T> {
        client: Rc<RefCell<QueryClient>>,
        _marker: PhantomData<T>,
    }

    impl<T> QueryObserver<T>
    where
        T: 'static,
    {
        fn query_value(&self, key: &Key) -> Option<Rc<T>> {
            let client = self.client.borrow();
            client.get_query_data(key).ok()
        }

        async fn observe<C, F, Fut, E>(&self, key: Key, fetch: F, callback: C)
        where
            C: Fn(QueryStatus<T>) + 'static,
            F: Fn() -> Fut + 'static,
            Fut: Future<Output = Result<T, E>> + 'static,
            E: Into<Error> + 'static,
        {
            {
                let client = self.client.borrow();
                if !client.contains_key(&key) {
                    callback(QueryStatus::new(QueryState::Loading, false, None));
                }
            }

            let client = self.client.clone();

            spawn_local(async move {
                let mut client = client.borrow_mut();
                let result = client.fetch_query(key, fetch).await;
                match result {
                    Ok(value) => {
                        // On ready
                        callback(QueryStatus::new(QueryState::Ready, false, Some(value)));
                    }
                    Err(err) => {
                        // On error
                        callback(QueryStatus::new(QueryState::Error(err.into()), false, None));
                    }
                }
            });
        }
    }
}


/*

fn _use_query(key: Key, f: F) {
    let client = use_query_client();
    let is_fetching = use_state(|| false);
    let query_state = use_state(||{
        let obs = QueryObserver::new(client);
        let value = obs.query_value(key);
        value
    });

    let fetch = {
        use_callback(|_| {
            let obs = QueryObserver::new(client);
            obs.observer(key, f, |observed| {
                let ObservedQuery { is_fetching, status, value } = observed;
                is_fetching.set(is_fetching);

                match status {
                    QueryState::Idle => {
                        query_state.set(QueryState::Idle);
                    },
                    QueryState::Loading => {
                        query_state.set(QueryState::Loading);
                    },
                    QueryState::Ready => {
                        query_state.set(QueryState::Ready);
                    },
                    QueryState::Error(err) => {
                        query_state.set(QueryState::Error(err));
                    }
                }
            });
        });
    };
}

*/