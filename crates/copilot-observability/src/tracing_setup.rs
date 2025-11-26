//! Tracing and OpenTelemetry setup
//!
//! Provides distributed tracing configuration and integration.

use crate::{ObservabilityError, Result};
use opentelemetry_otlp::WithExportConfig;
use serde::{Deserialize, Serialize};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Service name for tracing
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Environment (production, staging, development)
    pub environment: String,
    /// Enable OpenTelemetry exporter
    pub enable_otlp: bool,
    /// OTLP endpoint URL
    pub otlp_endpoint: String,
    /// Log level
    pub log_level: String,
    /// Enable JSON logging
    pub json_logs: bool,
    /// Enable span events (enter, exit)
    pub span_events: bool,
    /// Enable colored output (for console)
    pub colored_output: bool,
    /// Sample rate (0.0 to 1.0)
    pub sample_rate: f64,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "copilot-agent".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            environment: "development".to_string(),
            enable_otlp: false,
            otlp_endpoint: "http://localhost:4317".to_string(),
            log_level: "info".to_string(),
            json_logs: false,
            span_events: false,
            colored_output: true,
            sample_rate: 1.0,
        }
    }
}

impl TracingConfig {
    pub fn production() -> Self {
        Self {
            environment: "production".to_string(),
            enable_otlp: true,
            json_logs: true,
            span_events: false,
            colored_output: false,
            sample_rate: 0.1, // Sample 10% in production
            ..Default::default()
        }
    }

    pub fn with_service_name(mut self, name: &str) -> Self {
        self.service_name = name.to_string();
        self
    }

    pub fn with_otlp(mut self, endpoint: &str) -> Self {
        self.enable_otlp = true;
        self.otlp_endpoint = endpoint.to_string();
        self
    }

    pub fn with_log_level(mut self, level: &str) -> Self {
        self.log_level = level.to_string();
        self
    }
}

/// Initialize tracing with basic console output
pub fn init_tracing_simple(config: &TracingConfig) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let subscriber = tracing_subscriber::registry().with(filter);

    if config.json_logs {
        let fmt_layer = fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true);
        subscriber
            .with(fmt_layer)
            .try_init()
            .map_err(|e| ObservabilityError::TracingInit(e.to_string()))?;
    } else {
        let mut layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false);

        if config.span_events {
            layer = layer.with_span_events(FmtSpan::ENTER | FmtSpan::EXIT);
        }

        if !config.colored_output {
            layer = layer.with_ansi(false);
        }

        subscriber
            .with(layer)
            .try_init()
            .map_err(|e| ObservabilityError::TracingInit(e.to_string()))?;
    }

    tracing::info!(
        service = %config.service_name,
        version = %config.service_version,
        environment = %config.environment,
        "Tracing initialized"
    );

    Ok(())
}

/// Initialize tracing with OpenTelemetry
pub fn init_tracing_with_otlp(config: &TracingConfig) -> Result<TracingGuard> {
    use opentelemetry::KeyValue;
    use opentelemetry_sdk::{trace as sdktrace, Resource};

    // Create OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&config.otlp_endpoint);

    // Create tracer from OTLP pipeline
    // Note: install_batch returns a Tracer directly in this version
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            sdktrace::config().with_resource(Resource::new(vec![
                KeyValue::new("service.name", config.service_name.clone()),
                KeyValue::new("service.version", config.service_version.clone()),
                KeyValue::new("deployment.environment", config.environment.clone()),
            ])),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .map_err(|e| ObservabilityError::TracingInit(e.to_string()))?;

    // Create OpenTelemetry layer with the tracer
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Create filter
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    // Create fmt layer based on config
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(otel_layer);

    if config.json_logs {
        let fmt_layer = fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(true);
        subscriber
            .with(fmt_layer)
            .try_init()
            .map_err(|e| ObservabilityError::TracingInit(e.to_string()))?;
    } else {
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_ansi(config.colored_output);
        subscriber
            .with(fmt_layer)
            .try_init()
            .map_err(|e| ObservabilityError::TracingInit(e.to_string()))?;
    }

    tracing::info!(
        service = %config.service_name,
        otlp_endpoint = %config.otlp_endpoint,
        "Tracing initialized with OpenTelemetry"
    );

    Ok(TracingGuard::new())
}

/// Guard that shuts down tracing on drop
///
/// When dropped, this guard will flush any pending spans and shut down
/// the OpenTelemetry tracer provider gracefully.
pub struct TracingGuard {
    _private: (),
}

impl TracingGuard {
    /// Create a new tracing guard
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for TracingGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        // Shutdown the global tracer provider
        opentelemetry::global::shutdown_tracer_provider();
    }
}

/// Create a new span with common attributes
#[macro_export]
macro_rules! span_with_context {
    ($level:expr, $name:expr, $($field:tt)*) => {
        tracing::span!(
            $level,
            $name,
            otel.kind = "internal",
            $($field)*
        )
    };
}

/// Create a server span for incoming requests
#[macro_export]
macro_rules! server_span {
    ($name:expr, $method:expr, $path:expr) => {
        tracing::info_span!(
            $name,
            otel.kind = "server",
            http.method = %$method,
            http.route = %$path,
            http.status_code = tracing::field::Empty,
        )
    };
}

/// Create a client span for outgoing requests
#[macro_export]
macro_rules! client_span {
    ($name:expr, $method:expr, $url:expr) => {
        tracing::info_span!(
            $name,
            otel.kind = "client",
            http.method = %$method,
            http.url = %$url,
            http.status_code = tracing::field::Empty,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TracingConfig::default();

        assert_eq!(config.service_name, "copilot-agent");
        assert!(!config.enable_otlp);
        assert!(!config.json_logs);
    }

    #[test]
    fn test_production_config() {
        let config = TracingConfig::production();

        assert_eq!(config.environment, "production");
        assert!(config.enable_otlp);
        assert!(config.json_logs);
        assert_eq!(config.sample_rate, 0.1);
    }

    #[test]
    fn test_config_builder() {
        let config = TracingConfig::default()
            .with_service_name("my-service")
            .with_otlp("http://jaeger:4317")
            .with_log_level("debug");

        assert_eq!(config.service_name, "my-service");
        assert!(config.enable_otlp);
        assert_eq!(config.otlp_endpoint, "http://jaeger:4317");
        assert_eq!(config.log_level, "debug");
    }
}
