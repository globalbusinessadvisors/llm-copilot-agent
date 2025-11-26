//! Workflow management commands

use crate::WorkflowCommands;
use anyhow::Result;
use colored::Colorize;
use copilot_sdk::CopilotClient;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::time::Duration;
use tabled::{Table, Tabled};

pub async fn run(
    api_url: &str,
    api_key: Option<&str>,
    cmd: WorkflowCommands,
    format: &str,
) -> Result<()> {
    let client = CopilotClient::builder()
        .base_url(api_url)
        .api_key(api_key.map(String::from))
        .build()?;

    match cmd {
        WorkflowCommands::List => list_workflows(&client, format).await,
        WorkflowCommands::Show { id } => show_workflow(&client, &id, format).await,
        WorkflowCommands::Run { workflow, param, wait } => {
            run_workflow(&client, &workflow, param, wait, format).await
        }
        WorkflowCommands::Status { execution_id } => {
            workflow_status(&client, &execution_id, format).await
        }
        WorkflowCommands::Cancel { execution_id } => {
            cancel_workflow(&client, &execution_id).await
        }
        WorkflowCommands::Validate { file } => validate_workflow(&file).await,
    }
}

async fn list_workflows(client: &CopilotClient, format: &str) -> Result<()> {
    let workflows = client.list_workflows().await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&workflows)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&workflows)?);
        }
        _ => {
            if workflows.is_empty() {
                println!("{}", "No workflows found.".dimmed());
                return Ok(());
            }

            #[derive(Tabled)]
            struct WorkflowRow {
                #[tabled(rename = "ID")]
                id: String,
                #[tabled(rename = "Name")]
                name: String,
                #[tabled(rename = "Steps")]
                steps: usize,
                #[tabled(rename = "Version")]
                version: String,
            }

            let rows: Vec<WorkflowRow> = workflows
                .iter()
                .map(|w| WorkflowRow {
                    id: w.id.clone(),
                    name: w.name.clone(),
                    steps: w.step_count,
                    version: w.version.clone(),
                })
                .collect();

            let table = Table::new(rows).to_string();
            println!("{}", table);
        }
    }

    Ok(())
}

async fn show_workflow(client: &CopilotClient, id: &str, format: &str) -> Result<()> {
    let workflow = client.get_workflow(id).await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&workflow)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&workflow)?);
        }
        _ => {
            println!("{}: {}", "ID".bold(), workflow.id);
            println!("{}: {}", "Name".bold(), workflow.name);
            println!("{}: {}", "Description".bold(), workflow.description);
            println!("{}: {}", "Version".bold(), workflow.version);
            println!();
            println!("{}", "Steps:".bold());

            for (i, step) in workflow.steps.iter().enumerate() {
                println!("  {}. {} ({})", i + 1, step.name, step.step_type);
                if !step.dependencies.is_empty() {
                    println!("     Depends on: {}", step.dependencies.join(", "));
                }
            }
        }
    }

    Ok(())
}

async fn run_workflow(
    client: &CopilotClient,
    workflow: &str,
    params: Vec<String>,
    wait: bool,
    format: &str,
) -> Result<()> {
    // Parse parameters
    let mut input: HashMap<String, serde_json::Value> = HashMap::new();
    for param in params {
        if let Some((key, value)) = param.split_once('=') {
            // Try to parse as JSON, fall back to string
            let json_value = serde_json::from_str(value)
                .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
            input.insert(key.to_string(), json_value);
        }
    }

    println!("{} workflow {}...", "Starting".green(), workflow.cyan());

    let execution = client.start_workflow(workflow, input).await?;

    if !wait {
        match format {
            "json" => {
                println!("{}", serde_json::to_string_pretty(&execution)?);
            }
            _ => {
                println!("{}: {}", "Execution ID".bold(), execution.id);
                println!("{}: {}", "Status".bold(), execution.status);
                println!();
                println!(
                    "{}",
                    "Use 'copilot workflow status <execution_id>' to check progress.".dimmed()
                );
            }
        }
        return Ok(());
    }

    // Wait for completion
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.cyan} {msg}")?,
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    loop {
        let status = client.get_workflow_status(&execution.id).await?;
        pb.set_message(format!("Status: {} - Step: {}", status.status, status.current_step));

        match status.status.as_str() {
            "completed" => {
                pb.finish_with_message(format!("{}", "Completed!".green()));
                break;
            }
            "failed" => {
                pb.finish_with_message(format!("{}", "Failed!".red()));
                if let Some(error) = status.error {
                    eprintln!("{}: {}", "Error".red(), error);
                }
                anyhow::bail!("Workflow execution failed");
            }
            "cancelled" => {
                pb.finish_with_message(format!("{}", "Cancelled".yellow()));
                break;
            }
            _ => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Ok(())
}

async fn workflow_status(client: &CopilotClient, execution_id: &str, format: &str) -> Result<()> {
    let status = client.get_workflow_status(execution_id).await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&status)?);
        }
        _ => {
            let status_color = match status.status.as_str() {
                "completed" => status.status.green(),
                "running" | "pending" => status.status.yellow(),
                "failed" => status.status.red(),
                _ => status.status.normal(),
            };

            println!("{}: {}", "Execution ID".bold(), execution_id);
            println!("{}: {}", "Status".bold(), status_color);
            println!("{}: {}", "Current Step".bold(), status.current_step);

            if let Some(progress) = status.progress {
                println!("{}: {}%", "Progress".bold(), progress);
            }

            if let Some(started) = status.started_at {
                println!("{}: {}", "Started".bold(), started);
            }

            if let Some(ended) = status.ended_at {
                println!("{}: {}", "Ended".bold(), ended);
            }
        }
    }

    Ok(())
}

async fn cancel_workflow(client: &CopilotClient, execution_id: &str) -> Result<()> {
    client.cancel_workflow(execution_id).await?;
    println!("{} workflow execution {}", "Cancelled".yellow(), execution_id);
    Ok(())
}

async fn validate_workflow(file: &str) -> Result<()> {
    let content = std::fs::read_to_string(file)?;

    // Try to parse as YAML first, then JSON
    let definition: serde_json::Value = if file.ends_with(".yaml") || file.ends_with(".yml") {
        serde_yaml::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };

    // Basic validation
    let mut errors: Vec<String> = Vec::new();

    if definition.get("name").is_none() {
        errors.push("Missing required field: name".to_string());
    }

    if definition.get("steps").is_none() {
        errors.push("Missing required field: steps".to_string());
    } else if let Some(steps) = definition.get("steps").and_then(|s| s.as_array()) {
        if steps.is_empty() {
            errors.push("Workflow must have at least one step".to_string());
        }

        for (i, step) in steps.iter().enumerate() {
            if step.get("id").is_none() {
                errors.push(format!("Step {} missing required field: id", i + 1));
            }
            if step.get("type").is_none() {
                errors.push(format!("Step {} missing required field: type", i + 1));
            }
        }
    }

    if errors.is_empty() {
        println!("{} Workflow definition is valid!", "✓".green());
    } else {
        println!("{} Workflow validation failed:", "✗".red());
        for error in errors {
            println!("  - {}", error);
        }
        anyhow::bail!("Validation failed");
    }

    Ok(())
}
