//! Telemetry initialization (logging and tracing)

use anyhow::{Context, Result};
use tracing::Level;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    fmt,
};

use crate::cli::Args;

/// Guards that must be kept alive for the duration of the program
pub struct TelemetryGuards;

/// Initialize telemetry stack (logging)
pub fn init_telemetry(args: &Args) -> Result<TelemetryGuards> {
    // Build environment filter
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&args.log_level))
        .context("Failed to create environment filter")?;

    // Create subscriber with formatting layer
    if args.json_logs {
        // JSON formatting for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .json()
                    .with_target(true)
            )
            .init();
    } else {
        // Pretty formatting for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .pretty()
                    .with_target(true)
                    .with_line_number(true)
                    .with_file(true)
            )
            .init();
    };

    Ok(TelemetryGuards)
}

/// Helper to get the current log level
pub fn get_log_level(level_str: &str) -> Level {
    match level_str.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_log_level() {
        assert_eq!(get_log_level("trace"), Level::TRACE);
        assert_eq!(get_log_level("debug"), Level::DEBUG);
        assert_eq!(get_log_level("info"), Level::INFO);
        assert_eq!(get_log_level("warn"), Level::WARN);
        assert_eq!(get_log_level("error"), Level::ERROR);
        assert_eq!(get_log_level("invalid"), Level::INFO);
    }
}
