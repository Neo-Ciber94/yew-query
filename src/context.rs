use crate::core::client::QueryClient;
use std::{cell::RefCell, rc::Rc};
use yew::{function_component, Children, ContextProvider, Properties};

pub struct QueryClientContext<C> {
    client: Rc<RefCell<QueryClient<C>>>,
}

impl<C> Clone for QueryClientContext<C> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl<C> PartialEq for QueryClientContext<C> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.client, &other.client)
    }
}

#[derive(Properties)]
pub struct QueryClientContextProps<C> {
    client: Rc<RefCell<QueryClient<C>>>,

    #[prop_or_default]
    children: Children,
}

impl<C> PartialEq for QueryClientContextProps<C> {
    fn eq(&self, other: &Self) -> bool {
        let p1 = &self.client as *const _ as usize;
        let p2 = &other.client as *const _ as usize;
        p1 == p2 && self.children == other.children
    }
}

#[function_component(Get)]
pub fn QueryClientProvider<C: 'static>(props: &QueryClientContextProps<C>) -> yew::Html {
    let context = QueryClientContext {
        client: props.client.clone(),
    };

    yew::html! {
        <ContextProvider<QueryClientContext<C>> context={context}>
            { for props.children.iter() }
        </ContextProvider<QueryClientContext<C>>>
    }
}
