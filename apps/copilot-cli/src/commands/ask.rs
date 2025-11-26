//! Single question command

use anyhow::Result;
use colored::Colorize;
use copilot_sdk::CopilotClient;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub async fn run(
    api_url: &str,
    api_key: Option<&str>,
    message: &str,
    context_file: Option<&str>,
    model: &str,
    format: &str,
) -> Result<()> {
    let client = CopilotClient::builder()
        .base_url(api_url)
        .api_key(api_key.map(String::from))
        .build()?;

    // Load context if provided
    let context = if let Some(file) = context_file {
        Some(std::fs::read_to_string(file)?)
    } else {
        None
    };

    // Show spinner while waiting
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.cyan} {msg}")?,
    );
    spinner.set_message("Processing...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    // Send the question
    let model_opt = if model.is_empty() {
        None
    } else {
        Some(model.to_string())
    };

    let response = client
        .ask(message, context.as_deref(), model_opt)
        .await?;

    spinner.finish_and_clear();

    // Output based on format
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&response)?);
        }
        _ => {
            println!("{}", response.content);

            if let Some(usage) = &response.usage {
                eprintln!();
                eprintln!("{}", format!("[{} tokens used]", usage.total_tokens).dimmed());
            }
        }
    }

    Ok(())
}
