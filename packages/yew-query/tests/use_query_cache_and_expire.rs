#![cfg(target_arch = "wasm32")]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

mod common;

use common::*;
use std::{
    convert::Infallible,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
use wasm_bindgen_test::wasm_bindgen_test;
use yew::{
    platform::{spawn_local, time::sleep},
    use_effect_with_deps, use_force_update,
};

use yew_query::{use_query, QueryClient, QueryClientProvider};

static FETCH_COUNT: AtomicU32 = AtomicU32::new(0);

async fn get_value() -> Result<i32, Infallible> {
    FETCH_COUNT.fetch_add(1, Ordering::Relaxed);
    Ok(54321)
}

#[yew::function_component]
fn AppTest() -> yew::Html {
    let client = QueryClient::builder()
        .cache_time(Duration::from_millis(20))
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

    use_effect_with_deps(
        move |_| {
            spawn_local(async move {
                sleep(Duration::from_millis(25)).await;
                update.force_update();
            });
        },
        (),
    );

    if query.is_loading() || query.data().is_none() {
        return yew::html! { <div id="result">{"Loading..."}</div> };
    }

    yew::html! {
       <div>
            {"The test result is: "}
            <div id="result">{ query.data().unwrap() }</div>
            {"\n"}
       </div>
    }
}

#[wasm_bindgen_test]
async fn use_query_cache_and_expire() {
    yew::Renderer::<AppTest>::with_root(
        gloo_utils::document().get_element_by_id("output").unwrap(),
    )
    .render();

    sleep(Duration::from_millis(200)).await;
    let result = get_inner_html("result");

    assert_eq!(2, FETCH_COUNT.load(Ordering::Relaxed));
    assert_eq!("54321", result);
}
