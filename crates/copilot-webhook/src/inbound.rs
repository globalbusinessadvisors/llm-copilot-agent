//! Inbound webhook handling
//!
//! Provides handlers for receiving webhooks from external services.

use crate::{
    signature::WebhookVerifier,
    Result, WebhookError,
};
use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Inbound webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundWebhookConfig {
    /// Webhook ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Source identifier (e.g., "github", "stripe")
    pub source: String,
    /// Secret for signature verification
    pub secret: String,
    /// Signature header name
    pub signature_header: String,
    /// Whether webhook is active
    pub enabled: bool,
    /// Tenant ID (if applicable)
    pub tenant_id: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl InboundWebhookConfig {
    /// Create a new inbound webhook config
    pub fn new(name: &str, source: &str, secret: &str) -> Self {
        Self {
            id: format!("iwh_{}", Uuid::new_v4().to_string().replace('-', "")),
            name: name.to_string(),
            source: source.to_string(),
            secret: secret.to_string(),
            signature_header: "X-Webhook-Signature".to_string(),
            enabled: true,
            tenant_id: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_signature_header(mut self, header: &str) -> Self {
        self.signature_header = header.to_string();
        self
    }

    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }
}

/// Received webhook payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundWebhook {
    /// Webhook receipt ID
    pub id: String,
    /// Config ID that received this
    pub config_id: String,
    /// Source identifier
    pub source: String,
    /// Raw payload
    pub payload: serde_json::Value,
    /// Headers
    pub headers: Vec<(String, String)>,
    /// Remote address
    pub remote_addr: Option<String>,
    /// Received timestamp
    pub received_at: DateTime<Utc>,
    /// Processing status
    pub status: InboundWebhookStatus,
    /// Error message if failed
    pub error: Option<String>,
    /// Tenant ID
    pub tenant_id: Option<String>,
}

/// Status of an inbound webhook
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboundWebhookStatus {
    Received,
    Processing,
    Processed,
    Failed,
}

/// Webhook handler trait
#[async_trait]
pub trait WebhookHandler: Send + Sync {
    /// Handle an incoming webhook
    async fn handle(&self, webhook: &InboundWebhook) -> Result<()>;

    /// Get the source this handler processes
    fn source(&self) -> &str;
}

/// Registry for webhook handlers
pub struct WebhookHandlerRegistry {
    handlers: DashMap<String, Arc<dyn WebhookHandler>>,
}

impl WebhookHandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }

    /// Register a handler for a source
    pub fn register(&self, handler: Arc<dyn WebhookHandler>) {
        let source = handler.source().to_string();
        info!(source = %source, "Registering webhook handler");
        self.handlers.insert(source, handler);
    }

    /// Get handler for a source
    pub fn get(&self, source: &str) -> Option<Arc<dyn WebhookHandler>> {
        self.handlers.get(source).map(|h| h.clone())
    }

    /// Remove a handler
    pub fn remove(&self, source: &str) {
        self.handlers.remove(source);
    }
}

impl Default for WebhookHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared state for webhook routes
pub struct InboundWebhookState {
    configs: DashMap<String, InboundWebhookConfig>,
    handlers: Arc<WebhookHandlerRegistry>,
    event_sender: mpsc::Sender<InboundWebhook>,
}

impl InboundWebhookState {
    pub fn new(
        handlers: Arc<WebhookHandlerRegistry>,
        event_sender: mpsc::Sender<InboundWebhook>,
    ) -> Self {
        Self {
            configs: DashMap::new(),
            handlers,
            event_sender,
        }
    }

    /// Register a webhook configuration
    pub fn register_config(&self, config: InboundWebhookConfig) {
        info!(
            config_id = %config.id,
            source = %config.source,
            "Registering inbound webhook config"
        );
        self.configs.insert(config.id.clone(), config);
    }

    /// Get config by ID
    pub fn get_config(&self, id: &str) -> Option<InboundWebhookConfig> {
        self.configs.get(id).map(|c| c.clone())
    }

    /// Remove a config
    pub fn remove_config(&self, id: &str) {
        self.configs.remove(id);
    }

    /// List all configs
    pub fn list_configs(&self) -> Vec<InboundWebhookConfig> {
        self.configs.iter().map(|c| c.clone()).collect()
    }
}

/// Create Axum router for inbound webhooks
pub fn create_webhook_router(state: Arc<InboundWebhookState>) -> Router {
    Router::new()
        .route("/webhooks/:config_id", post(handle_webhook))
        .with_state(state)
}

/// Handle incoming webhook request
async fn handle_webhook(
    State(state): State<Arc<InboundWebhookState>>,
    Path(config_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Get config
    let config = match state.get_config(&config_id) {
        Some(c) => c,
        None => {
            warn!(config_id = %config_id, "Webhook config not found");
            return (StatusCode::NOT_FOUND, "Webhook not found").into_response();
        }
    };

    // Check if enabled
    if !config.enabled {
        warn!(config_id = %config_id, "Webhook is disabled");
        return (StatusCode::FORBIDDEN, "Webhook disabled").into_response();
    }

    // Verify signature if present
    if let Some(signature) = headers.get(&config.signature_header) {
        let signature_str = match signature.to_str() {
            Ok(s) => s,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Invalid signature header").into_response();
            }
        };

        let verifier = WebhookVerifier::new(&config.secret);
        if let Err(e) = verifier.verify(&body, signature_str) {
            warn!(
                config_id = %config_id,
                error = %e,
                "Webhook signature verification failed"
            );
            return (StatusCode::UNAUTHORIZED, "Invalid signature").into_response();
        }
    }

    // Parse payload
    let payload: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            warn!(config_id = %config_id, error = %e, "Failed to parse webhook payload");
            return (StatusCode::BAD_REQUEST, "Invalid JSON payload").into_response();
        }
    };

    // Create webhook record
    let webhook = InboundWebhook {
        id: format!("iwhr_{}", Uuid::new_v4().to_string().replace('-', "")),
        config_id: config.id.clone(),
        source: config.source.clone(),
        payload,
        headers: headers
            .iter()
            .filter_map(|(k, v)| {
                v.to_str()
                    .ok()
                    .map(|v| (k.as_str().to_string(), v.to_string()))
            })
            .collect(),
        remote_addr: None,
        received_at: Utc::now(),
        status: InboundWebhookStatus::Received,
        error: None,
        tenant_id: config.tenant_id.clone(),
    };

    info!(
        webhook_id = %webhook.id,
        config_id = %config_id,
        source = %config.source,
        "Received inbound webhook"
    );

    // Process webhook
    if let Some(handler) = state.handlers.get(&config.source) {
        let mut processed_webhook = webhook.clone();
        processed_webhook.status = InboundWebhookStatus::Processing;

        match handler.handle(&processed_webhook).await {
            Ok(()) => {
                processed_webhook.status = InboundWebhookStatus::Processed;
                debug!(webhook_id = %webhook.id, "Webhook processed successfully");
            }
            Err(e) => {
                processed_webhook.status = InboundWebhookStatus::Failed;
                processed_webhook.error = Some(e.to_string());
                error!(webhook_id = %webhook.id, error = %e, "Webhook processing failed");
            }
        }

        // Send to event channel
        let _ = state.event_sender.send(processed_webhook).await;
    } else {
        // No handler, just acknowledge receipt
        let _ = state.event_sender.send(webhook).await;
    }

    (StatusCode::OK, "OK").into_response()
}

/// Inbound webhook processor
pub struct InboundWebhookProcessor {
    receiver: mpsc::Receiver<InboundWebhook>,
    on_webhook: Option<Box<dyn Fn(InboundWebhook) + Send + Sync>>,
}

impl InboundWebhookProcessor {
    pub fn new(receiver: mpsc::Receiver<InboundWebhook>) -> Self {
        Self {
            receiver,
            on_webhook: None,
        }
    }

    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(InboundWebhook) + Send + Sync + 'static,
    {
        self.on_webhook = Some(Box::new(callback));
        self
    }

    pub async fn run(mut self) {
        info!("Starting inbound webhook processor");

        while let Some(webhook) = self.receiver.recv().await {
            if let Some(ref callback) = self.on_webhook {
                callback(webhook);
            }
        }

        info!("Inbound webhook processor stopped");
    }
}

/// Pre-built handlers for common webhook sources

/// GitHub webhook handler
pub struct GitHubWebhookHandler {
    event_sender: mpsc::Sender<GitHubEvent>,
}

/// GitHub event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubEvent {
    pub event_type: String,
    pub action: Option<String>,
    pub repository: Option<String>,
    pub sender: Option<String>,
    pub payload: serde_json::Value,
    pub received_at: DateTime<Utc>,
}

impl GitHubWebhookHandler {
    pub fn new(event_sender: mpsc::Sender<GitHubEvent>) -> Self {
        Self { event_sender }
    }
}

#[async_trait]
impl WebhookHandler for GitHubWebhookHandler {
    async fn handle(&self, webhook: &InboundWebhook) -> Result<()> {
        let event_type = webhook
            .headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == "x-github-event")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let event = GitHubEvent {
            event_type,
            action: webhook.payload.get("action").and_then(|v| v.as_str()).map(String::from),
            repository: webhook
                .payload
                .get("repository")
                .and_then(|r| r.get("full_name"))
                .and_then(|v| v.as_str())
                .map(String::from),
            sender: webhook
                .payload
                .get("sender")
                .and_then(|s| s.get("login"))
                .and_then(|v| v.as_str())
                .map(String::from),
            payload: webhook.payload.clone(),
            received_at: webhook.received_at,
        };

        self.event_sender
            .send(event)
            .await
            .map_err(|_| WebhookError::DeliveryFailed("Event channel closed".to_string()))?;

        Ok(())
    }

    fn source(&self) -> &str {
        "github"
    }
}

/// Stripe webhook handler
pub struct StripeWebhookHandler {
    event_sender: mpsc::Sender<StripeEvent>,
}

/// Stripe event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeEvent {
    pub id: String,
    pub event_type: String,
    pub object_type: Option<String>,
    pub object_id: Option<String>,
    pub livemode: bool,
    pub payload: serde_json::Value,
    pub received_at: DateTime<Utc>,
}

impl StripeWebhookHandler {
    pub fn new(event_sender: mpsc::Sender<StripeEvent>) -> Self {
        Self { event_sender }
    }
}

#[async_trait]
impl WebhookHandler for StripeWebhookHandler {
    async fn handle(&self, webhook: &InboundWebhook) -> Result<()> {
        let event = StripeEvent {
            id: webhook
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default(),
            event_type: webhook
                .payload
                .get("type")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default(),
            object_type: webhook
                .payload
                .get("data")
                .and_then(|d| d.get("object"))
                .and_then(|o| o.get("object"))
                .and_then(|v| v.as_str())
                .map(String::from),
            object_id: webhook
                .payload
                .get("data")
                .and_then(|d| d.get("object"))
                .and_then(|o| o.get("id"))
                .and_then(|v| v.as_str())
                .map(String::from),
            livemode: webhook
                .payload
                .get("livemode")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            payload: webhook.payload.clone(),
            received_at: webhook.received_at,
        };

        self.event_sender
            .send(event)
            .await
            .map_err(|_| WebhookError::DeliveryFailed("Event channel closed".to_string()))?;

        Ok(())
    }

    fn source(&self) -> &str {
        "stripe"
    }
}

/// Generic JSON webhook handler
pub struct GenericWebhookHandler {
    source_name: String,
    event_sender: mpsc::Sender<InboundWebhook>,
}

impl GenericWebhookHandler {
    pub fn new(source: &str, event_sender: mpsc::Sender<InboundWebhook>) -> Self {
        Self {
            source_name: source.to_string(),
            event_sender,
        }
    }
}

#[async_trait]
impl WebhookHandler for GenericWebhookHandler {
    async fn handle(&self, webhook: &InboundWebhook) -> Result<()> {
        self.event_sender
            .send(webhook.clone())
            .await
            .map_err(|_| WebhookError::DeliveryFailed("Event channel closed".to_string()))?;
        Ok(())
    }

    fn source(&self) -> &str {
        &self.source_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inbound_config_creation() {
        let config = InboundWebhookConfig::new("GitHub", "github", "secret123")
            .with_signature_header("X-Hub-Signature-256")
            .with_tenant("tenant-1");

        assert!(config.id.starts_with("iwh_"));
        assert_eq!(config.source, "github");
        assert_eq!(config.signature_header, "X-Hub-Signature-256");
        assert!(config.enabled);
    }

    #[test]
    fn test_handler_registry() {
        let registry = WebhookHandlerRegistry::new();
        let (tx, _rx) = mpsc::channel(100);

        let handler = Arc::new(GenericWebhookHandler::new("test", tx));
        registry.register(handler);

        assert!(registry.get("test").is_some());
        assert!(registry.get("unknown").is_none());

        registry.remove("test");
        assert!(registry.get("test").is_none());
    }

    #[tokio::test]
    async fn test_state_config_management() {
        let registry = Arc::new(WebhookHandlerRegistry::new());
        let (tx, _rx) = mpsc::channel(100);
        let state = InboundWebhookState::new(registry, tx);

        let config = InboundWebhookConfig::new("Test", "test", "secret");
        let config_id = config.id.clone();

        state.register_config(config);
        assert!(state.get_config(&config_id).is_some());

        let configs = state.list_configs();
        assert_eq!(configs.len(), 1);

        state.remove_config(&config_id);
        assert!(state.get_config(&config_id).is_none());
    }
}
