//! Multi-tenancy support for LLM CoPilot Agent
//!
//! This crate provides comprehensive multi-tenant functionality including:
//! - Tenant isolation with separate schemas/namespaces
//! - Per-tenant rate limiting and quotas
//! - Tenant onboarding and management
//! - Usage metering and billing hooks
//! - Resource quotas and limits

pub mod tenant;
pub mod isolation;
pub mod quota;
pub mod metering;
pub mod billing;
pub mod onboarding;

pub use tenant::*;
pub use isolation::*;
pub use quota::*;
pub use metering::*;
pub use billing::*;
pub use onboarding::*;

use thiserror::Error;

/// Multi-tenancy errors
#[derive(Error, Debug)]
pub enum TenantError {
    #[error("Tenant not found: {0}")]
    NotFound(String),

    #[error("Tenant already exists: {0}")]
    AlreadyExists(String),

    #[error("Tenant is disabled: {0}")]
    Disabled(String),

    #[error("Tenant is suspended: {0}")]
    Suspended(String),

    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),

    #[error("Rate limit exceeded for tenant: {0}")]
    RateLimitExceeded(String),

    #[error("Invalid tenant configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Isolation error: {0}")]
    Isolation(String),

    #[error("Billing error: {0}")]
    Billing(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, TenantError>;
