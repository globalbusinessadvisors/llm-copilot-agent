use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Newtype wrappers for type safety

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(Uuid);

impl ConversationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ConversationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Intent types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub category: IntentCategory,
    pub confidence: f32,
    pub entities: Vec<Entity>,
}

impl Intent {
    pub fn new(category: IntentCategory, confidence: f32) -> Self {
        Self {
            category,
            confidence,
            entities: Vec::new(),
        }
    }

    pub fn with_entities(mut self, entities: Vec<Entity>) -> Self {
        self.entities = entities;
        self
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    pub fn is_confident(&self) -> bool {
        self.confidence >= 0.7
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentCategory {
    MetricQuery,
    LogQuery,
    IncidentCreate,
    IncidentUpdate,
    IncidentResolve,
    WorkflowTrigger,
    WorkflowStatus,
    TestGenerate,
    TestExecute,
    DeploymentQuery,
    DeploymentTrigger,
    DocumentationSearch,
    ConfigurationUpdate,
    AlertQuery,
    AlertCreate,
    Unknown,
}

impl std::fmt::Display for IntentCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntentCategory::MetricQuery => write!(f, "metric_query"),
            IntentCategory::LogQuery => write!(f, "log_query"),
            IntentCategory::IncidentCreate => write!(f, "incident_create"),
            IntentCategory::IncidentUpdate => write!(f, "incident_update"),
            IntentCategory::IncidentResolve => write!(f, "incident_resolve"),
            IntentCategory::WorkflowTrigger => write!(f, "workflow_trigger"),
            IntentCategory::WorkflowStatus => write!(f, "workflow_status"),
            IntentCategory::TestGenerate => write!(f, "test_generate"),
            IntentCategory::TestExecute => write!(f, "test_execute"),
            IntentCategory::DeploymentQuery => write!(f, "deployment_query"),
            IntentCategory::DeploymentTrigger => write!(f, "deployment_trigger"),
            IntentCategory::DocumentationSearch => write!(f, "documentation_search"),
            IntentCategory::ConfigurationUpdate => write!(f, "configuration_update"),
            IntentCategory::AlertQuery => write!(f, "alert_query"),
            IntentCategory::AlertCreate => write!(f, "alert_create"),
            IntentCategory::Unknown => write!(f, "unknown"),
        }
    }
}

// Entity types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_type: String,
    pub value: String,
    pub confidence: f32,
    pub span: Option<Span>,
}

impl Entity {
    pub fn new(entity_type: String, value: String, confidence: f32) -> Self {
        Self {
            entity_type,
            value,
            confidence,
            span: None,
        }
    }

    pub fn with_span(mut self, start: usize, end: usize) -> Self {
        self.span = Some(Span { start, end });
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

// Message types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Message {
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            id: MessageId::new(),
            role,
            content,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn user(content: String) -> Self {
        Self::new(MessageRole::User, content)
    }

    pub fn assistant(content: String) -> Self {
        Self::new(MessageRole::Assistant, content)
    }

    pub fn system(content: String) -> Self {
        Self::new(MessageRole::System, content)
    }

    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
        }
    }
}

// Conversation types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    pub session_id: SessionId,
    pub messages: Vec<Message>,
    pub context: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    pub fn new(session_id: SessionId) -> Self {
        let now = Utc::now();
        Self {
            id: ConversationId::new(),
            session_id,
            messages: Vec::new(),
            context: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    pub fn set_context(&mut self, key: String, value: serde_json::Value) {
        self.context.insert(key, value);
        self.updated_at = Utc::now();
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn get_messages_by_role(&self, role: MessageRole) -> Vec<&Message> {
        self.messages.iter().filter(|m| m.role == role).collect()
    }
}

// Session types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub conversations: Vec<ConversationId>,
    pub token_count: usize,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn new(user_id: UserId) -> Self {
        Self {
            id: SessionId::new(),
            user_id,
            conversations: Vec::new(),
            token_count: 0,
            created_at: Utc::now(),
        }
    }

    pub fn add_conversation(&mut self, conversation_id: ConversationId) {
        self.conversations.push(conversation_id);
    }

    pub fn increment_tokens(&mut self, count: usize) {
        self.token_count += count;
    }

    pub fn conversation_count(&self) -> usize {
        self.conversations.len()
    }
}

/// The main CoPilot Engine that orchestrates all operations.
///
/// This is a placeholder struct that will be implemented with full
/// functionality as the system matures.
#[derive(Debug, Clone)]
pub struct CoPilotEngine {
    /// Engine configuration
    pub config: CoPilotEngineConfig,
}

/// Configuration for the CoPilot Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoPilotEngineConfig {
    /// Maximum tokens per conversation
    pub max_tokens: usize,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Enable debug mode
    pub debug: bool,
}

impl Default for CoPilotEngineConfig {
    fn default() -> Self {
        Self {
            max_tokens: 8192,
            timeout_secs: 30,
            debug: false,
        }
    }
}

impl CoPilotEngine {
    /// Create a new CoPilot engine with default configuration.
    pub fn new() -> Self {
        Self::with_config(CoPilotEngineConfig::default())
    }

    /// Create a new CoPilot engine with the given configuration.
    pub fn with_config(config: CoPilotEngineConfig) -> Self {
        Self { config }
    }

    /// Process a user query and return a response.
    pub async fn process_query(&self, query: &str) -> crate::AppResult<String> {
        // Placeholder implementation
        Ok(format!("Processed: {}", query))
    }
}

impl Default for CoPilotEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_creation() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello".to_string());
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_intent_confidence() {
        let intent = Intent::new(IntentCategory::MetricQuery, 0.8);
        assert!(intent.is_confident());

        let intent = Intent::new(IntentCategory::MetricQuery, 0.5);
        assert!(!intent.is_confident());
    }

    #[test]
    fn test_conversation_messages() {
        let session_id = SessionId::new();
        let mut conv = Conversation::new(session_id);

        conv.add_message(Message::user("Test".to_string()));
        assert_eq!(conv.message_count(), 1);

        let user_messages = conv.get_messages_by_role(MessageRole::User);
        assert_eq!(user_messages.len(), 1);
    }
}
