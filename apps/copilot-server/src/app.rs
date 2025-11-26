//! Application state and initialization

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::info;

use copilot_core::CoPilotEngine;
use copilot_conversation::ConversationManager;
use copilot_nlp::NlpEngineImpl;
use copilot_context::{ContextEngineImpl, ContextEngineConfig};

use crate::cli::Args;
use crate::server::Server;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// The CoPilot engine
    pub engine: Arc<CoPilotEngine>,
    /// Conversation manager
    pub conversation_manager: Arc<ConversationManager>,
    /// JWT secret for authentication
    pub jwt_secret: String,
}

impl AppState {
    /// Create a new application state with all dependencies
    pub async fn new() -> Result<Self> {
        info!("Initializing application components");

        // Initialize core engine
        let engine = Arc::new(CoPilotEngine::new());

        // Initialize NLP engine
        let nlp_engine = Arc::new(NlpEngineImpl::new());

        // Initialize context engine
        let context_engine = ContextEngineImpl::new(ContextEngineConfig::default())
            .map_err(|e| anyhow::anyhow!("Failed to create context engine: {}", e))?;
        let context_engine = Arc::new(context_engine);

        // Initialize conversation manager
        let conversation_manager = Arc::new(
            ConversationManager::new(nlp_engine, context_engine)
        );

        // JWT secret (should come from config in production)
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "default-dev-secret-change-in-production".to_string());

        Ok(Self {
            engine,
            conversation_manager,
            jwt_secret,
        })
    }
}

/// Main application
pub struct App {
    args: Args,
    state: AppState,
}

impl App {
    /// Build the application with all dependencies
    pub async fn build(args: Args) -> Result<Self> {
        // Validate arguments
        args.validate()
            .context("Invalid command line arguments")?;

        // Initialize application state
        let state = AppState::new().await?;

        Ok(Self { args, state })
    }

    /// Run the application
    pub async fn run(self) -> Result<()> {
        info!("Starting server");
        info!("HTTP port: {}", self.args.port);

        // Create and run server
        let server = Server::new(self.args, self.state)?;
        server.run().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_state_creation() {
        let result = AppState::new().await;
        assert!(result.is_ok());
    }
}
