//! HTTP Server implementation

use anyhow::{Context, Result};
use axum::{
    Router,
    routing::get,
    response::Json,
    http::StatusCode,
};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::{
    trace::TraceLayer,
    cors::CorsLayer,
};
use tracing::info;

use copilot_api::create_router;
use copilot_api::AppState as ApiAppState;

use crate::app::AppState;
use crate::cli::Args;

pub struct Server {
    args: Args,
    state: AppState,
}

impl Server {
    pub fn new(args: Args, state: AppState) -> Result<Self> {
        Ok(Self { args, state })
    }

    pub async fn run(self) -> Result<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.args.port));

        // Build HTTP router
        let app = self.build_http_router();

        info!("HTTP server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .context("Failed to bind HTTP server")?;

        axum::serve(listener, app.into_make_service())
            .await
            .context("HTTP server error")?;

        Ok(())
    }

    fn build_http_router(&self) -> Router {
        // Create API app state
        let api_state = ApiAppState::new(
            self.state.engine.clone(),
            self.state.conversation_manager.clone(),
            self.state.jwt_secret.clone(),
        );

        // Create API router from copilot-api crate
        let api_router = create_router(api_state);

        // Combine routes
        Router::new()
            .route("/", get(root))
            .route("/health", get(health_check))
            .nest("/api", api_router)
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::permissive())
    }
}

// Route handlers

async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "service": "LLM CoPilot Agent",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running"
    }))
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_root_handler() {
        let response = root().await;
        assert_eq!(response.0["service"], "LLM CoPilot Agent");
    }

    #[tokio::test]
    async fn test_health_check_handler() {
        let status = health_check().await;
        assert_eq!(status, StatusCode::OK);
    }
}
