use std::collections::HashMap;
use std::time::Duration;

use log::Level;
use serde::{Deserialize, Serialize};
use yew::platform::time::sleep;
use yew::prelude::*;
use yew_query::use_query;
use yew_query::QueryClient;
use yew_query::QueryClientProvider;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub user_id: i64,
    pub id: i64,
    pub title: String,
    pub body: String,
}

#[function_component]
fn PostList() -> Html {
    let query = use_query("posts", fetch_posts);

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

    let posts = query.data().cloned().unwrap_or_default();

    html! {
        <ul style="list-style-type: none;">
            { posts.iter().map(|post| {
                html! {
                    <li style="padding-bottom: 10px;">
                        <fieldset>
                            <legend>{format!("id: {}", post.id)}</legend>
                            <p>{format!("title: {}", post.title)}</p>
                        </fieldset>
                    </li>
                }
            }).collect::<Html>()}
        </ul>
    }
}

#[function_component]
fn Content() -> Html {
    let show = use_state(|| false);

    let toggle_show = {
        let show = show.clone();

        move |_| {
            let visible = !*show;
            show.set(visible);
        }
    };

    html! {
        <>
            <button onclick={toggle_show}>{if *show { "Hide" } else { "Show" } }</button>
            if *show {
                <PostList/>
            }
        </>
    }
}

#[function_component]
fn App() -> Html {
    let client = QueryClient::builder()
        .cache_time(Duration::from_secs(3))
        .refetch_time(Duration::from_secs(4))
        .cache(HashMap::new())
        .build();

    html! {
        <QueryClientProvider {client}>
            <Content />
        </QueryClientProvider>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(Level::Trace));
    yew::Renderer::<App>::new().render();
}

async fn fetch_posts() -> reqwest::Result<Vec<Post>> {
    sleep(Duration::from_secs(5)).await;
    reqwest::get("https://jsonplaceholder.typicode.com/posts")
        .await?
        .json::<Vec<Post>>()
        .await
}
