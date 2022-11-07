use super::common::Callback;
use futures::Future;
use std::{
    cell::{Cell, RefCell},
    convert::Infallible,
    fmt::Debug,
    ops::Deref,
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;
use web_sys::AbortSignal;
use yew::{use_effect_with_deps, use_mut_ref, use_state, virtual_dom::Key, UseStateHandle};

use super::{
    common::{
        use_abort_controller, use_callback, use_is_first_render, use_on_reconnect,
        use_on_window_focus,
    },
    use_query_client::use_query_client,
};
use crate::core::{client::QueryClient, Error};

pub enum QueryState<T> {
    Idle,
    Loading,
    Ready(Rc<T>),
    Failed(Error),
}

impl<T> Debug for QueryState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Loading => write!(f, "Loading"),
            Self::Ready(_) => write!(f, "Ready"),
            Self::Failed(err) => write!(f, "Failed({err:?})"),
        }
    }
}

impl<T> QueryState<T> {
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
    key: Key,
    fetch: Rc<dyn Fn(AbortSignal) -> Fut>,
    enabled: bool,
    refetch_on_mount: bool,
    refetch_on_reconnect: bool,
    refetch_on_window_focus: bool,
}

impl<Fut, T, E> UseQueryOptions<Fut, T, E>
where
    Fut: Future<Output = Result<T, E>>,
    T: 'static,
    E: Into<Error> + 'static,
{
    pub fn new_abortable<K, F>(key: K, fetch: F) -> Self
    where
        F: Fn(AbortSignal) -> Fut + 'static,
        K: Into<Key>,
    {
        let fetch = Rc::new(fetch);
        let key = key.into();

        UseQueryOptions {
            key,
            fetch,
            // initial_data: None,
            // placeholder_data: None,
            enabled: true,
            refetch_on_mount: true,
            refetch_on_reconnect: true,
            refetch_on_window_focus: true,
        }
    }

    pub fn new<K, F>(key: K, fetch: F) -> Self
    where
        K: Into<Key>,
        F: Fn() -> Fut + 'static,
    {
        Self::new_abortable(key, move |_| fetch())
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn refetch_on_mount(mut self, refetch_on_mount: bool) -> Self {
        self.refetch_on_mount = refetch_on_mount;
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
struct Refetcher {
    callback: Callback<()>,
    is_refetching: UseStateHandle<bool>,
}

#[derive(Debug)]
pub struct UseQueryHandle<T> {
    key: Key,
    state: UseStateHandle<QueryState<T>>,
    client: Rc<RefCell<QueryClient>>,
    refetcher: Refetcher,
}

impl<T> UseQueryHandle<T> {
    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn data(&self) -> Option<&T> {
        match &*self.state {
            QueryState::Ready(x) => Some(x.as_ref()),
            _ => None,
        }
    }

    pub fn error(&self) -> Option<&Error> {
        match &*self.state {
            QueryState::Failed(error) => Some(error),
            _ => None,
        }
    }

    pub fn is_refetching(&self) -> bool {
        *self.refetcher.is_refetching
    }

    pub fn refetch(&self) {
        self.refetcher.callback.emit(());
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
    use_query_with_options(UseQueryOptions::new(key.into(), move || {
        let fut = fetcher();
        async move {
            let ret = fut.await;
            Ok::<_, Infallible>(ret)
        }
    }))
}

pub fn use_query_with_signal<F, Fut, K, T>(key: K, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn(AbortSignal) -> Fut + 'static,
    Fut: Future<Output = T> + 'static,
    K: Into<Key>,
    T: 'static,
{
    use_query_with_options(UseQueryOptions::new_abortable(key.into(), move |signal| {
        let fut = fetcher(signal);
        async move {
            let ret = fut.await;
            Ok::<_, Infallible>(ret)
        }
    }))
}

pub fn use_query_with_failure<F, Fut, K, T, E>(key: K, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
    K: Into<Key>,
    E: Into<Error> + 'static,
    T: 'static,
{
    use_query_with_options(UseQueryOptions::new(key.into(), fetcher))
}

pub fn use_query_with_signal_and_failure<F, Fut, K, T, E>(key: K, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn(AbortSignal) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
    K: Into<Key>,
    E: Into<Error> + 'static,
    T: 'static,
{
    use_query_with_options(UseQueryOptions::new_abortable(key.into(), fetcher))
}

pub fn use_query_with_options<Fut, T, E>(options: UseQueryOptions<Fut, T, E>) -> UseQueryHandle<T>
where
    Fut: Future<Output = Result<T, E>> + 'static,
    T: 'static,
    E: Into<Error> + 'static,
{
    let UseQueryOptions {
        key,
        fetch,
        enabled,
        refetch_on_mount,
        refetch_on_reconnect,
        refetch_on_window_focus,
    } = options;

    let client = use_query_client().expect("expected `QueryClient`");
    let state = use_state(|| QueryState::Idle);
    let refetching = use_state(|| false);
    let last_id = use_mut_ref(|| Cell::new(0_usize));
    let first_render = use_is_first_render();
    let abort_controller = use_abort_controller();

    let do_fetch = {
        let state = state.clone();
        let refetching = refetching.clone();
        let abort_controller = abort_controller.clone();
        let client = client.clone();

        use_callback(
            move |(), deps| {
                let state = state.clone();
                let refetching = refetching.clone();
                let fetch = fetch.clone();
                let client = client.clone();
                let last_id = last_id.clone();

                //
                let signal = abort_controller.signal();
                let key = deps.0.clone();
                let enabled = deps.1;

                if !enabled {
                    return;
                }

                state.set(QueryState::Loading);
                log::trace!("loading: {key}");

                let id = last_id.borrow().get().wrapping_add(1);
                last_id.borrow().set(id);

                spawn_local(async move {
                    log::trace!("fetching: {key}");

                    let mut client = client.borrow_mut();
                    let result = client
                        .fetch_query(key.clone(), move || fetch(signal.clone()))
                        .await;

                    if id == last_id.borrow().get() {
                        if *refetching {
                            refetching.set(false);
                        }

                        match result {
                            _ if !enabled => {
                                log::trace!("fetch disabled: {key}");
                                state.set(QueryState::Idle);
                            }
                            Ok(x) => {
                                log::trace!("fetch success: {key}");
                                state.set(QueryState::Ready(x));
                            }
                            Err(e) => {
                                log::trace!("fetch failed: {key}");
                                state.set(QueryState::Failed(e));
                            }
                        }
                    }
                });
            },
            (key.clone(), enabled),
        )
    };

    // Refetch
    let refetcher = {
        let is_refetching = refetching.clone();
        let callback = {
            let is_refetching = refetching.clone();
            let do_fetch = do_fetch.clone();
            
            Callback::from(move |_| {
                is_refetching.set(true);
                do_fetch.emit(());
            })
        };

        Refetcher {
            callback,
            is_refetching,
        }
    };

    // On mount
    {
        let do_fetch = do_fetch.clone();
        let key = key.clone();

        use_effect_with_deps(
            move |_| {
                if !first_render && refetch_on_mount {
                    log::trace!("refetching on mount: {key}");
                    do_fetch.emit(());
                }

                || ()
            },
            (),
        )
    }

    // On reconnection
    {
        let do_fetch = do_fetch.clone();
        let key = key.clone();

        use_on_reconnect(move || {
            if refetch_on_reconnect {
                log::trace!("refetch on reconnect: {key}");
                do_fetch.emit(());
            }
        });
    }

    // On window focus
    {
        let do_fetch = do_fetch.clone();
        let key = key.clone();

        use_on_window_focus(move || {
            if refetch_on_window_focus {
                log::trace!("refetch on window focus: {key}");
                do_fetch.emit(());
            }
        });
    }

    // Fetch
    {
        let do_fetch = do_fetch.clone();

        use_effect_with_deps(
            move |_| {
                do_fetch.emit(());

                move || {
                    log::trace!("unmount");
                    abort_controller.abort();
                }
            },
            (),
        );
    }

    UseQueryHandle {
        key,
        state,
        client,
        refetcher,
    }
}
