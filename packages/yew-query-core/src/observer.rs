use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use futures::Future;
use wasm_bindgen_futures::spawn_local;
use yew::virtual_dom::Key;

use crate::{client::QueryClient, Error};

pub enum QueryState {
    Idle,
    Loading,
    Ready,
    Failed(Error),
}

pub struct QueryEvent<T> {
    pub state: QueryState,
    pub is_fetching: bool,
    pub value: Option<Rc<T>>,
}

pub struct QueryObserver<T> {
    client: Rc<RefCell<QueryClient>>,
    key: Key,
    _marker: PhantomData<T>,
}

impl<T> QueryObserver<T>
where
    T: 'static,
{
    pub fn new(client: Rc<RefCell<QueryClient>>, key: Key) -> Self {
        QueryObserver {
            client,
            key,
            _marker: PhantomData,
        }
    }

    pub fn get_last_value(&self) -> Option<Rc<T>> {
        let key = &self.key;
        let client = self.client.borrow();
        let value = client.get_query_data(key);
        value.ok()
    }

    pub fn observe<F, Fut, E, C>(&self, fetch: F, callback: C)
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
        E: Into<Error> + 'static,
        C: Fn(QueryEvent<T>) + 'static,
    {
        let key = &self.key;
        let client = self.client.borrow();
        let is_cached = client.contains_key(key);

        // If the value is cached and still fresh return
        if is_cached && !client.is_stale(key) {
            log::trace!("{key} is cached");
            let value = self.get_last_value();
            debug_assert!(value.is_some());

            callback(QueryEvent {
                state: QueryState::Ready,
                is_fetching: false,
                value,
            });
            return;
        }

        // If value is not in cache we set the loading state

        if is_cached {
            callback(QueryEvent {
                state: QueryState::Idle,
                is_fetching: true,
                value: None,
            });
        } else {
            callback(QueryEvent {
                state: QueryState::Loading,
                is_fetching: true,
                value: None,
            });
        }

        let client = self.client.clone();
        let key = key.clone();

        spawn_local(async move {
            let mut client = client.borrow_mut();
            let ret = client.fetch_query(key, fetch).await;
            log::trace!("fetching...");

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
