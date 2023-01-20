#![cfg(target_arch = "wasm32")]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod common;

use common::*;
use gloo_utils::window;
use std::{
    convert::Infallible,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::CustomEvent;
use yew::platform::time::sleep;
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

    if !query.is_completed() {
        return yew::html! { <div id="result">{"Loading..."}</div> };
    }

    yew::html! {
        <div id="result">{ query.data().unwrap() }</div>
    }
}

#[wasm_bindgen_test]
async fn use_query_refetch_on_focus() {
    yew::Renderer::<AppTest>::with_root(
        gloo_utils::document().get_element_by_id("output").unwrap(),
    )
    .render();

    sleep(Duration::from_millis(10)).await;

    let window = window();
    let event = CustomEvent::new("focus").unwrap();
    window.dispatch_event(&event).unwrap();

    sleep(Duration::from_millis(10)).await;
    let result = get_inner_html("result");

    assert_eq!(2, REFETCH_COUNT.load(Ordering::Relaxed));
    assert_eq!("12345", result);
}
