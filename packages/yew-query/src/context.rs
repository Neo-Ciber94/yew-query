use yew::{function_component, Children, ContextProvider, Properties};
use yew_query_core::client::QueryClient;

/// A context with the `QueryClient`.
#[derive(PartialEq)]
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

/// Properties for a `QueryClientContext`.
#[derive(Properties, PartialEq)]
pub struct QueryClientContextProps {
    pub client: QueryClient,

    #[prop_or_default]
    pub children: Children,
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
