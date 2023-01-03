use yew::{function_component, Children, ContextProvider, Properties};
use yew_query_core::QueryClient;

/// A context with the `QueryClient`.
pub struct QueryClientContext {
    pub(crate) client: QueryClient,
}

impl Clone for QueryClientContext {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl PartialEq for QueryClientContext {
    fn eq(&self, other: &Self) -> bool {
        eq_query_client(&self.client, &other.client)
    }
}

/// Properties for a `QueryClientContext`.
#[derive(Properties)]
pub struct QueryClientContextProps {
    pub client: QueryClient,

    #[prop_or_default]
    pub children: Children,
}

impl PartialEq for QueryClientContextProps {
    fn eq(&self, other: &Self) -> bool {
        eq_query_client(&self.client, &other.client) && self.children == other.children
    }
}

/// Declares a `QueryClient` for the app.
#[function_component]
pub fn QueryClientProvider(props: &QueryClientContextProps) -> yew::Html {
    let context = QueryClientContext {
        client: props.client.clone(),
    };

    yew::html! {
        <ContextProvider<QueryClientContext> context={context}>
            { for props.children.iter() }
        </ContextProvider<QueryClientContext>>
    }
}

fn eq_query_client(a: &QueryClient, b: &QueryClient) -> bool {
    std::ptr::eq(a, b)
}
