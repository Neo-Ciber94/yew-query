pub mod cache;
pub mod client;
pub mod error;
pub mod fetcher;
pub mod infinite;
pub mod query;
pub mod retry;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

//
pub(crate) mod timeout;
