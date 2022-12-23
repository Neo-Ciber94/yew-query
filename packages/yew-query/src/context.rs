use std::{cell::RefCell, rc::Rc};
use yew::{function_component, Children, ContextProvider, Properties};
use yew_query_core::client::QueryClient;

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

// impl PartialEq for QueryClientContext {
//     fn eq(&self, other: &Self) -> bool {
//         Rc::ptr_eq(&self.client, &other.client)
//     }
// }

#[derive(Properties, PartialEq)]
pub struct QueryClientContextProps {
    pub client: QueryClient,

    #[prop_or_default]
    pub children: Children,
}

// impl PartialEq for QueryClientContextProps {
//     fn eq(&self, other: &Self) -> bool {
//         Rc::ptr_eq(&self.client, &other.client) && self.children == other.children
//     }
// }

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
