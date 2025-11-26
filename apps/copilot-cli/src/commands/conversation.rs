//! Conversation management commands

use crate::ConversationCommands;
use anyhow::Result;
use colored::Colorize;
use copilot_sdk::CopilotClient;
use dialoguer::Confirm;
use tabled::{Table, Tabled};

pub async fn run(
    api_url: &str,
    api_key: Option<&str>,
    cmd: ConversationCommands,
    format: &str,
) -> Result<()> {
    let client = CopilotClient::builder()
        .base_url(api_url)
        .api_key(api_key.map(String::from))
        .build()?;

    match cmd {
        ConversationCommands::List { limit } => {
            list_conversations(&client, limit, format).await
        }
        ConversationCommands::Show { id } => {
            show_conversation(&client, &id, format).await
        }
        ConversationCommands::Delete { id, force } => {
            delete_conversation(&client, &id, force).await
        }
        ConversationCommands::Export { id, output, format: export_format } => {
            export_conversation(&client, &id, output, &export_format).await
        }
    }
}

async fn list_conversations(client: &CopilotClient, limit: usize, format: &str) -> Result<()> {
    let conversations = client.list_conversations(limit).await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&conversations)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&conversations)?);
        }
        _ => {
            if conversations.is_empty() {
                println!("{}", "No conversations found.".dimmed());
                return Ok(());
            }

            #[derive(Tabled)]
            struct ConversationRow {
                #[tabled(rename = "ID")]
                id: String,
                #[tabled(rename = "Created")]
                created: String,
                #[tabled(rename = "Messages")]
                messages: usize,
                #[tabled(rename = "Model")]
                model: String,
            }

            let rows: Vec<ConversationRow> = conversations
                .iter()
                .map(|c| ConversationRow {
                    id: c.id[..8.min(c.id.len())].to_string(),
                    created: c.created_at.clone(),
                    messages: c.message_count,
                    model: c.model.clone().unwrap_or_else(|| "default".to_string()),
                })
                .collect();

            let table = Table::new(rows).to_string();
            println!("{}", table);
        }
    }

    Ok(())
}

async fn show_conversation(client: &CopilotClient, id: &str, format: &str) -> Result<()> {
    let conversation = client.get_conversation(id).await?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&conversation)?);
        }
        "yaml" => {
            println!("{}", serde_yaml::to_string(&conversation)?);
        }
        _ => {
            println!("{}: {}", "ID".bold(), conversation.id);
            println!("{}: {}", "Created".bold(), conversation.created_at);
            println!("{}: {}", "Messages".bold(), conversation.messages.len());
            println!();

            for msg in conversation.messages {
                let role = match msg.role.as_str() {
                    "user" => "You".green(),
                    "assistant" => "Assistant".cyan(),
                    _ => msg.role.normal(),
                };
                println!("{}: {}", role.bold(), msg.content);
                println!();
            }
        }
    }

    Ok(())
}

async fn delete_conversation(client: &CopilotClient, id: &str, force: bool) -> Result<()> {
    if !force {
        let confirmed = Confirm::new()
            .with_prompt(format!("Delete conversation {}?", id))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Cancelled.".yellow());
            return Ok(());
        }
    }

    client.delete_conversation(id).await?;
    println!("{} conversation {}", "Deleted".green(), id);

    Ok(())
}

async fn export_conversation(
    client: &CopilotClient,
    id: &str,
    output: Option<String>,
    format: &str,
) -> Result<()> {
    let conversation = client.get_conversation(id).await?;

    let content = match format {
        "yaml" => serde_yaml::to_string(&conversation)?,
        "markdown" | "md" => {
            let mut md = format!("# Conversation {}\n\n", id);
            md.push_str(&format!("Created: {}\n\n", conversation.created_at));
            md.push_str("---\n\n");

            for msg in conversation.messages {
                let role = if msg.role == "user" { "**You**" } else { "**Assistant**" };
                md.push_str(&format!("{}\n\n{}\n\n---\n\n", role, msg.content));
            }
            md
        }
        _ => serde_json::to_string_pretty(&conversation)?,
    };

    match output {
        Some(path) => {
            std::fs::write(&path, content)?;
            println!("{} to {}", "Exported".green(), path.cyan());
        }
        None => {
            println!("{}", content);
        }
    }

    Ok(())
}
