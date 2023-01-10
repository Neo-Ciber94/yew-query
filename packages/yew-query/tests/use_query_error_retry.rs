#![cfg(target_arch = "wasm32")]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod common;

use common::*;
use std::{
    fmt::{self, Display},
    rc::Rc,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration, cell::Cell,
};
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

#[derive(Default, Clone)]
struct Counter(Rc<Cell<u32>>);
impl Counter {
    fn increment(&self) {
        self.0.set(self.0.get() + 1);
    }

    fn get(&self) -> u32 {
        self.0.get()
    }
}

impl PartialEq for Counter {
    fn eq(&self, other: &Self) -> bool {
        self.0.get() == other.0.get()
    }
}

#[derive(Properties, PartialEq)]
struct AppTestProps {
    retry_count: Counter,
}

#[derive(Properties, PartialEq)]
struct UseQueryComponentProps {
    retry_count: UseStateHandle<Counter>,
}

#[yew::function_component]
fn AppTest(props: &AppTestProps) -> yew::Html {
    let retry_count = {
        let count = props.retry_count.clone();
        use_state(|| count)
    };

    let client = QueryClient::builder()
        .retry({
            let retry_count = retry_count.clone();
            move || {
                let retry_count = retry_count.clone();
                std::iter::repeat(Duration::from_millis(10)).inspect(move |_| {
                    retry_count.increment();
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

#[yew::function_component]
fn UseQueryComponent(props: &UseQueryComponentProps) -> yew::Html {
    static COUNT: AtomicU32 = AtomicU32::new(0);

    let query = use_query("number", || async move {
        let val = COUNT.fetch_add(1, Ordering::Relaxed) + 1;
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

    if query.is_loading() || query.data().is_none() {
        return yew::html! { <div id="result">{"Loading..."}</div> };
    }

    let retry_count = props.retry_count.get();
    yew::html! {
        <>
            <div id="result">{ query.data().unwrap()  }</div>
            <div id="retry_count">{ retry_count }</div>
        </>
    }
}

#[wasm_bindgen_test]
async fn use_query_error_retry() {
    let props = AppTestProps {
        retry_count: Default::default(),
    };

    yew::Renderer::<AppTest>::with_root_and_props(
        gloo_utils::document().get_element_by_id("output").unwrap(),
        props,
    )
    .render();

    sleep(Duration::from_millis(100)).await;

    let result = get_inner_html("result");
    let retry_count = get_inner_html("retry_count");

    assert_eq!("69420", result);
    assert_eq!("3", retry_count);
}
