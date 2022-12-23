use crate::context::QueryClientContext;
use std::{cell::RefCell, rc::Rc};
use yew::{use_context, hook};
use yew_query_core::client::QueryClient;

#[hook]
pub fn use_query_client() -> Option<QueryClient> {
    let ctx = use_context::<QueryClientContext>()?;
    Some(ctx.client)
}
