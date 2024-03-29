use super::use_query_client;
use crate::{
    common::{use_abort_controller, use_is_first_render, use_on_online, use_on_window_focus},
    utils::{id::Id, OptionExt},
};
use futures::Future;
use instant::Duration;
use std::rc::Rc;
use web_sys::AbortSignal;
use yew::{hook, use_callback, use_effect_with_deps, use_state, Callback, UseStateHandle, use_memo};
use yew_query_core::{
    Error, Key, QueryChangeEvent, QueryKey, QueryObserver, QueryOptions, QueryState, ObserveTarget,
};

/// Options for a `use_query`.
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
    options: Option<QueryOptions>,
}

impl<Fut, T, E> UseQueryOptions<Fut, T, E>
where
    Fut: Future<Output = Result<T, E>>,
    T: 'static,
    E: Into<Error> + 'static,
{
    /// Constructs a new `UseQueryOptions` with an abort signal.
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
            options: None,
        }
    }

    /// Constructs a new `UseQueryOptions`.
    pub fn new<K, F>(key: K, fetch: F) -> Self
    where
        K: Into<Key>,
        F: Fn() -> Fut + 'static,
    {
        Self::new_abortable(key, move |_| fetch())
    }

    /// Sets the cache duration for this specific query.
    pub fn cache_time(mut self, cache_time: Duration) -> Self {
        self.options.get_or_insert_with(Default::default);
        self.options.update(move |opts| opts.cache_time(cache_time));

        self
    }

    /// Sets the refetch time interval for this specific query.
    pub fn refetch_time(mut self, refetch_time: Duration) -> Self {
        self.options.get_or_insert_with(Default::default);
        self.options
            .update(move |opts| opts.refetch_time(refetch_time));
        self
    }

    /// Sets the function used to retry on failure.
    pub fn retry<F, I>(mut self, retry: F) -> Self
    where
        F: Fn() -> I + 'static,
        I: Iterator<Item = Duration> + 'static,
    {
        self.options.get_or_insert_with(Default::default);
        self.options.update(move |opts| opts.retry(retry));
        self
    }

    /// Sets a value for enable for disable this query.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets a value indicating whether if refetch the data on mount.
    pub fn refetch_on_mount(mut self, refetch_on_mount: bool) -> Self {
        self.refetch_on_mount = refetch_on_mount;
        self
    }

    /// Sets a value indicating whether if refetch on reconnection.
    pub fn refetch_on_reconnect(mut self, refetch_on_reconnect: bool) -> Self {
        self.refetch_on_reconnect = refetch_on_reconnect;
        self
    }

    /// Sets a value indicating whether if refetch when window is focused.
    pub fn refetch_on_window_focus(mut self, refetch_on_window_focus: bool) -> Self {
        self.refetch_on_window_focus = refetch_on_window_focus;
        self
    }
}

/// Handle returned by `use_query`.
pub struct UseQueryHandle<T> {
    id: Id,
    key: QueryKey,
    fetch: Callback<ObserveTarget>,
    remove: Callback<()>,
    is_fetching: UseStateHandle<bool>,
    state: UseStateHandle<QueryState>,
    value: UseStateHandle<Option<Rc<T>>>,
}

impl<T> UseQueryHandle<T> {
    pub fn id(&self) -> Id {
        self.id
    }

    /// Returns the currently available data.
    pub fn data(&self) -> Option<&T> {
        self.value.as_deref()
    }

    /// Returns a error that ocurred during the fetching, if any.
    pub fn error(&self) -> Option<&Error> {
        match &*self.state {
            QueryState::Failed(err) => Some(err),
            _ => None,
        }
    }

    /// Returns the current state of the query.
    pub fn state(&self) -> &QueryState {
        &self.state
    }

    /// Returns the key used to identify the query.
    pub fn key(&self) -> &QueryKey {
        &self.key
    }

    /// Returns `true` if the query is idle.
    pub fn is_idle(&self) -> bool {
        matches!(self.state(), QueryState::Idle)
    }

    /// Returns `true` if the query has no data and is loading.
    pub fn is_loading(&self) -> bool {
        matches!(self.state(), QueryState::Loading)
    }

    /// Returns `true` if is fetching data.
    pub fn is_fetching(&self) -> bool {
        *self.is_fetching
    }

    /// Returns `true` if has an error.
    pub fn is_error(&self) -> bool {
        matches!(self.state(), QueryState::Failed(_))
    }

    /// Returns `true` if the data is available.
    pub fn is_ready(&self) -> bool {
        matches!(self.state(), QueryState::Ready)
    }

    /// Returns `true` if the query finished with either an error or value.
    pub fn is_completed(&self) -> bool {
        self.is_ready() || self.is_error()
    }

    /// Refetch ths data.
    pub fn refetch(&self) {
        self.fetch.emit(ObserveTarget::Refetch);
    }

    /// Removes the query data.
    pub fn remove(&self) {
        self.remove.emit(());
    }
}

impl<T> Clone for UseQueryHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            key: self.key.clone(),
            fetch: self.fetch.clone(),
            remove: self.remove.clone(),
            is_fetching: self.is_fetching.clone(),
            state: self.state.clone(),
            value: self.value.clone(),
        }
    }
}

/// This hook allows to observe the result and state of a future.
#[hook]
pub fn use_query<F, Fut, K, T, E>(key: K, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
    K: Into<Key>,
    T: 'static,
    E: Into<Error> + 'static,
{
    use_query_with_options(UseQueryOptions::new(key.into(), fetcher))
}

/// This hook allows to observe the result and state of a future with a abort signal.
#[hook]
pub fn use_query_with_signal<F, Fut, K, T, E>(key: K, fetcher: F) -> UseQueryHandle<T>
where
    F: Fn(AbortSignal) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
    K: Into<Key>,
    T: 'static,
    E: Into<Error> + 'static,
{
    use_query_with_options(UseQueryOptions::new_abortable(key.into(), fetcher))
}

/// This hook allows to observe the result and state of a future using the given `UseQueryOptions`.
#[hook]
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
        options,
    } = options;

    let id = *use_memo(|_| Id::next(), ());
    let client = use_query_client().expect("expected QueryClient");
    let abort_controller = use_abort_controller();
    let observer =
        use_state(|| QueryObserver::<T>::with_options(client.clone(), key.clone(), options));
    let first_render = use_is_first_render();
    let query_key = QueryKey::of::<T>(key.clone());

    let query_fetching = {
        let is_fetching = observer.is_fetching();
        use_state(|| is_fetching)
    };

    let query_state = {
        let last_state = observer.last_state();
        use_state(|| last_state.unwrap_or(QueryState::Idle))
    };

    let query_value = {
        let last_value = observer.last_value();
        use_state(move || last_value)
    };

    // We use an id to ensure only set the last value
    // https://docs.rs/yew/0.20.0/src/yew/suspense/hooks.rs.html#97
    let latest_id = use_state(|| std::cell::Cell::new(0_u32));
    let is_stale = observer.is_stale();

    let do_fetch = {
        let query_state = query_state.clone();
        let query_value = query_value.clone();
        let query_fetching = query_fetching.clone();
        let fetch = fetch.clone();
        let latest_id = latest_id.clone();
        let abort_controller = abort_controller.clone();

        use_callback(
            move |target, deps| {
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

                observer.observe(target, f, move |event| {
                    if !enabled {
                        return;
                    }

                    let QueryChangeEvent {
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
            (enabled, query_key.clone()),
        )
    };

    let remove = {
        let query_value = query_value.clone();
        let query_state = query_state.clone();
        let query_fetching = query_fetching.clone();
        let client = client.clone();
        let query_key = query_key.clone();

        use_callback(
            move |(), (key,)| {
                let mut client = client.clone();

                // Updates the id to prevent update the state
                let self_id = latest_id.get().wrapping_add(1);
                (*latest_id).set(self_id);

                client.remove_query_data(key);
                query_state.set(QueryState::Idle);
                query_value.set(None);
                query_fetching.set(false);
            },
            (query_key.clone(),),
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

    // On mount
    {
        let do_fetch = do_fetch.clone();

        use_effect_with_deps(
            move |_| {
                if first_render || refetch_on_mount {
                    do_fetch.emit(ObserveTarget::Fetch);
                }

                move || {
                    abort_controller.abort();
                }
            },
            (is_stale,),
        );
    }

    // On reconnect
    {
        let do_fetch = do_fetch.clone();
        use_on_online(move || {
            if refetch_on_reconnect {
                do_fetch.emit(ObserveTarget::Refetch);
            }
        });
    }

    // On window focus
    {
        let do_fetch = do_fetch.clone();
        use_on_window_focus(move || {
            if refetch_on_window_focus {
                do_fetch.emit(ObserveTarget::Refetch);
            }
        });
    }

    //

    UseQueryHandle {
        id,
        key: query_key,
        remove,
        fetch: do_fetch,
        state: query_state,
        value: query_value,
        is_fetching: query_fetching,
    }
}
