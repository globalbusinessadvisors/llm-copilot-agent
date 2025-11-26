//! LLM CoPilot Agent CLI
//!
//! A command-line interface for interacting with the LLM CoPilot Agent platform.

mod commands;
mod config;
mod output;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "copilot",
    author = "LLM-CoPilot-Agent Team",
    version,
    about = "LLM CoPilot Agent - AI-powered development assistant",
    long_about = "A command-line interface for the LLM CoPilot Agent platform.\n\n\
                  Use this CLI to interact with AI agents, manage conversations,\n\
                  execute workflows, and integrate with your development environment."
)]
struct Cli {
    /// API endpoint URL
    #[arg(
        short,
        long,
        env = "COPILOT_API_URL",
        default_value = "http://localhost:8080"
    )]
    api_url: String,

    /// API key for authentication
    #[arg(short = 'k', long, env = "COPILOT_API_KEY")]
    api_key: Option<String>,

    /// Output format (text, json, yaml)
    #[arg(
        short,
        long,
        default_value = "text",
        value_parser = ["text", "json", "yaml"]
    )]
    format: String,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive chat session
    Chat {
        /// Initial message to send
        #[arg(short, long)]
        message: Option<String>,

        /// Session ID to resume
        #[arg(short, long)]
        session: Option<String>,

        /// Model to use
        #[arg(long, default_value = "default")]
        model: String,
    },

    /// Send a single message and get a response
    Ask {
        /// The question or prompt to send
        message: String,

        /// Include context from a file
        #[arg(short, long)]
        context: Option<String>,

        /// Model to use
        #[arg(long, default_value = "default")]
        model: String,
    },

    /// Manage conversations
    #[command(subcommand)]
    Conversation(ConversationCommands),

    /// Manage and execute workflows
    #[command(subcommand)]
    Workflow(WorkflowCommands),

    /// Execute code in a sandboxed environment
    #[command(subcommand)]
    Sandbox(SandboxCommands),

    /// Manage context and memory
    #[command(subcommand)]
    Context(ContextCommands),

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Server management (start, stop, status)
    #[command(subcommand)]
    Server(ServerCommands),

    /// Check API connectivity and health
    Health {
        /// Include detailed component health
        #[arg(short, long)]
        detailed: bool,
    },

    /// Display version information
    Version {
        /// Show all component versions
        #[arg(short, long)]
        all: bool,
    },

    /// Initialize a new project with CoPilot
    Init {
        /// Project directory
        #[arg(default_value = ".")]
        path: String,

        /// Project template
        #[arg(short, long, default_value = "default")]
        template: String,
    },

    /// Generate shell completions
    Completions {
        /// Shell type
        #[arg(value_parser = ["bash", "zsh", "fish", "powershell"])]
        shell: String,
    },
}

#[derive(Subcommand)]
enum ConversationCommands {
    /// List all conversations
    List {
        /// Maximum number to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Show conversation details
    Show {
        /// Conversation ID
        id: String,
    },
    /// Delete a conversation
    Delete {
        /// Conversation ID
        id: String,
        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Export conversation history
    Export {
        /// Conversation ID
        id: String,
        /// Output file
        #[arg(short, long)]
        output: Option<String>,
        /// Export format
        #[arg(short, long, default_value = "json")]
        format: String,
    },
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// List available workflows
    List,
    /// Show workflow details
    Show {
        /// Workflow ID
        id: String,
    },
    /// Execute a workflow
    Run {
        /// Workflow ID or file path
        workflow: String,
        /// Input parameters (key=value)
        #[arg(short, long)]
        param: Vec<String>,
        /// Wait for completion
        #[arg(short, long)]
        wait: bool,
    },
    /// Check workflow execution status
    Status {
        /// Execution ID
        execution_id: String,
    },
    /// Cancel a running workflow
    Cancel {
        /// Execution ID
        execution_id: String,
    },
    /// Validate a workflow definition
    Validate {
        /// Workflow file path
        file: String,
    },
}

#[derive(Subcommand)]
enum SandboxCommands {
    /// Execute code in a sandbox
    Run {
        /// Code to execute (or use --file)
        #[arg(short, long)]
        code: Option<String>,
        /// File containing code to execute
        #[arg(short, long)]
        file: Option<String>,
        /// Runtime (python, nodejs, bash, rust, go)
        #[arg(short, long, default_value = "python")]
        runtime: String,
        /// Execution timeout in seconds
        #[arg(short, long, default_value = "60")]
        timeout: u64,
    },
    /// List active sandboxes
    List,
    /// Get sandbox status
    Status {
        /// Sandbox ID
        id: String,
    },
    /// Destroy a sandbox
    Destroy {
        /// Sandbox ID
        id: String,
    },
}

#[derive(Subcommand)]
enum ContextCommands {
    /// Add context from a file or directory
    Add {
        /// Path to file or directory
        path: String,
        /// Context tags
        #[arg(short, long)]
        tag: Vec<String>,
    },
    /// List stored context
    List {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },
    /// Clear context
    Clear {
        /// Clear specific tags only
        #[arg(short, long)]
        tag: Option<String>,
        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Search context
    Search {
        /// Search query
        query: String,
        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// Reset configuration to defaults
    Reset {
        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Open configuration in editor
    Edit,
}

#[derive(Subcommand)]
enum ServerCommands {
    /// Start the server
    Start {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
        /// Run in background
        #[arg(short, long)]
        daemon: bool,
    },
    /// Stop the server
    Stop,
    /// Show server status
    Status,
    /// Show server logs
    Logs {
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
        /// Number of lines to show
        #[arg(short, long, default_value = "100")]
        lines: usize,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Setup colored output
    if cli.no_color {
        colored::control::set_override(false);
    }

    // Execute command
    let result = match cli.command {
        Commands::Chat { message, session, model } => {
            commands::chat::run(&cli.api_url, cli.api_key.as_deref(), message, session, &model).await
        }
        Commands::Ask { message, context, model } => {
            commands::ask::run(&cli.api_url, cli.api_key.as_deref(), &message, context.as_deref(), &model, &cli.format).await
        }
        Commands::Conversation(cmd) => {
            commands::conversation::run(&cli.api_url, cli.api_key.as_deref(), cmd, &cli.format).await
        }
        Commands::Workflow(cmd) => {
            commands::workflow::run(&cli.api_url, cli.api_key.as_deref(), cmd, &cli.format).await
        }
        Commands::Sandbox(cmd) => {
            commands::sandbox::run(&cli.api_url, cli.api_key.as_deref(), cmd, &cli.format).await
        }
        Commands::Context(cmd) => {
            commands::context::run(&cli.api_url, cli.api_key.as_deref(), cmd, &cli.format).await
        }
        Commands::Config(cmd) => {
            commands::config::run(cmd).await
        }
        Commands::Server(cmd) => {
            commands::server::run(cmd).await
        }
        Commands::Health { detailed } => {
            commands::health::run(&cli.api_url, detailed, &cli.format).await
        }
        Commands::Version { all } => {
            commands::version::run(all, &cli.format).await
        }
        Commands::Init { path, template } => {
            commands::init::run(&path, &template).await
        }
        Commands::Completions { shell } => {
            commands::completions::run(&shell)
        }
    };

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            if cli.verbose {
                if let Some(source) = e.source() {
                    eprintln!("{}: {}", "Caused by".yellow(), source);
                }
            }
            ExitCode::FAILURE
        }
    }
}
