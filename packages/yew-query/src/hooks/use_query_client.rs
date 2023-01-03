use crate::context::QueryClientContext;
use yew::{hook, use_context};
use yew_query_core::QueryClient;

/// Returns the current `QueryClient`.
#[hook]
pub fn use_query_client() -> Option<QueryClient> {
    let ctx = use_context::<QueryClientContext>()?;
    Some(ctx.client)
}
