use std::{cell::Cell, convert::Infallible, ops::Deref, rc::Rc};
use futures::Future;
use wasm_bindgen_futures::spawn_local;
use web_sys::{AbortController, AbortSignal};
use yew::{use_effect_with_deps, use_state, virtual_dom::Key, UseStateHandle};

use crate::core::Error;

use super::use_query_client::use_query_client;

pub enum QueryState<T> {
    Idle,
    Loading,
    Ready(Rc<T>),
    Failed(Error),
}

impl<T> QueryState<T> {
    pub fn value(&self) -> Option<&T> {
        match &*self {
            QueryState::Ready(x) => Some(x.as_ref()),
            _ => None,
        }
    }

    pub fn error(&self) -> Option<&Error> {
        match &*self {
            QueryState::Failed(x) => Some(x),
            _ => None,
        }
    }

    pub fn is_idle(&self) -> bool {
        matches!(&*self, QueryState::Idle)
    }

    pub fn is_loading(&self) -> bool {
        matches!(&*self, QueryState::Loading)
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
    disabled: bool,
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
            disabled: false,
        }
    }

    pub fn new<F>(fetch: F) -> Self
    where
        F: Fn() -> Fut + 'static,
    {
        let fetch = Box::new(move |_| fetch());
        UseQueryOptions {
            fetch,
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub struct UseQueryHandle<T> {
    state: UseStateHandle<QueryState<T>>,
}

impl<T> UseQueryHandle<T> {
    pub fn refetch(&self) {
        todo!()
    }

    pub fn is_cancelled(&self) -> bool {
        todo!()
    }
}

impl<T> Deref for UseQueryHandle<T> {
    type Target = QueryState<T>;

    fn deref(&self) -> &Self::Target {
        &*self.state
    }
}

pub fn use_query<F, Fut, T>(key: Key, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = T> + 'static,
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

pub fn use_query_with_signal<F, Fut, T>(key: Key, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn(AbortSignal) -> Fut + 'static,
    Fut: Future<Output = T> + 'static,
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

pub fn use_query_with_options<Fut, T, E>(
    key: Key,
    options: UseQueryOptions<Fut, T, E>,
) -> UseQueryHandle<T>
where
    Fut: Future<Output = Result<T, E>> + 'static,
    T: 'static,
    E: Into<Error> + 'static,
{
    let UseQueryOptions { disabled, fetch } = options;
    let client = use_query_client().expect("expected `QueryClient`");
    let state = use_state(|| QueryState::Idle);
    let last_id = use_state(|| Cell::new(0_usize));

    {
        let state = state.clone();
        use_effect_with_deps(
            move |(k, disabled)| {
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

                if *disabled {
                    return cleanup;
                }

                state.set(QueryState::Loading);

                let id = last_id.get().wrapping_add(1);
                (*last_id).set(id);

                let key = k.clone();
                let disabled = *disabled;

                spawn_local(async move {
                    let state = state.clone();
                    let mut client = client.borrow_mut();
                    let result = client.fetch_query(key, move || fetch(signal.clone())).await;

                    if id == last_id.get() {
                        match result {
                            _ if disabled => {
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
            (key.clone(), disabled),
        );
    }

    UseQueryHandle { state }
}
