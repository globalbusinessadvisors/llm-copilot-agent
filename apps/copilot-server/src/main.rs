mod app;
mod cli;
mod server;
mod telemetry;

use anyhow::Result;
use clap::Parser;
use tracing::{info, error};

use crate::cli::Args;
use crate::app::App;
use crate::telemetry::init_telemetry;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file if it exists
    dotenv::dotenv().ok();

    // Parse command line arguments
    let args = Args::parse();

    // Initialize telemetry (logging, tracing, metrics)
    let _guards = init_telemetry(&args)?;

    info!("Starting LLM CoPilot Agent Server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Environment: {}", args.env);

    // Build and run the application
    let result = run_application(args).await;

    // Log any errors that occurred
    if let Err(ref e) = result {
        error!("Application error: {:#}", e);
    }

    info!("Server shutdown complete");

    result
}

async fn run_application(args: Args) -> Result<()> {
    // Build the application with all dependencies
    let app = App::build(args).await?;

    // Run the application until shutdown signal
    app.run().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Args::command().debug_assert()
    }
}
