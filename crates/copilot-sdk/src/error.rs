//! Error types for the Copilot SDK

use thiserror::Error;

/// Result type alias for Copilot SDK operations
pub type Result<T> = std::result::Result<T, CopilotError>;

/// Errors that can occur when using the Copilot SDK
#[derive(Error, Debug)]
pub enum CopilotError {
    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// API returned an error response
    #[error("API error ({status}): {message}")]
    Api {
        status: u16,
        message: String,
        code: Option<String>,
    },

    /// JSON serialization/deserialization failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing failed
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Retry after {retry_after:?} seconds")]
    RateLimit { retry_after: Option<u64> },

    /// Request timeout
    #[error("Request timeout")]
    Timeout,

    /// Server error
    #[error("Server error: {0}")]
    Server(String),

    /// Invalid configuration
    #[error("Configuration error: {0}")]
    Config(String),

    /// Stream error
    #[error("Stream error: {0}")]
    Stream(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl CopilotError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            CopilotError::RateLimit { .. }
                | CopilotError::Timeout
                | CopilotError::Server(_)
                | CopilotError::Http(_)
        )
    }

    /// Get the HTTP status code if available
    pub fn status_code(&self) -> Option<u16> {
        match self {
            CopilotError::Api { status, .. } => Some(*status),
            CopilotError::RateLimit { .. } => Some(429),
            CopilotError::NotFound(_) => Some(404),
            CopilotError::Auth(_) => Some(401),
            _ => None,
        }
    }
}
