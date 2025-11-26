//! Webhook event types
//!
//! Defines all webhook event types and payloads.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Webhook event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    // Conversation events
    ConversationCreated,
    ConversationUpdated,
    ConversationDeleted,
    MessageCreated,
    MessageUpdated,

    // Workflow events
    WorkflowStarted,
    WorkflowCompleted,
    WorkflowFailed,
    WorkflowStepCompleted,

    // User events
    UserCreated,
    UserUpdated,
    UserDeleted,
    UserLogin,
    UserLogout,

    // Tenant events
    TenantCreated,
    TenantUpdated,
    TenantSuspended,
    TenantDeleted,

    // API key events
    ApiKeyCreated,
    ApiKeyRevoked,

    // Context events
    ContextItemCreated,
    ContextItemUpdated,
    ContextItemDeleted,

    // Billing events
    SubscriptionCreated,
    SubscriptionUpdated,
    SubscriptionCanceled,
    InvoiceCreated,
    InvoicePaid,
    PaymentFailed,

    // System events
    SystemAlert,
    QuotaWarning,
    QuotaExceeded,

    // Custom events
    Custom,
}

impl WebhookEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConversationCreated => "conversation.created",
            Self::ConversationUpdated => "conversation.updated",
            Self::ConversationDeleted => "conversation.deleted",
            Self::MessageCreated => "message.created",
            Self::MessageUpdated => "message.updated",
            Self::WorkflowStarted => "workflow.started",
            Self::WorkflowCompleted => "workflow.completed",
            Self::WorkflowFailed => "workflow.failed",
            Self::WorkflowStepCompleted => "workflow.step.completed",
            Self::UserCreated => "user.created",
            Self::UserUpdated => "user.updated",
            Self::UserDeleted => "user.deleted",
            Self::UserLogin => "user.login",
            Self::UserLogout => "user.logout",
            Self::TenantCreated => "tenant.created",
            Self::TenantUpdated => "tenant.updated",
            Self::TenantSuspended => "tenant.suspended",
            Self::TenantDeleted => "tenant.deleted",
            Self::ApiKeyCreated => "api_key.created",
            Self::ApiKeyRevoked => "api_key.revoked",
            Self::ContextItemCreated => "context.created",
            Self::ContextItemUpdated => "context.updated",
            Self::ContextItemDeleted => "context.deleted",
            Self::SubscriptionCreated => "subscription.created",
            Self::SubscriptionUpdated => "subscription.updated",
            Self::SubscriptionCanceled => "subscription.canceled",
            Self::InvoiceCreated => "invoice.created",
            Self::InvoicePaid => "invoice.paid",
            Self::PaymentFailed => "payment.failed",
            Self::SystemAlert => "system.alert",
            Self::QuotaWarning => "quota.warning",
            Self::QuotaExceeded => "quota.exceeded",
            Self::Custom => "custom",
        }
    }

    /// Get category for this event type
    pub fn category(&self) -> &'static str {
        match self {
            Self::ConversationCreated
            | Self::ConversationUpdated
            | Self::ConversationDeleted
            | Self::MessageCreated
            | Self::MessageUpdated => "conversation",
            Self::WorkflowStarted
            | Self::WorkflowCompleted
            | Self::WorkflowFailed
            | Self::WorkflowStepCompleted => "workflow",
            Self::UserCreated
            | Self::UserUpdated
            | Self::UserDeleted
            | Self::UserLogin
            | Self::UserLogout => "user",
            Self::TenantCreated
            | Self::TenantUpdated
            | Self::TenantSuspended
            | Self::TenantDeleted => "tenant",
            Self::ApiKeyCreated | Self::ApiKeyRevoked => "api_key",
            Self::ContextItemCreated
            | Self::ContextItemUpdated
            | Self::ContextItemDeleted => "context",
            Self::SubscriptionCreated
            | Self::SubscriptionUpdated
            | Self::SubscriptionCanceled
            | Self::InvoiceCreated
            | Self::InvoicePaid
            | Self::PaymentFailed => "billing",
            Self::SystemAlert | Self::QuotaWarning | Self::QuotaExceeded => "system",
            Self::Custom => "custom",
        }
    }
}

/// Webhook event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Event ID
    pub id: String,
    /// Event type
    #[serde(rename = "type")]
    pub event_type: WebhookEventType,
    /// API version
    pub api_version: String,
    /// Event timestamp
    pub created_at: DateTime<Utc>,
    /// Tenant ID (if applicable)
    pub tenant_id: Option<String>,
    /// Event data
    pub data: WebhookEventData,
    /// Previous data (for update events)
    pub previous_data: Option<serde_json::Value>,
    /// Request ID that triggered this event
    pub request_id: Option<String>,
}

impl WebhookEvent {
    /// Create a new webhook event
    pub fn new(event_type: WebhookEventType, data: WebhookEventData) -> Self {
        Self {
            id: format!("evt_{}", Uuid::new_v4().to_string().replace('-', "")),
            event_type,
            api_version: "2024-01-01".to_string(),
            created_at: Utc::now(),
            tenant_id: None,
            data,
            previous_data: None,
            request_id: None,
        }
    }

    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    pub fn with_request_id(mut self, request_id: &str) -> Self {
        self.request_id = Some(request_id.to_string());
        self
    }

    pub fn with_previous_data(mut self, data: serde_json::Value) -> Self {
        self.previous_data = Some(data);
        self
    }

    /// Convert to JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// Webhook event data wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "object")]
pub enum WebhookEventData {
    #[serde(rename = "conversation")]
    Conversation(ConversationEventData),
    #[serde(rename = "message")]
    Message(MessageEventData),
    #[serde(rename = "workflow")]
    Workflow(WorkflowEventData),
    #[serde(rename = "user")]
    User(UserEventData),
    #[serde(rename = "tenant")]
    Tenant(TenantEventData),
    #[serde(rename = "api_key")]
    ApiKey(ApiKeyEventData),
    #[serde(rename = "context")]
    Context(ContextEventData),
    #[serde(rename = "subscription")]
    Subscription(SubscriptionEventData),
    #[serde(rename = "invoice")]
    Invoice(InvoiceEventData),
    #[serde(rename = "system")]
    System(SystemEventData),
    #[serde(rename = "custom")]
    Custom(CustomEventData),
}

/// Conversation event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEventData {
    pub id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub message_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Message event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEventData {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub tokens: Option<u32>,
    pub created_at: DateTime<Utc>,
}

/// Workflow event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEventData {
    pub id: String,
    pub workflow_id: String,
    pub name: String,
    pub status: String,
    pub current_step: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub output: Option<serde_json::Value>,
}

/// User event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEventData {
    pub id: String,
    pub username: String,
    pub email: String,
    pub roles: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Tenant event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantEventData {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub tier: String,
    pub status: String,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
}

/// API key event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEventData {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Context event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEventData {
    pub id: String,
    pub name: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
}

/// Subscription event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEventData {
    pub id: String,
    pub tenant_id: String,
    pub tier: String,
    pub status: String,
    pub current_period_start: DateTime<Utc>,
    pub current_period_end: DateTime<Utc>,
}

/// Invoice event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceEventData {
    pub id: String,
    pub tenant_id: String,
    pub number: String,
    pub status: String,
    pub amount_cents: i64,
    pub currency: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// System event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEventData {
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub details: HashMap<String, serde_json::Value>,
}

/// Custom event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEventData {
    pub event_name: String,
    pub data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_string() {
        assert_eq!(WebhookEventType::ConversationCreated.as_str(), "conversation.created");
        assert_eq!(WebhookEventType::WorkflowCompleted.as_str(), "workflow.completed");
    }

    #[test]
    fn test_event_category() {
        assert_eq!(WebhookEventType::ConversationCreated.category(), "conversation");
        assert_eq!(WebhookEventType::WorkflowStarted.category(), "workflow");
        assert_eq!(WebhookEventType::UserCreated.category(), "user");
    }

    #[test]
    fn test_webhook_event() {
        let data = WebhookEventData::Conversation(ConversationEventData {
            id: "conv-123".to_string(),
            user_id: "user-456".to_string(),
            title: Some("Test Conversation".to_string()),
            message_count: 5,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        let event = WebhookEvent::new(WebhookEventType::ConversationCreated, data)
            .with_tenant("tenant-789")
            .with_request_id("req-abc");

        assert!(event.id.starts_with("evt_"));
        assert_eq!(event.tenant_id, Some("tenant-789".to_string()));

        let json = event.to_json();
        assert!(json["id"].is_string());
        assert_eq!(json["type"], "conversation_created");
    }

    #[test]
    fn test_event_serialization() {
        let data = WebhookEventData::Workflow(WorkflowEventData {
            id: "run-123".to_string(),
            workflow_id: "wf-456".to_string(),
            name: "Test Workflow".to_string(),
            status: "completed".to_string(),
            current_step: None,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            error: None,
            output: Some(serde_json::json!({"result": "success"})),
        });

        let event = WebhookEvent::new(WebhookEventType::WorkflowCompleted, data);
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("workflow.completed") || json.contains("workflow_completed"));
    }
}
