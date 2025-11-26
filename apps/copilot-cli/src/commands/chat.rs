//! Interactive chat command

use anyhow::Result;
use colored::Colorize;
use copilot_sdk::CopilotClient;
use dialoguer::{theme::ColorfulTheme, Input};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub async fn run(
    api_url: &str,
    api_key: Option<&str>,
    initial_message: Option<String>,
    session_id: Option<String>,
    model: &str,
) -> Result<()> {
    let client = CopilotClient::builder()
        .base_url(api_url)
        .api_key(api_key.map(String::from))
        .build()?;

    // Create or resume session
    let model_opt = if model.is_empty() {
        None
    } else {
        Some(model.to_string())
    };

    let session = match session_id {
        Some(id) => {
            println!("{} session {}", "Resuming".green(), id.cyan());
            client.resume_session(&id).await?
        }
        None => {
            println!("{} new chat session...", "Starting".green());
            client.create_session(model_opt).await?
        }
    };

    println!("Session ID: {}", session.id.cyan());
    println!("{}", "Type 'exit' or 'quit' to end the session.".dimmed());
    println!("{}", "Type '/help' for available commands.".dimmed());
    println!();

    // Send initial message if provided
    if let Some(msg) = initial_message {
        send_message(&client, &session.id, &msg).await?;
    }

    // Interactive loop
    loop {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("You")
            .allow_empty(true)
            .interact_text()?;

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        // Handle special commands
        match input.to_lowercase().as_str() {
            "exit" | "quit" | "/exit" | "/quit" => {
                println!("{}", "Goodbye!".green());
                break;
            }
            "/help" => {
                print_help();
                continue;
            }
            "/clear" => {
                print!("\x1B[2J\x1B[1;1H"); // Clear screen
                continue;
            }
            "/history" => {
                show_history(&client, &session.id).await?;
                continue;
            }
            "/export" => {
                export_session(&client, &session.id).await?;
                continue;
            }
            _ => {}
        }

        // Send message
        send_message(&client, &session.id, input).await?;
    }

    Ok(())
}

async fn send_message(client: &CopilotClient, session_id: &str, message: &str) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.cyan} {msg}")?,
    );
    spinner.set_message("Thinking...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let response = client.send_message(session_id, message).await?;

    spinner.finish_and_clear();

    // Print response
    println!();
    println!("{}: {}", "Assistant".cyan().bold(), response.content);
    println!();

    // Print metadata if available
    if let Some(usage) = &response.usage {
        println!("{}", format!("[{} tokens]", usage.total_tokens).dimmed());
    }

    Ok(())
}

fn print_help() {
    println!();
    println!("{}", "Available Commands:".yellow().bold());
    println!("  {}  - End the chat session", "/exit, /quit".cyan());
    println!("  {}      - Show this help message", "/help".cyan());
    println!("  {}     - Clear the screen", "/clear".cyan());
    println!("  {}   - Show conversation history", "/history".cyan());
    println!("  {}    - Export conversation to file", "/export".cyan());
    println!();
}

async fn show_history(client: &CopilotClient, session_id: &str) -> Result<()> {
    let history = client.get_history(session_id).await?;

    println!();
    println!("{}", "Conversation History:".yellow().bold());
    println!("{}", "=".repeat(50));

    for msg in history {
        let role_str = match msg.role.as_str() {
            "user" => "You".green(),
            "assistant" => "Assistant".cyan(),
            _ => msg.role.normal(),
        };
        println!("{}: {}", role_str.bold(), msg.content);
        println!();
    }

    Ok(())
}

async fn export_session(client: &CopilotClient, session_id: &str) -> Result<()> {
    let history = client.get_history(session_id).await?;
    let filename = format!("conversation_{}.json", session_id);

    let json = serde_json::to_string_pretty(&history)?;
    std::fs::write(&filename, json)?;

    println!("{} conversation to {}", "Exported".green(), filename.cyan());
    Ok(())
}
