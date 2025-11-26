//! E2B configuration management

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for E2B integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2BConfig {
    /// E2B API key
    pub api_key: String,

    /// Default sandbox template to use
    pub default_template: SandboxTemplate,

    /// Default timeout for sandbox operations
    pub timeout: Duration,

    /// Maximum number of concurrent sandboxes
    pub max_sandboxes: usize,

    /// Enable sandbox keep-alive
    pub keep_alive: bool,

    /// Keep-alive interval
    pub keep_alive_interval: Duration,

    /// Custom environment variables for sandboxes
    pub env_vars: std::collections::HashMap<String, String>,
}

impl E2BConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> crate::Result<Self> {
        let api_key = std::env::var("E2B_API_KEY")
            .map_err(|_| crate::E2BError::config("E2B_API_KEY environment variable not set"))?;

        Ok(Self {
            api_key,
            ..Default::default()
        })
    }

    /// Create configuration with the given API key
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            ..Default::default()
        }
    }

    /// Set the default sandbox template
    pub fn template(mut self, template: SandboxTemplate) -> Self {
        self.default_template = template;
        self
    }

    /// Set the operation timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set maximum concurrent sandboxes
    pub fn max_sandboxes(mut self, max: usize) -> Self {
        self.max_sandboxes = max;
        self
    }

    /// Add an environment variable
    pub fn env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

impl Default for E2BConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            default_template: SandboxTemplate::Python,
            timeout: Duration::from_secs(300),
            max_sandboxes: 5,
            keep_alive: true,
            keep_alive_interval: Duration::from_secs(30),
            env_vars: std::collections::HashMap::new(),
        }
    }
}

/// Available sandbox templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxTemplate {
    /// Python environment with common data science packages
    Python,
    /// Node.js environment
    NodeJs,
    /// Go environment
    Go,
    /// Rust environment
    Rust,
    /// Shell/Bash environment
    Bash,
    /// Custom template (requires template ID)
    Custom,
}

impl SandboxTemplate {
    /// Get the E2B template ID for this template
    pub fn template_id(&self) -> &'static str {
        match self {
            SandboxTemplate::Python => "python",
            SandboxTemplate::NodeJs => "nodejs",
            SandboxTemplate::Go => "go",
            SandboxTemplate::Rust => "rust",
            SandboxTemplate::Bash => "bash",
            SandboxTemplate::Custom => "custom",
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            SandboxTemplate::Python => "Python 3.x with common data science packages",
            SandboxTemplate::NodeJs => "Node.js with npm",
            SandboxTemplate::Go => "Go 1.x environment",
            SandboxTemplate::Rust => "Rust with Cargo",
            SandboxTemplate::Bash => "Shell/Bash environment",
            SandboxTemplate::Custom => "Custom template",
        }
    }
}

impl std::fmt::Display for SandboxTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.template_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = E2BConfig::with_api_key("test-key")
            .template(SandboxTemplate::Python)
            .timeout(Duration::from_secs(60))
            .max_sandboxes(3)
            .env_var("MY_VAR", "my_value");

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.default_template, SandboxTemplate::Python);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_sandboxes, 3);
        assert_eq!(config.env_vars.get("MY_VAR"), Some(&"my_value".to_string()));
    }

    #[test]
    fn test_template_ids() {
        assert_eq!(SandboxTemplate::Python.template_id(), "python");
        assert_eq!(SandboxTemplate::NodeJs.template_id(), "nodejs");
        assert_eq!(SandboxTemplate::Rust.template_id(), "rust");
    }
}
