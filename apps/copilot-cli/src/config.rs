//! CLI configuration management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliConfig {
    /// API server URL
    pub api_url: Option<String>,
    /// API authentication key
    pub api_key: Option<String>,
    /// Default model to use
    pub default_model: Option<String>,
    /// Default output format
    pub output_format: Option<String>,
    /// Request timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Custom configuration values
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl CliConfig {
    /// Load configuration from default path
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to default path
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;

        Ok(())
    }

    /// Get the default configuration file path
    pub fn config_path() -> anyhow::Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        Ok(config_dir.join("copilot").join("config.toml"))
    }

    /// Get a configuration value
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "api_url" => self.api_url.clone(),
            "api_key" => self.api_key.clone(),
            "default_model" => self.default_model.clone(),
            "output_format" => self.output_format.clone(),
            "timeout_seconds" => self.timeout_seconds.map(|t| t.to_string()),
            _ => self.custom.get(key).cloned(),
        }
    }

    /// Set a configuration value
    pub fn set(&mut self, key: &str, value: String) -> anyhow::Result<()> {
        match key {
            "api_url" => self.api_url = Some(value),
            "api_key" => self.api_key = Some(value),
            "default_model" => self.default_model = Some(value),
            "output_format" => self.output_format = Some(value),
            "timeout_seconds" => {
                self.timeout_seconds = Some(value.parse()?);
            }
            _ => {
                self.custom.insert(key.to_string(), value);
            }
        }
        Ok(())
    }

    /// Merge with command line options
    pub fn with_overrides(
        &self,
        api_url: Option<&str>,
        api_key: Option<&str>,
    ) -> (String, Option<String>) {
        let url = api_url
            .map(String::from)
            .or_else(|| self.api_url.clone())
            .unwrap_or_else(|| "http://localhost:8080".to_string());

        let key = api_key.map(String::from).or_else(|| self.api_key.clone());

        (url, key)
    }
}
