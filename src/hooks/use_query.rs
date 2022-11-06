use futures::Future;
use std::{
    cell::{Cell, RefCell},
    convert::Infallible,
    ops::Deref,
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;
use web_sys::{AbortController, AbortSignal};
use yew::{use_effect_with_deps, use_state, virtual_dom::Key, UseStateHandle};

use super::{
    common::{use_on_reconnect, use_on_window_focus},
    use_query_client::use_query_client,
};
use crate::core::{client::QueryClient, Error};

#[derive(Debug)]
pub enum QueryState<T> {
    Idle,
    Loading,
    Refetching,
    Ready(Rc<T>),
    Failed(Error),
}

impl<T> QueryState<T> {
    pub fn is_idle(&self) -> bool {
        matches!(&*self, QueryState::Idle)
    }

    pub fn is_loading(&self) -> bool {
        matches!(&*self, QueryState::Loading)
    }

    pub fn is_refetching(&self) -> bool {
        matches!(&*self, QueryState::Refetching)
    }

    pub fn is_ready(&self) -> bool {
        matches!(&*self, QueryState::Ready(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(&*self, QueryState::Failed(_))
    }
}

pub struct UseQueryOptions<Fut, T, E>
where
    Fut: Future<Output = Result<T, E>>,
    T: 'static,
    E: Into<Error> + 'static,
{
    fetch: Box<dyn Fn(AbortSignal) -> Fut>,
    initial_data: Option<T>,
    enabled: bool,
    refetch_on_reconnect: bool,
    refetch_on_window_focus: bool,
}

impl<Fut, T, E> UseQueryOptions<Fut, T, E>
where
    Fut: Future<Output = Result<T, E>>,
    T: 'static,
    E: Into<Error> + 'static,
{
    pub fn new_abortable<F>(fetch: F) -> Self
    where
        F: Fn(AbortSignal) -> Fut + 'static,
    {
        let fetch = Box::new(fetch);
        UseQueryOptions {
            fetch,
            initial_data: None,
            enabled: true,
            refetch_on_reconnect: true,
            refetch_on_window_focus: true,
        }
    }

    pub fn new<F>(fetch: F) -> Self
    where
        F: Fn() -> Fut + 'static,
    {
        Self::new_abortable(move |_| fetch())
    }

    pub fn initial_data(mut self, initial_data: T) -> Self {
        self.initial_data = Some(initial_data);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn refetch_on_reconnect(mut self, refetch_on_reconnect: bool) -> Self {
        self.refetch_on_reconnect = refetch_on_reconnect;
        self
    }

    pub fn refetch_on_window_focus(mut self, refetch_on_window_focus: bool) -> Self {
        self.refetch_on_window_focus = refetch_on_window_focus;
        self
    }
}

#[derive(Debug)]
pub struct UseQueryHandle<T> {
    key: Key,
    state: UseStateHandle<QueryState<T>>,
    client: Rc<RefCell<QueryClient>>,
    initial_data: Option<T>,
}

impl<T> UseQueryHandle<T> {
    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn data(&self) -> Option<&T> {
        match &*self.state {
            QueryState::Ready(x) => Some(x.as_ref()),
            QueryState::Loading | QueryState::Idle => self.initial_data.as_ref(),
            _ => None,
        }
    }

    pub fn error(&self) -> Option<&Error> {
        match &*self.state {
            QueryState::Failed(error) => Some(error),
            _ => None,
        }
    }

    pub fn refetch(&self) {
        self.state.set(QueryState::Refetching);
    }

    pub fn remove(&self) {
        self.client.borrow_mut().remove_query_data(&self.key);
    }
}

impl<T> Deref for UseQueryHandle<T> {
    type Target = QueryState<T>;

    fn deref(&self) -> &Self::Target {
        &*self.state
    }
}

pub fn use_query<F, Fut, K, T>(key: K, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = T> + 'static,
    K: Into<Key>,
    T: 'static,
{
    use_query_with_options(
        key,
        UseQueryOptions::new(move || {
            let fut = fetcher();
            async move {
                let ret = fut.await;
                Ok::<_, Infallible>(ret)
            }
        }),
    )
}

pub fn use_query_with_signal<F, Fut, K, T>(key: K, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn(AbortSignal) -> Fut + 'static,
    Fut: Future<Output = T> + 'static,
    K: Into<Key>,
    T: 'static,
{
    use_query_with_options(
        key,
        UseQueryOptions::new_abortable(move |signal| {
            let fut = fetcher(signal);
            async move {
                let ret = fut.await;
                Ok::<_, Infallible>(ret)
            }
        }),
    )
}

pub fn use_query_with_signal_and_failure<F, Fut, K, T, E>(key: K, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn(AbortSignal) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
    K: Into<Key>,
    E: Into<Error> + 'static,
    T: 'static,
{
    use_query_with_options(key, UseQueryOptions::new_abortable(fetcher))
}

pub fn use_query_with_options<Fut, K, T, E>(
    key: K,
    options: UseQueryOptions<Fut, T, E>,
) -> UseQueryHandle<T>
where
    Fut: Future<Output = Result<T, E>> + 'static,
    K: Into<Key>,
    T: 'static,
    E: Into<Error> + 'static,
{
    let UseQueryOptions {
        fetch,
        initial_data,
        enabled,
        refetch_on_reconnect,
        refetch_on_window_focus,
    } = options;

    let key = key.into();
    let client = use_query_client().expect("expected `QueryClient`");
    let state = use_state(|| QueryState::Idle);
    let query_state = std::mem::discriminant(&*state);
    let last_id = use_state(|| Cell::new(0_usize));

    {
        let state = state.clone();
        use_on_reconnect(move || {
            if !refetch_on_reconnect || !enabled || !state.is_loading() {
                return;
            }

            state.set(QueryState::Refetching);
        });
    }

    {
        let state = state.clone();
        use_on_window_focus(move || {
            if !refetch_on_window_focus || !enabled || !state.is_loading() {
                return;
            }

            state.set(QueryState::Refetching);
        });
    }

    {
        let state = state.clone();
        let client = client.clone();

        use_effect_with_deps(
            move |(key, enabled, _)| {
                let abort_controller = AbortController::new().expect("expected `AbortController`");
                let signal = abort_controller.signal();

                let cleanup = {
                    let state = state.clone();
                    move || {
                        if state.is_loading() {
                            abort_controller.abort();
                            state.set(QueryState::Idle);
                        }
                    }
                };

                if *enabled == false || !(state.is_idle() || state.is_refetching()) {
                    return cleanup;
                }

                state.set(QueryState::Loading);

                let id = last_id.get().wrapping_add(1);
                (*last_id).set(id);

                let key = key.clone();
                let enabled = *enabled;

                spawn_local(async move {
                    let state = state.clone();
                    let mut client = client.borrow_mut();
                    let result = client.fetch_query(key, move || fetch(signal.clone())).await;

                    if id == last_id.get() {
                        match result {
                            _ if !enabled => {
                                state.set(QueryState::Idle);
                            }
                            Ok(x) => {
                                state.set(QueryState::Ready(x));
                            }
                            Err(e) => {
                                state.set(QueryState::Failed(e));
                            }
                        }
                    }
                });

                cleanup
            },
            (key.clone(), enabled, query_state),
        );
    }

    UseQueryHandle {
        key,
        state,
        client,
        initial_data,
    }
}
