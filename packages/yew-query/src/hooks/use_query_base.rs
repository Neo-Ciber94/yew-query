use super::use_query_client;
use crate::common::{
    use_abort_controller, use_is_first_render, use_on_online, use_on_window_focus,
};
use futures::Future;
use std::rc::Rc;
use web_sys::AbortSignal;
use yew::virtual_dom::Key;
use yew::{
    hook, use_callback, use_effect_with_deps, use_state, Callback,
    UseStateHandle,
};
use yew_query_core::observer::QueryEvent;
use yew_query_core::{
    observer::{QueryObserver, QueryState},
    Error,
};

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

pub struct UseQueryHandle<T> {
    key: Key,
    fetch: Callback<()>,
    is_fetching: bool,
    state: UseStateHandle<QueryState>,
    value: UseStateHandle<Option<Rc<T>>>,
}

impl<T> UseQueryHandle<T> {
    pub fn data(&self) -> Option<&T> {
        self.value.as_deref()
    }

    pub fn error(&self) -> Option<&Error> {
        match &*self.state {
            QueryState::Failed(err) => Some(err),
            _ => None,
        }
    }

    pub fn state(&self) -> &QueryState {
        &self.state
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.state(), QueryState::Idle)
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.state(), QueryState::Loading)
    }

    pub fn is_fetching(&self) -> bool {
        self.is_fetching
    }

    pub fn is_error(&self) -> bool {
        matches!(self.state(), QueryState::Failed(_))
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.state(), QueryState::Ready)
    }
}

#[hook]
pub fn use_query_base<F, Fut, K, T, E>(key: K, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
    K: Into<Key>,
    T: 'static,
    E: Into<Error> + 'static,
{
    use_query_base_with_options(UseQueryOptions::new(key.into(), fetcher))
}

#[hook]
pub fn use_query_base_with_signal<F, Fut, K, T, E>(key: K, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn(AbortSignal) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
    K: Into<Key>,
    T: 'static,
    E: Into<Error> + 'static,
{
    use_query_base_with_options(UseQueryOptions::new_abortable(key.into(), fetcher))
}

#[hook]
pub fn use_query_base_with_options<Fut, T, E>(
    options: UseQueryOptions<Fut, T, E>,
) -> UseQueryHandle<T>
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

    let client = use_query_client().expect("expected QueryClient");
    let abort_controller = use_abort_controller();
    let observer = QueryObserver::<T>::new(client.clone(), key.clone());
    let first_render = use_is_first_render();
    let query_fetching = use_state(|| false);
    let query_state = use_state(|| QueryState::Idle);
    let query_value = {
        let last_value = observer.get_last_value();
        use_state(move || last_value)
    };

    let latest_id = use_state(|| std::cell::Cell::new(0_u32));

    let do_fetch = {
        let query_value = query_value.clone();
        let query_state = query_state.clone();
        let query_fetching = query_fetching.clone();
        let fetch = fetch.clone();
        let latest_id = latest_id.clone();
        let abort_controller = abort_controller.clone();

        use_callback(
            move |(), deps| {
                let enabled = deps.0;
                let self_id = latest_id.get().wrapping_add(1);
                (*latest_id).set(self_id);

                let query_value = query_value.clone();
                let query_state = query_state.clone();
                let query_fetching = query_fetching.clone();
                let latest_id = latest_id.clone();
                
                let signal = abort_controller.signal();
                let fetch = fetch.clone();
                let f = move || fetch(signal.clone());

                observer.observe(f, move |event| {
                    if !enabled {
                        return;
                    }

                    let QueryEvent {
                        state,
                        value,
                        is_fetching,
                    } = event;

                    if latest_id.get() == self_id {
                        query_value.set(value);
                        query_state.set(state);
                        query_fetching.set(is_fetching);
                    }
                });
            },
            (enabled, key.clone()),
        )
    };

    // Check enabled
    {
        let query_state = query_state.clone();
        use_effect_with_deps(
            move |enabled| {
                if !enabled {
                    query_state.set(QueryState::Idle);
                }
            },
            enabled,
        );
    }

    // First fetch
    {
        let do_fetch = do_fetch.clone();
        let key = key.clone();
        use_effect_with_deps(
            move |_| {
                log::trace!("fetching: {key}");
                do_fetch.emit(());

                move || {
                    log::trace!("unmount");
                    abort_controller.abort();
                }
            },
            (),
        );
    }

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
            },
            (),
        )
    }

    // On online
    {
        let do_fetch = do_fetch.clone();
        let key = key.clone();

        use_on_online(move || {
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

    UseQueryHandle {
        key,
        fetch: do_fetch.clone(),
        state: query_state.clone(),
        value: query_value.clone(),
        is_fetching: *query_fetching,
    }
}
