use std::{cell::Cell, convert::Infallible, rc::Rc};

use futures::Future;
use wasm_bindgen_futures::spawn_local;
use yew::{use_effect_with_deps, use_mut_ref, use_state, virtual_dom::Key};

use crate::core::Error;

use super::use_query_client::use_query_client;

pub enum QueryState<T> {
    Idle,
    Loading,
    Ready(Rc<T>),
    Failed(Error),
}

pub struct UseQueryOptions<F, Fut>
where
    F: Fn() -> Fut,
    Fut: Future,
{
    fetch: F,
    disabled: bool,
}

pub struct UseQueryHandle<T> {
    state: QueryState<T>,
}

pub fn use_query_with_options<F, Fut>(
    key: Key,
    options: UseQueryOptions<F, Fut>,
) -> UseQueryHandle<Fut::Output>
where
    F: Fn() -> Fut + 'static,
    Fut: Future + 'static,
{
    let UseQueryOptions { disabled, fetch } = options;
    let client = use_query_client().expect("expected `QueryClient`");
    let state = use_state(|| QueryState::<Fut::Output>::Idle);
    let last_id = use_state(|| Cell::new(0_usize));

    use_effect_with_deps(
        move |(k, disabled)| {
            let cleanup = || {};

            if *disabled {
                return cleanup;
            }

            state.set(QueryState::Loading);

            let id = last_id.get().wrapping_add(1);
            (*last_id).set(id);

            // spawn_local(async move {
            //     let fut = client.borrow_mut().fetch_query(k.clone(), || async move {
            //         let ret = fetch().await;
            //         Ok::<_, Infallible>(ret)
            //     });

            //     let result = fut.await;

            //     if id == last_id.get() {}
            // });

            cleanup
        },
        (key.clone(), disabled),
    );

    todo!()
}
