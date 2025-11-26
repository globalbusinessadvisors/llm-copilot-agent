//! Command-line argument parsing

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "copilot-server",
    about = "LLM CoPilot Agent Server",
    version,
    long_about = "A high-performance AI agent server with LLM orchestration, \
                  RAG capabilities, and workflow automation."
)]
pub struct Args {
    /// Path to configuration file
    #[arg(
        short,
        long,
        env = "CONFIG_PATH",
        default_value = "config/default.toml"
    )]
    pub config: PathBuf,

    /// HTTP server port
    #[arg(short, long, env = "PORT", default_value = "8080")]
    pub port: u16,

    /// Log level (trace, debug, info, warn, error)
    #[arg(
        short,
        long,
        env = "LOG_LEVEL",
        default_value = "info",
        value_parser = ["trace", "debug", "info", "warn", "error"]
    )]
    pub log_level: String,

    /// Environment (dev, staging, prod)
    #[arg(
        short,
        long,
        env = "ENVIRONMENT",
        default_value = "dev",
        value_parser = ["dev", "staging", "prod"]
    )]
    pub env: String,

    /// Enable JSON log format (useful for production)
    #[arg(long, env = "JSON_LOGS")]
    pub json_logs: bool,
}

impl Args {
    /// Validate the arguments
    pub fn validate(&self) -> anyhow::Result<()> {
        // Config file is optional for MVP - we'll use defaults if not found
        Ok(())
    }
}
