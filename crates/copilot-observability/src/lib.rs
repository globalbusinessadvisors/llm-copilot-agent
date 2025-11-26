//! Observability and analytics for LLM CoPilot Agent
//!
//! This crate provides comprehensive observability features:
//! - Distributed tracing with OpenTelemetry
//! - Structured logging with correlation IDs
//! - Custom business metrics
//! - Analytics dashboards data
//! - SLA monitoring

pub mod tracing_setup;
pub mod correlation;
pub mod analytics;
pub mod sla;
pub mod dashboards;

pub use tracing_setup::*;
pub use correlation::*;
pub use analytics::*;
pub use sla::*;
pub use dashboards::*;

use thiserror::Error;

/// Observability errors
#[derive(Error, Debug)]
pub enum ObservabilityError {
    #[error("Tracing initialization failed: {0}")]
    TracingInit(String),

    #[error("Metrics error: {0}")]
    Metrics(String),

    #[error("Analytics error: {0}")]
    Analytics(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

pub type Result<T> = std::result::Result<T, ObservabilityError>;
