#![cfg(target_arch = "wasm32")]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod common;

use common::*;
use std::{convert::Infallible, time::Duration};
use wasm_bindgen_test::wasm_bindgen_test;
use yew::platform::time::sleep;
use yew_query::{use_query, QueryClient, QueryClientProvider};

#[yew::function_component]
fn AppTest() -> yew::Html {
    let client = QueryClient::builder().build();

    yew::html! {
        <QueryClientProvider client={client}>
            <UseQueryComponent/>
        </QueryClientProvider>
    }
}

#[yew::function_component]
fn UseQueryComponent() -> yew::Html {
    let query = use_query("number", || async { Ok::<_, Infallible>(23_i32) });

    if query.is_loading() || query.data().is_none() {
        return yew::html! { <div id="result">{"Loading..."}</div> };
    }

    yew::html! {
        <div id="result">{ query.data().unwrap() }</div>
    }
}

#[wasm_bindgen_test]
async fn use_query_expect_value() {
    yew::Renderer::<AppTest>::with_root(
        gloo_utils::document().get_element_by_id("output").unwrap(),
    )
    .render();

    sleep(Duration::ZERO).await;
    let result = get_inner_html("result");
    assert_eq!("23", result);
}
