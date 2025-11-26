//! # Copilot SDK
//!
//! Official Rust SDK for the LLM CoPilot Agent platform.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use copilot_sdk::CopilotClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = CopilotClient::builder()
//!         .base_url("http://localhost:8080")
//!         .api_key(Some("your-api-key".to_string()))
//!         .build()?;
//!
//!     // Start a chat
//!     let response = client.chat("Hello, how can you help me?", None).await?;
//!     println!("{}", response.content);
//!
//!     Ok(())
//! }
//! ```

mod client;
mod error;
mod models;
mod streaming;

pub use client::{CopilotClient, CopilotClientBuilder};
pub use error::{CopilotError, Result};
pub use models::*;
pub use streaming::StreamEvent;

/// SDK version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
