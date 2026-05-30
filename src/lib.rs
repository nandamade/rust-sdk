//! # Rust SDK - Production-Grade SDK Library
//!
//! A comprehensive, feature-rich Rust SDK with:
//! - Async HTTP client with retry logic
//! - Advanced error handling
//! - Middleware system
//! - Configuration management
//! - Caching layer
//! - Request validation
//! - Rate limiting
//! - Database integration
//!
//! ## Quick Start
//!
//! ```rust
//! use rust_sdk::client::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::new("https://api.example.com")?;
//!     Ok(())
//! }
//! ```

pub mod cache;
pub mod client;
pub mod config;
pub mod database;
pub mod error;
pub mod middleware;
pub mod models;
pub mod rate_limit;
pub mod utils;
pub mod validation;

pub use client::Client;
pub use config::Config;
pub use error::{Result, SdkError};

#[doc(hidden)]
pub mod prelude {
    pub use crate::client::Client;
    pub use crate::config::Config;
    pub use crate::error::{Result, SdkError};
    pub use crate::middleware::{Middleware, MiddlewareChain};
    pub use crate::models::*;
}
