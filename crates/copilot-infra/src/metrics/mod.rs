//! Prometheus metrics for observability
//!
//! Provides application metrics collection and exposition.

pub mod prometheus;
pub mod collector;

pub use prometheus::{
    PrometheusMetrics, MetricsConfig, MetricsHandle, HttpMetrics, DatabaseMetrics,
    CacheMetrics, CircuitBreakerMetrics,
};
pub use collector::{MetricsCollector, SystemMetrics};
