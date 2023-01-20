#![cfg(target_arch = "wasm32")]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod common;

use common::*;
use std::{
    convert::Infallible,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_test::wasm_bindgen_test;
use yew::{platform::time::sleep, use_effect_with_deps, use_force_update};
use yew_query::{use_query, QueryClient, QueryClientProvider};

static REFETCH_COUNT: AtomicUsize = AtomicUsize::new(0);

async fn get_value() -> Result<u32, Infallible> {
    REFETCH_COUNT.fetch_add(1, Ordering::Relaxed);
    Ok(12345)
}

#[yew::function_component]
fn AppTest() -> yew::Html {
    let client = QueryClient::builder()
        .cache_time(Duration::from_millis(500))
        .build();

    yew::html! {
        <QueryClientProvider client={client}>
            <UseQueryComponent/>
        </QueryClientProvider>
    }
}

#[yew::function_component]
fn UseQueryComponent() -> yew::Html {
    let query = use_query("number", get_value);
    let update = use_force_update();

    {
        let query = query.clone();
        use_effect_with_deps(
            move |_| {
                spawn_local(async move {
                    query.remove();
                    sleep(Duration::from_millis(10)).await;
                    update.force_update();
                });
            },
            (),
        );
    }

    if !query.is_completed() {
        return yew::html! { <div id="result">{"Loading..."}</div> };
    }

    yew::html! {
        <div id="result">{ query.data().unwrap() }</div>
    }
}

#[wasm_bindgen_test]
async fn use_query_remove() {
    yew::Renderer::<AppTest>::with_root(
        gloo_utils::document().get_element_by_id("output").unwrap(),
    )
    .render();

    sleep(Duration::from_millis(10)).await;
    let result = get_inner_html("result");

    assert_eq!(2, REFETCH_COUNT.load(Ordering::Relaxed));
    assert_eq!("12345", result);
}
