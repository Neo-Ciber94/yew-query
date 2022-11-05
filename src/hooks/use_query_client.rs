use crate::{context::QueryClientContext, core::client::QueryClient};
use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};
use yew::use_context;

pub struct QueryClientHandle {
    inner: Rc<RefCell<QueryClient>>,
}

impl QueryClientHandle {
    pub fn get(&self) -> Ref<'_, QueryClient> {
        self.inner.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, QueryClient> {
        self.inner.borrow_mut()
    }
}

impl PartialEq for QueryClientHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

pub fn use_query_client() -> Option<QueryClientHandle> {
    let inner = use_context::<QueryClientContext>()?.client;
    Some(QueryClientHandle { inner })
}
