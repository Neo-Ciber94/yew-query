use crate::context::QueryClientContext;
use std::{cell::RefCell, rc::Rc};
use yew::use_context;
use yew_query_core::client::QueryClient;

pub fn use_query_client() -> Option<Rc<RefCell<QueryClient>>> {
    let ctx = use_context::<QueryClientContext>()?;
    Some(ctx.client)
}
