use crate::core::client::QueryClient;
use std::{cell::RefCell, rc::Rc};
use yew::{function_component, Children, ContextProvider, Properties};

pub struct QueryClientContext {
    pub(crate) client: Rc<RefCell<QueryClient>>,
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
        Rc::ptr_eq(&self.client, &other.client)
    }
}

#[derive(Properties)]
pub struct QueryClientContextProps {
    client: Rc<RefCell<QueryClient>>,

    #[prop_or_default]
    children: Children,
}

impl PartialEq for QueryClientContextProps {
    fn eq(&self, other: &Self) -> bool {
        let p1 = &self.client as *const _ as usize;
        let p2 = &other.client as *const _ as usize;
        p1 == p2 && self.children == other.children
    }
}

#[function_component(Get)]
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
