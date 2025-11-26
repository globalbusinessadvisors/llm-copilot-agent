//! Output formatting utilities

use colored::Colorize;
use serde::Serialize;

/// Output format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text
    Text,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Markdown format
    Markdown,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" | "plain" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "yaml" | "yml" => Ok(OutputFormat::Yaml),
            "markdown" | "md" => Ok(OutputFormat::Markdown),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Yaml => write!(f, "yaml"),
            OutputFormat::Markdown => write!(f, "markdown"),
        }
    }
}

/// Format output based on specified format
pub fn format_output<T: Serialize>(value: &T, format: OutputFormat) -> anyhow::Result<String> {
    match format {
        OutputFormat::Text => {
            // For text, just use debug format as fallback
            Ok(format!("{:?}", serde_json::to_value(value)?))
        }
        OutputFormat::Json => Ok(serde_json::to_string_pretty(value)?),
        OutputFormat::Yaml => Ok(serde_yaml::to_string(value)?),
        OutputFormat::Markdown => Ok(serde_json::to_string_pretty(value)?), // Fallback to JSON
    }
}

/// Print a success message
pub fn success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

/// Print an error message
pub fn error(message: &str) {
    eprintln!("{} {}", "✗".red(), message);
}

/// Print a warning message
pub fn warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}

/// Print an info message
pub fn info(message: &str) {
    println!("{} {}", "ℹ".blue(), message);
}

/// Print a key-value pair
pub fn key_value(key: &str, value: &str) {
    println!("{}: {}", key.bold(), value);
}

/// Print a dimmed message
pub fn dimmed(message: &str) {
    println!("{}", message.dimmed());
}

/// Print a list item
pub fn list_item(index: usize, message: &str) {
    println!("  {}. {}", index, message);
}

/// Print a section header
pub fn section(title: &str) {
    println!();
    println!("{}", title.bold().underline());
}

/// Format bytes as human-readable size
pub fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    }
}

/// Format duration in human-readable format
pub fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let minutes = ms / 60_000;
        let seconds = (ms % 60_000) / 1000;
        format!("{}m {}s", minutes, seconds)
    }
}

/// Truncate a string to a maximum length
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
