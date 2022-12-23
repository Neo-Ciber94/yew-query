pub mod cache;
pub mod client;
pub mod error;
pub mod fetcher;
pub mod query;
pub mod retry;
pub mod observer;
pub mod key;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[doc(hidden)]
pub mod timeout;
