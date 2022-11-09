pub mod cache;
pub mod client;
pub mod error;
pub mod fetcher;
pub mod query;
pub mod retry;
pub mod infinite_data;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
