//! Sandbox execution commands

use crate::SandboxCommands;
use anyhow::Result;
use colored::Colorize;
use copilot_sdk::CopilotClient;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use tabled::{Table, Tabled};

pub async fn run(
    api_url: &str,
    api_key: Option<&str>,
    cmd: SandboxCommands,
    format: &str,
) -> Result<()> {
    let client = CopilotClient::builder()
        .base_url(api_url)
        .api_key(api_key.map(String::from))
        .build()?;

    match cmd {
        SandboxCommands::Run { code, file, runtime, timeout } => {
            run_code(&client, code, file, &runtime, timeout, format).await
        }
        SandboxCommands::List => list_sandboxes(&client, format).await,
        SandboxCommands::Status { id } => sandbox_status(&client, &id, format).await,
        SandboxCommands::Destroy { id } => destroy_sandbox(&client, &id).await,
    }
}

async fn run_code(
    client: &CopilotClient,
    code: Option<String>,
    file: Option<String>,
    runtime: &str,
    timeout: u64,
    format: &str,
) -> Result<()> {
    // Get code from argument or file
    let code = match (code, file) {
        (Some(c), _) => c,
        (None, Some(f)) => std::fs::read_to_string(&f)?,
        (None, None) => {
            // Read from stdin
            use std::io::Read;
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.cyan} {msg}")?,
    );
    spinner.set_message(format!("Executing {} code...", runtime));
    spinner.enable_steady_tick(Duration::from_millis(80));

    let result = client.execute_code(&code, runtime, timeout).await?;

    spinner.finish_and_clear();

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&result)?);
        }
        _ => {
            if result.success {
                println!("{}", "✓ Execution successful".green());
            } else {
                println!("{}", "✗ Execution failed".red());
            }

            println!();

            if !result.stdout.is_empty() {
                println!("{}", "Output:".bold());
                println!("{}", result.stdout);
            }

            if !result.stderr.is_empty() {
                println!();
                println!("{}", "Errors:".bold().red());
                println!("{}", result.stderr);
            }

            println!();
            println!(
                "{}",
                format!(
                    "[Exit code: {} | Duration: {}ms]",
                    result.exit_code, result.duration_ms
                )
                .dimmed()
            );
        }
    }

    if !result.success {
        std::process::exit(result.exit_code);
    }

    Ok(())
}

async fn list_sandboxes(client: &CopilotClient, format: &str) -> Result<()> {
    let sandboxes = client.list_sandboxes().await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&sandboxes)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&sandboxes)?);
        }
        _ => {
            if sandboxes.is_empty() {
                println!("{}", "No active sandboxes.".dimmed());
                return Ok(());
            }

            #[derive(Tabled)]
            struct SandboxRow {
                #[tabled(rename = "ID")]
                id: String,
                #[tabled(rename = "Template")]
                template: String,
                #[tabled(rename = "Status")]
                status: String,
                #[tabled(rename = "Created")]
                created: String,
            }

            let rows: Vec<SandboxRow> = sandboxes
                .iter()
                .map(|s| SandboxRow {
                    id: s.id[..8.min(s.id.len())].to_string(),
                    template: s.template.clone(),
                    status: s.status.clone(),
                    created: s.created_at.clone(),
                })
                .collect();

            let table = Table::new(rows).to_string();
            println!("{}", table);
        }
    }

    Ok(())
}

async fn sandbox_status(client: &CopilotClient, id: &str, format: &str) -> Result<()> {
    let sandbox = client.get_sandbox(id).await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&sandbox)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&sandbox)?);
        }
        _ => {
            let status_color = match sandbox.status.as_str() {
                "running" => sandbox.status.green(),
                "paused" => sandbox.status.yellow(),
                "stopped" | "error" => sandbox.status.red(),
                _ => sandbox.status.normal(),
            };

            println!("{}: {}", "ID".bold(), sandbox.id);
            println!("{}: {}", "Template".bold(), sandbox.template);
            println!("{}: {}", "Status".bold(), status_color);
            println!("{}: {}", "Created".bold(), sandbox.created_at);

            if let Some(last_activity) = sandbox.last_activity {
                println!("{}: {}", "Last Activity".bold(), last_activity);
            }
        }
    }

    Ok(())
}

async fn destroy_sandbox(client: &CopilotClient, id: &str) -> Result<()> {
    client.destroy_sandbox(id).await?;
    println!("{} sandbox {}", "Destroyed".green(), id);
    Ok(())
}
