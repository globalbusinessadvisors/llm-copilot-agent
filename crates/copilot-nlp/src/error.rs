//! NLP-specific error types

use thiserror::Error;

/// NLP-specific error types
#[derive(Error, Debug)]
pub enum NlpError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Classification error: {0}")]
    Classification(String),

    #[error("Entity extraction error: {0}")]
    EntityExtraction(String),

    #[error("Query translation error: {0}")]
    QueryTranslation(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl NlpError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn classification(msg: impl Into<String>) -> Self {
        Self::Classification(msg.into())
    }

    pub fn entity_extraction(msg: impl Into<String>) -> Self {
        Self::EntityExtraction(msg.into())
    }

    pub fn query_translation(msg: impl Into<String>) -> Self {
        Self::QueryTranslation(msg.into())
    }

    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::Unsupported(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

/// Result type for NLP operations
pub type Result<T> = std::result::Result<T, NlpError>;

// Convert to copilot_core AppError
impl From<NlpError> for copilot_core::AppError {
    fn from(err: NlpError) -> Self {
        match err {
            NlpError::Validation(msg) => copilot_core::AppError::validation(msg),
            NlpError::Classification(msg) => copilot_core::AppError::internal(msg),
            NlpError::EntityExtraction(msg) => copilot_core::AppError::internal(msg),
            NlpError::QueryTranslation(msg) => copilot_core::AppError::internal(msg),
            NlpError::Unsupported(msg) => copilot_core::AppError::validation(msg),
            NlpError::Internal(msg) => copilot_core::AppError::internal(msg),
        }
    }
}
