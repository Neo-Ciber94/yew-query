#![cfg(target_arch = "wasm32")]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod common;

use common::*;
use instant::Duration;
use std::{convert::Infallible, sync::atomic::{AtomicU32, Ordering}};
use wasm_bindgen_test::wasm_bindgen_test;
use yew::platform::time::sleep;
use yew_query::{use_query, QueryClient, QueryClientProvider};

#[yew::function_component]
fn AppTest() -> yew::Html {
    let client = QueryClient::builder()
        .cache_time(Duration::from_millis(1000))
        .build();

    yew::html! {
        <QueryClientProvider client={client}>
            <CompA/>
            <CompB/>
            <CompC/>
        </QueryClientProvider>
    }
}

static FETCH_COUNT : AtomicU32 = AtomicU32::new(0);

async fn get_value() -> Result<i32, Infallible> {
    FETCH_COUNT.fetch_add(1, Ordering::Relaxed);
    sleep(Duration::from_millis(100)).await;
    Ok::<_, Infallible>(9000_i32)
}

#[yew::function_component]
fn CompA() -> yew::Html {
    let query = use_query("number", get_value);

    if !query.is_completed() {
        return yew::html! { <div id="result_1">{"Loading..."}</div> };
    }

    yew::html! {
        <div id="result_1">{ query.data().unwrap() }</div>
    }
}

#[yew::function_component]
fn CompB() -> yew::Html {
    let query = use_query("number", get_value);

    if !query.is_completed() {
        return yew::html! { <div id="result_2">{"Loading..."}</div> };
    }

    yew::html! {
        <div id="result_2">{ query.data().unwrap() }</div>
    }
}

#[yew::function_component]
fn CompC() -> yew::Html {
    let query = use_query("number", get_value);

    if !query.is_completed() {
        return yew::html! { <div id="result_3">{"Loading..."}</div> };
    }

    yew::html! {
        <div id="result_3">{ query.data().unwrap() }</div>
    }
}

#[wasm_bindgen_test]
async fn use_query_share_cache_value() {
    yew::Renderer::<AppTest>::with_root(
        gloo_utils::document().get_element_by_id("output").unwrap(),
    )
    .render();

    sleep(std::time::Duration::from_millis(300)).await;

    assert_eq!(1, FETCH_COUNT.load(Ordering::Relaxed));

    let result_1 = get_inner_html("result_1");
    let result_2 = get_inner_html("result_2");
    let result_3 = get_inner_html("result_3");

    assert_eq!("9000", result_1);
    assert_eq!("9000", result_2);
    assert_eq!("9000", result_3);
}
