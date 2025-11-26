//! Context management commands

use crate::ContextCommands;
use anyhow::Result;
use colored::Colorize;
use copilot_sdk::CopilotClient;
use dialoguer::Confirm;
use tabled::{Table, Tabled};

pub async fn run(
    api_url: &str,
    api_key: Option<&str>,
    cmd: ContextCommands,
    format: &str,
) -> Result<()> {
    let client = CopilotClient::builder()
        .base_url(api_url)
        .api_key(api_key.map(String::from))
        .build()?;

    match cmd {
        ContextCommands::Add { path, tag } => add_context(&client, &path, tag).await,
        ContextCommands::List { tag } => list_context(&client, tag, format).await,
        ContextCommands::Clear { tag, force } => clear_context(&client, tag, force).await,
        ContextCommands::Search { query, limit } => search_context(&client, &query, limit, format).await,
    }
}

async fn add_context(client: &CopilotClient, path: &str, tags: Vec<String>) -> Result<()> {
    let metadata = std::fs::metadata(path)?;

    if metadata.is_file() {
        let content = std::fs::read_to_string(path)?;
        client.add_context(path, &content, tags.clone()).await?;
        println!("{} context from {}", "Added".green(), path.cyan());
    } else if metadata.is_dir() {
        let mut count = 0;
        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();

            // Skip binary and hidden files
            if let Some(ext) = file_path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if matches!(
                    ext.as_str(),
                    "exe" | "dll" | "so" | "dylib" | "bin" | "o" | "a"
                ) {
                    continue;
                }
            }

            if let Some(name) = file_path.file_name() {
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
            }

            if let Ok(content) = std::fs::read_to_string(file_path) {
                let path_str = file_path.to_string_lossy();
                client
                    .add_context(&path_str, &content, tags.clone())
                    .await?;
                count += 1;
            }
        }
        println!("{} {} files to context", "Added".green(), count);
    }

    Ok(())
}

async fn list_context(client: &CopilotClient, tag: Option<String>, format: &str) -> Result<()> {
    let context_items = client.list_context(tag).await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&context_items)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&context_items)?);
        }
        _ => {
            if context_items.is_empty() {
                println!("{}", "No context items found.".dimmed());
                return Ok(());
            }

            #[derive(Tabled)]
            struct ContextRow {
                #[tabled(rename = "ID")]
                id: String,
                #[tabled(rename = "Source")]
                source: String,
                #[tabled(rename = "Size")]
                size: String,
                #[tabled(rename = "Tags")]
                tags: String,
            }

            let rows: Vec<ContextRow> = context_items
                .iter()
                .map(|c| ContextRow {
                    id: c.id[..8.min(c.id.len())].to_string(),
                    source: c.source.clone().unwrap_or_else(|| "unknown".to_string()),
                    size: format_size(c.size),
                    tags: c.tags.join(", "),
                })
                .collect();

            let table = Table::new(rows).to_string();
            println!("{}", table);
        }
    }

    Ok(())
}

async fn clear_context(client: &CopilotClient, tag: Option<String>, force: bool) -> Result<()> {
    if !force {
        let msg = match &tag {
            Some(t) => format!("Clear all context with tag '{}'?", t),
            None => "Clear ALL context?".to_string(),
        };

        let confirmed = Confirm::new()
            .with_prompt(msg)
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Cancelled.".yellow());
            return Ok(());
        }
    }

    client.clear_context(tag.clone()).await?;

    match tag {
        Some(t) => println!("{} context with tag '{}'", "Cleared".green(), t),
        None => println!("{} all context", "Cleared".green()),
    }

    Ok(())
}

async fn search_context(
    client: &CopilotClient,
    query: &str,
    limit: usize,
    format: &str,
) -> Result<()> {
    let results = client.search_context(query, limit).await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&results)?);
        }
        _ => {
            if results.is_empty() {
                println!("{}", "No matching context found.".dimmed());
                return Ok(());
            }

            println!("{} {} results:\n", "Found".green(), results.len());

            for (i, result) in results.iter().enumerate() {
                println!(
                    "{}. {} (score: {:.2})",
                    i + 1,
                    result.source.as_ref().unwrap_or(&"unknown".to_string()).cyan(),
                    result.score
                );
                println!("   {}", truncate(&result.snippet, 100).dimmed());
                println!();
            }
        }
    }

    Ok(())
}

fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
