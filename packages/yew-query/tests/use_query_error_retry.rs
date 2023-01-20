#![cfg(target_arch = "wasm32")]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod common;

use common::*;
use std::{
    fmt::{self, Display},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
use wasm_bindgen_test::wasm_bindgen_test;
use yew::platform::time::sleep;
use yew_query::{use_query, QueryClient, QueryClientProvider};

#[derive(Debug)]
struct NoValueError;
impl std::error::Error for NoValueError {}
impl Display for NoValueError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "no value found")
    }
}

static RETRY_COUNT: AtomicU32 = AtomicU32::new(0);

#[yew::function_component]
fn AppTest() -> yew::Html {
    let client = QueryClient::builder()
        .retry({
            move || {
                std::iter::repeat(Duration::from_millis(10)).inspect(move |_| {
                    RETRY_COUNT.fetch_add(1, Ordering::Relaxed);
                })
            }
        })
        .build();

    yew::html! {
        <QueryClientProvider client={client}>
            <UseQueryComponent />
        </QueryClientProvider>
    }
}

#[yew::function_component]
fn UseQueryComponent() -> yew::Html {
    static FETCH_COUNT: AtomicU32 = AtomicU32::new(0);

    let query = use_query("number", || async move {
        let val = FETCH_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if val == 4 {
            Ok(69420)
        } else {
            Err(NoValueError)
        }
    });

    if query.is_error() {
        return yew::html! {
            <div id="result">{format!("{}", query.error().unwrap())}</div>
        };
    }

    if !query.is_completed() {
        return yew::html! { <div id="result">{"Loading..."}</div> };
    }

    yew::html! {
        <>
            <div id="result">{ query.data().unwrap() }</div>
        </>
    }
}

#[wasm_bindgen_test]
async fn use_query_error_retry() {
    yew::Renderer::<AppTest>::with_root(
        gloo_utils::document().get_element_by_id("output").unwrap(),
    )
    .render();

    sleep(Duration::from_millis(100)).await;

    let result = get_inner_html("result");

    assert_eq!("69420", result);
    assert_eq!(3, RETRY_COUNT.load(Ordering::Relaxed));
}
