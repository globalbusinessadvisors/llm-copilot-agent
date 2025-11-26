//! Security error types

use thiserror::Error;

/// Result type alias for security operations
pub type Result<T> = std::result::Result<T, SecurityError>;

/// Security-related errors
#[derive(Error, Debug)]
pub enum SecurityError {
    /// Authentication failed (invalid credentials)
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Authorization failed (insufficient permissions)
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    /// Token is invalid or malformed
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// Token has expired
    #[error("Token expired")]
    TokenExpired,

    /// Token has been revoked
    #[error("Token revoked")]
    TokenRevoked,

    /// Refresh token is invalid
    #[error("Invalid refresh token")]
    InvalidRefreshToken,

    /// Invalid API key
    #[error("Invalid API key")]
    InvalidApiKey,

    /// API key expired
    #[error("API key expired")]
    ApiKeyExpired,

    /// API key lacks required scope
    #[error("API key lacks scope: {0}")]
    InsufficientScope(String),

    /// Password does not meet requirements
    #[error("Password validation failed: {0}")]
    PasswordValidation(String),

    /// Password hashing failed
    #[error("Password hashing failed: {0}")]
    PasswordHashingFailed(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Retry after {retry_after_secs} seconds")]
    RateLimitExceeded { retry_after_secs: u64 },

    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// User already exists
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),

    /// Invalid role
    #[error("Invalid role: {0}")]
    InvalidRole(String),

    /// Configuration error
    #[error("Security configuration error: {0}")]
    Configuration(String),

    /// Internal error
    #[error("Internal security error: {0}")]
    Internal(String),

    /// JWT library error
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl SecurityError {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> u16 {
        match self {
            SecurityError::AuthenticationFailed(_) => 401,
            SecurityError::AuthorizationFailed(_) => 403,
            SecurityError::InvalidToken(_) => 401,
            SecurityError::TokenExpired => 401,
            SecurityError::TokenRevoked => 401,
            SecurityError::InvalidRefreshToken => 401,
            SecurityError::InvalidApiKey => 401,
            SecurityError::ApiKeyExpired => 401,
            SecurityError::InsufficientScope(_) => 403,
            SecurityError::PasswordValidation(_) => 400,
            SecurityError::PasswordHashingFailed(_) => 500,
            SecurityError::RateLimitExceeded { .. } => 429,
            SecurityError::UserNotFound => 404,
            SecurityError::UserAlreadyExists(_) => 409,
            SecurityError::InvalidRole(_) => 400,
            SecurityError::Configuration(_) => 500,
            SecurityError::Internal(_) => 500,
            SecurityError::Jwt(_) => 401,
        }
    }

    /// Get error code for this error
    pub fn error_code(&self) -> &'static str {
        match self {
            SecurityError::AuthenticationFailed(_) => "AUTHENTICATION_FAILED",
            SecurityError::AuthorizationFailed(_) => "AUTHORIZATION_FAILED",
            SecurityError::InvalidToken(_) => "INVALID_TOKEN",
            SecurityError::TokenExpired => "TOKEN_EXPIRED",
            SecurityError::TokenRevoked => "TOKEN_REVOKED",
            SecurityError::InvalidRefreshToken => "INVALID_REFRESH_TOKEN",
            SecurityError::InvalidApiKey => "INVALID_API_KEY",
            SecurityError::ApiKeyExpired => "API_KEY_EXPIRED",
            SecurityError::InsufficientScope(_) => "INSUFFICIENT_SCOPE",
            SecurityError::PasswordValidation(_) => "PASSWORD_VALIDATION_FAILED",
            SecurityError::PasswordHashingFailed(_) => "PASSWORD_HASHING_FAILED",
            SecurityError::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            SecurityError::UserNotFound => "USER_NOT_FOUND",
            SecurityError::UserAlreadyExists(_) => "USER_ALREADY_EXISTS",
            SecurityError::InvalidRole(_) => "INVALID_ROLE",
            SecurityError::Configuration(_) => "CONFIGURATION_ERROR",
            SecurityError::Internal(_) => "INTERNAL_ERROR",
            SecurityError::Jwt(_) => "JWT_ERROR",
        }
    }
}
