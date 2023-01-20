#![cfg(target_arch = "wasm32")]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod common;

use common::*;
use std::{
    convert::Infallible,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use wasm_bindgen_test::wasm_bindgen_test;
use yew::{platform::time::sleep, use_mut_ref};
use yew_query::{use_query, QueryClient, QueryClientProvider};

static FETCH_COUNT: AtomicU64 = AtomicU64::new(0);

async fn get_data() -> Result<u64, Infallible> {
    let val = FETCH_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    Ok(val)
}

#[yew::function_component]
fn AppTest() -> yew::Html {
    let client = QueryClient::builder()
        .refetch_time(Duration::from_millis(20))
        .build();

    yew::html! {
        <QueryClientProvider client={client}>
            <UseQueryComponent/>
        </QueryClientProvider>
    }
}

#[yew::function_component]
fn UseQueryComponent() -> yew::Html {
    let query = use_query("number", get_data);
    let items = use_mut_ref(|| Vec::new());

    {
        let items = items.clone();
        let query = query.clone();

        if let Some(value) = query.data() {
            items.borrow_mut().push(*value);
        }
    }

    if query.is_loading() || query.data().is_none() {
        return yew::html! { <div id="result">{"Loading..."}</div> };
    }

    let result = items
        .borrow()
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>();

    yew::html! {
        <div id="result">{ format!("{}", result.join(",")) }</div>
    }
}

#[wasm_bindgen_test]
async fn use_query_refetch() {
    yew::Renderer::<AppTest>::with_root(
        gloo_utils::document().get_element_by_id("output").unwrap(),
    )
    .render();

    sleep(Duration::from_millis(60)).await;
    let result = get_inner_html("result");

    assert_eq!(3, FETCH_COUNT.load(Ordering::Relaxed));
    assert_eq!("1,2,3", result);
}
