pub mod cache;
pub mod client;
pub mod fetcher;
pub mod query;
pub mod retry;
pub mod error;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;