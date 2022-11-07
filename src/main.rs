use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use log::Level;
use serde::{Deserialize, Serialize};
use yew::prelude::*;
use yew_query::context::QueryClientProvider;
use yew_query::core::client::QueryClient;
use yew_query::hooks::use_query_with_failure;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub user_id: i64,
    pub id: i64,
    pub title: String,
    pub body: String,
}

#[function_component(PostList)]
fn post_list() -> Html {
    let query = use_query_with_failure("posts", fetch_posts);

    if query.is_loading() {
        return html! {
            "Loading..."
        };
    }

    if query.is_error() {
        return html! {
            <p style="color: red;">{format!("Error: {}", query.error().unwrap())}</p>
        };
    }

    log::info!("Result: {query:#?}");

    let posts = query.data().cloned().unwrap_or_default();

    html! {
        <>
            <ul>
                { posts.iter().map(|post| {
                    html! {
                        <li>
                            <p>{format!("id: {}", post.id)}</p>
                            <p>{format!("title: {}", post.title)}</p>
                        </li>
                    }
                }).collect::<Html>()}
            </ul>
        </>
    }
}

#[function_component(Content)]
fn content() -> Html {
    let show_state = use_state(|| false);

    let toggle_show = {
        let show_state = show_state.clone();

        move |_| {
            let show = !*show_state;
            show_state.set(show);
        }
    };

    html! {
        <>
            <button onclick={toggle_show}>{"Show"}</button>
            if *show_state {
                <PostList/>
            }
        </>
    }
}

#[function_component(App)]
fn app() -> Html {
    let client = Rc::new(RefCell::new(
        QueryClient::builder()
            .stale_time(Duration::from_secs(10))
            .build(HashMap::new()),
    ));

    html! {
        <QueryClientProvider {client}>
            <Content />
        </QueryClientProvider>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(Level::Trace));

    yew::start_app::<App>();
}

async fn fetch_posts() -> reqwest::Result<Vec<Post>> {
    reqwest::get("https://jsonplaceholder.typicode.com/posts")
        .await?
        .json::<Vec<Post>>()
        .await
}
