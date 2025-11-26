//! E2B Integration for Autonomous Agent Execution
//!
//! This crate provides integration with E2B (e2b.dev) for running autonomous
//! AI agents in secure, isolated cloud sandboxes.
//!
//! # Features
//!
//! - Sandbox lifecycle management (create, run, destroy)
//! - Code execution in isolated environments
//! - File system operations within sandboxes
//! - Process management and output streaming
//! - Custom environment configuration
//!
//! # Example
//!
//! ```rust,no_run
//! use copilot_e2b::{E2BAgent, E2BConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = E2BConfig::from_env()?;
//!     let mut agent = E2BAgent::new(config).await?;
//!
//!     // Execute code in sandbox
//!     let result = agent.execute_code("print('Hello from E2B!')").await?;
//!     println!("Output: {}", result.stdout);
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod sandbox;
pub mod agent;
pub mod execution;

pub use config::{E2BConfig, SandboxTemplate};
pub use sandbox::{Sandbox, SandboxStatus};
pub use agent::{E2BAgent, AgentTask, AgentResult};
pub use execution::{ExecutionResult, ExecutionError};

use thiserror::Error;

/// Errors that can occur during E2B operations
#[derive(Error, Debug)]
pub enum E2BError {
    #[error("E2B API error: {0}")]
    ApiError(String),

    #[error("Sandbox error: {0}")]
    SandboxError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("File system error: {0}")]
    FileSystemError(String),

    #[error("Process error: {0}")]
    ProcessError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl E2BError {
    pub fn api(msg: impl Into<String>) -> Self {
        Self::ApiError(msg.into())
    }

    pub fn sandbox(msg: impl Into<String>) -> Self {
        Self::SandboxError(msg.into())
    }

    pub fn execution(msg: impl Into<String>) -> Self {
        Self::ExecutionError(msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    pub fn auth(msg: impl Into<String>) -> Self {
        Self::AuthError(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, E2BError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = E2BError::api("API failed");
        assert!(err.to_string().contains("API failed"));

        let err = E2BError::sandbox("Sandbox creation failed");
        assert!(err.to_string().contains("Sandbox creation failed"));
    }
}
