//! Webhook support for LLM CoPilot Agent
//!
//! This crate provides webhook functionality:
//! - Outbound webhooks for event notifications
//! - Inbound webhooks for external triggers
//! - Webhook signature verification
//! - Retry policies and delivery tracking
//!
//! # Features
//!
//! - **Outbound Webhooks**: Send events to external endpoints with automatic retries
//! - **Inbound Webhooks**: Receive and process webhooks from external services
//! - **Signature Verification**: HMAC-SHA256 signature generation and verification
//! - **Delivery Tracking**: Track delivery status, attempts, and statistics
//! - **Pre-built Handlers**: GitHub, Stripe, and generic webhook handlers
//!
//! # Example
//!
//! ```rust,ignore
//! use copilot_webhook::{
//!     WebhookDispatcher, WebhookEndpoint, WebhookEvent, WebhookEventData,
//!     WebhookEventType, RetryConfig, ConversationEventData,
//! };
//! use tokio::sync::mpsc;
//!
//! // Create a dispatcher
//! let (tx, rx) = mpsc::channel(1000);
//! let dispatcher = WebhookDispatcher::new(tx, RetryConfig::default());
//!
//! // Register an endpoint
//! let endpoint = WebhookEndpoint::new("My Webhook", "https://example.com/webhook", "secret")
//!     .with_events(vec![WebhookEventType::ConversationCreated]);
//! dispatcher.register_endpoint(endpoint);
//!
//! // Dispatch an event
//! let event = WebhookEvent::new(
//!     WebhookEventType::ConversationCreated,
//!     WebhookEventData::Conversation(ConversationEventData {
//!         id: "conv-123".to_string(),
//!         user_id: "user-456".to_string(),
//!         title: Some("Test".to_string()),
//!         message_count: 1,
//!         created_at: chrono::Utc::now(),
//!         updated_at: chrono::Utc::now(),
//!     }),
//! );
//! dispatcher.dispatch(event).await;
//! ```

// Module order matters due to dependencies
pub mod events;
pub mod signature;
pub mod delivery;
pub mod outbound;
pub mod inbound;

pub use events::*;
pub use signature::*;
pub use delivery::*;
pub use outbound::*;
pub use inbound::*;

use thiserror::Error;

/// Webhook errors
#[derive(Error, Debug)]
pub enum WebhookError {
    #[error("Webhook not found: {0}")]
    NotFound(String),

    #[error("Invalid webhook URL: {0}")]
    InvalidUrl(String),

    #[error("Delivery failed: {0}")]
    DeliveryFailed(String),

    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Webhook disabled")]
    Disabled,

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type Result<T> = std::result::Result<T, WebhookError>;
