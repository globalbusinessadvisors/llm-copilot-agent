//! Configuration management commands

use crate::ConfigCommands;
use anyhow::Result;
use colored::Colorize;
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CliConfig {
    pub api_url: Option<String>,
    pub api_key: Option<String>,
    pub default_model: Option<String>,
    pub output_format: Option<String>,
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl CliConfig {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        Ok(config_dir.join("copilot").join("config.toml"))
    }
}

pub async fn run(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::Show => show_config().await,
        ConfigCommands::Set { key, value } => set_config(&key, &value).await,
        ConfigCommands::Get { key } => get_config(&key).await,
        ConfigCommands::Reset { force } => reset_config(force).await,
        ConfigCommands::Edit => edit_config().await,
    }
}

async fn show_config() -> Result<()> {
    let config = CliConfig::load()?;
    let path = CliConfig::config_path()?;

    println!("{}: {}", "Config file".bold(), path.display());
    println!();

    if let Some(url) = &config.api_url {
        println!("{}: {}", "api_url".cyan(), url);
    }
    if config.api_key.is_some() {
        println!("{}: {}", "api_key".cyan(), "[set]".dimmed());
    }
    if let Some(model) = &config.default_model {
        println!("{}: {}", "default_model".cyan(), model);
    }
    if let Some(format) = &config.output_format {
        println!("{}: {}", "output_format".cyan(), format);
    }
    if let Some(timeout) = config.timeout_seconds {
        println!("{}: {}s", "timeout_seconds".cyan(), timeout);
    }

    if !config.custom.is_empty() {
        println!();
        println!("{}", "Custom settings:".bold());
        for (key, value) in &config.custom {
            println!("  {}: {}", key.cyan(), value);
        }
    }

    Ok(())
}

async fn set_config(key: &str, value: &str) -> Result<()> {
    let mut config = CliConfig::load()?;

    match key {
        "api_url" => config.api_url = Some(value.to_string()),
        "api_key" => config.api_key = Some(value.to_string()),
        "default_model" => config.default_model = Some(value.to_string()),
        "output_format" => config.output_format = Some(value.to_string()),
        "timeout_seconds" => {
            config.timeout_seconds = Some(value.parse()?);
        }
        _ => {
            config.custom.insert(key.to_string(), value.to_string());
        }
    }

    config.save()?;
    println!("{} {} = {}", "Set".green(), key.cyan(), value);

    Ok(())
}

async fn get_config(key: &str) -> Result<()> {
    let config = CliConfig::load()?;

    let value = match key {
        "api_url" => config.api_url,
        "api_key" => config.api_key.map(|_| "[redacted]".to_string()),
        "default_model" => config.default_model,
        "output_format" => config.output_format,
        "timeout_seconds" => config.timeout_seconds.map(|t| t.to_string()),
        _ => config.custom.get(key).cloned(),
    };

    match value {
        Some(v) => println!("{}", v),
        None => println!("{}", "(not set)".dimmed()),
    }

    Ok(())
}

async fn reset_config(force: bool) -> Result<()> {
    if !force {
        let confirmed = Confirm::new()
            .with_prompt("Reset configuration to defaults?")
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Cancelled.".yellow());
            return Ok(());
        }
    }

    let path = CliConfig::config_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }

    println!("{} configuration", "Reset".green());
    Ok(())
}

async fn edit_config() -> Result<()> {
    let path = CliConfig::config_path()?;

    // Create default config if it doesn't exist
    if !path.exists() {
        let config = CliConfig::default();
        config.save()?;
    }

    // Get editor from environment
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        });

    // Open editor
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()?;

    if status.success() {
        // Validate the config after editing
        match CliConfig::load() {
            Ok(_) => println!("{} configuration updated", "✓".green()),
            Err(e) => {
                eprintln!("{} Invalid configuration: {}", "✗".red(), e);
                anyhow::bail!("Configuration validation failed");
            }
        }
    }

    Ok(())
}
