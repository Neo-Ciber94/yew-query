#[cfg(target = "wasm32")]
mod rt_timeout_wasm;

#[cfg(not(target = "wasm32"))]
mod rt_timeout_tokio;


#[cfg(target = "wasm32")]
pub use rt_timeout_wasm::Timeout;

#[cfg(not(target = "wasm32"))]
pub use rt_timeout_tokio::Timeout;

pub mod interval;