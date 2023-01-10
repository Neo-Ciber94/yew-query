#![cfg(target_arch = "wasm32")]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod common;

use common::*;
use std::{time::Duration, fmt::{Display, self}, sync::atomic::{Ordering, AtomicU32}};
use wasm_bindgen_test::wasm_bindgen_test;
use yew::{platform::time::sleep, prelude::*};
use yew_query::{use_query, QueryClient, QueryClientProvider};

#[derive(Debug)]
struct NoValueError;
impl std::error::Error for NoValueError {}
impl Display for NoValueError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "no value found")
    }
}

#[derive(Properties, PartialEq)]
struct UseQueryComponentProps {
    retry_count: UseStateHandle<u32>
}

#[yew::function_component]
fn AppTest() -> yew::Html {
    let retry_count = use_state(|| 0);
    let client = QueryClient::builder()
        .retry({
            let retry_count = retry_count.clone();
            move || {
                let retry_count = retry_count.clone();
                std::iter::repeat(Duration::from_millis(10)).inspect(move |_| {
                    retry_count.set(*retry_count + 1);
                })
            }
        })
        .build();

    yew::html! {
        <QueryClientProvider client={client}>
            <UseQueryComponent retry_count={retry_count}/>
        </QueryClientProvider>
    }
}

static COUNT: AtomicU32 = AtomicU32::new(0);

#[yew::function_component]
fn UseQueryComponent(props: &UseQueryComponentProps) -> yew::Html {


    let query =  use_query("number", || async move {
        let count = COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if count == 3 {
            Ok(69420)
        } else {
            Err(NoValueError)
        }
    });

    if query.is_error() {
        return yew::html! {
            <div id="error">{format!("{}", query.error().unwrap())}</div>
        };
    }

    if query.is_loading() || query.data().is_none() {
        return yew::html! { <div id="result">{"Loading..."}</div> };
    }

    let retry_count = *props.retry_count;
    yew::html! {
        <>
            <div id="result">{ query.data().unwrap()  }</div>
            <div id="retry_count">{ retry_count }</div>
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
    let retry_count = get_inner_html("retry_count");

    assert_eq!("3", retry_count);
    assert_eq!("69420", result);
}
