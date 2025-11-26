//! Event-driven workflow triggers
//!
//! Provides event-based workflow triggering with pattern matching and filtering.

use crate::{engine::{WorkflowEngine, WorkflowDefinition}, Result, WorkflowError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Event source types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    /// Internal system events
    System,
    /// Webhook events
    Webhook,
    /// API events
    Api,
    /// Message queue events
    MessageQueue,
    /// Database change events
    Database,
    /// File system events
    FileSystem,
    /// Custom source
    Custom(String),
}

/// Event that can trigger workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEvent {
    /// Event ID
    pub id: String,
    /// Event type/name
    pub event_type: String,
    /// Event source
    pub source: EventSource,
    /// Event payload
    pub payload: serde_json::Value,
    /// Event metadata
    pub metadata: HashMap<String, String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Tenant ID
    pub tenant_id: Option<String>,
    /// Correlation ID
    pub correlation_id: Option<String>,
}

impl TriggerEvent {
    pub fn new(event_type: &str, source: EventSource, payload: serde_json::Value) -> Self {
        Self {
            id: format!(
                "evt_{}",
                uuid::Uuid::new_v4().to_string().replace('-', "")
            ),
            event_type: event_type.to_string(),
            source,
            payload,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            tenant_id: None,
            correlation_id: None,
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: &str) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }
}

/// Condition for trigger matching
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerCondition {
    /// Match exact event type
    EventType { value: String },
    /// Match event source
    EventSource { value: EventSource },
    /// Match payload field value
    PayloadField { path: String, value: serde_json::Value },
    /// Match payload field exists
    PayloadFieldExists { path: String },
    /// Match metadata value
    Metadata { key: String, value: String },
    /// Match tenant
    Tenant { tenant_id: String },
    /// Match all conditions (AND)
    All { conditions: Vec<TriggerCondition> },
    /// Match any condition (OR)
    Any { conditions: Vec<TriggerCondition> },
    /// Negate condition
    Not { condition: Box<TriggerCondition> },
    /// Match using regex on event type
    EventTypePattern { pattern: String },
}

impl TriggerCondition {
    /// Check if event matches this condition
    pub fn matches(&self, event: &TriggerEvent) -> bool {
        match self {
            TriggerCondition::EventType { value } => event.event_type == *value,
            TriggerCondition::EventSource { value } => &event.source == value,
            TriggerCondition::PayloadField { path, value } => {
                get_json_path(&event.payload, path)
                    .map(|v| v == value)
                    .unwrap_or(false)
            }
            TriggerCondition::PayloadFieldExists { path } => {
                get_json_path(&event.payload, path).is_some()
            }
            TriggerCondition::Metadata { key, value } => {
                event.metadata.get(key).map(|v| v == value).unwrap_or(false)
            }
            TriggerCondition::Tenant { tenant_id } => {
                event.tenant_id.as_ref() == Some(tenant_id)
            }
            TriggerCondition::All { conditions } => {
                conditions.iter().all(|c| c.matches(event))
            }
            TriggerCondition::Any { conditions } => {
                conditions.iter().any(|c| c.matches(event))
            }
            TriggerCondition::Not { condition } => !condition.matches(event),
            TriggerCondition::EventTypePattern { pattern } => {
                regex::Regex::new(pattern)
                    .map(|re| re.is_match(&event.event_type))
                    .unwrap_or(false)
            }
        }
    }
}

/// Get value at JSON path (simplified dot notation)
fn get_json_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        // Handle array index
        if let Ok(index) = part.parse::<usize>() {
            current = current.get(index)?;
        } else {
            current = current.get(part)?;
        }
    }

    Some(current)
}

/// Workflow trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    /// Trigger ID
    pub id: String,
    /// Trigger name
    pub name: String,
    /// Workflow ID to execute
    pub workflow_id: String,
    /// Trigger condition
    pub condition: TriggerCondition,
    /// Whether trigger is active
    pub enabled: bool,
    /// Input mapping (event fields to workflow inputs)
    pub input_mapping: HashMap<String, String>,
    /// Static inputs to pass
    pub static_inputs: serde_json::Value,
    /// Maximum executions per time window
    pub rate_limit: Option<RateLimit>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Tenant ID
    pub tenant_id: Option<String>,
    /// Priority (lower = higher priority)
    pub priority: i32,
    /// Tags
    pub tags: Vec<String>,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum executions
    pub max_executions: u32,
    /// Time window in seconds
    pub window_seconds: u64,
}

impl WorkflowTrigger {
    pub fn new(name: &str, workflow_id: &str, condition: TriggerCondition) -> Self {
        Self {
            id: format!(
                "trg_{}",
                uuid::Uuid::new_v4().to_string().replace('-', "")
            ),
            name: name.to_string(),
            workflow_id: workflow_id.to_string(),
            condition,
            enabled: true,
            input_mapping: HashMap::new(),
            static_inputs: serde_json::Value::Null,
            rate_limit: None,
            created_at: Utc::now(),
            tenant_id: None,
            priority: 100,
            tags: Vec::new(),
        }
    }

    pub fn with_input_mapping(mut self, event_path: &str, input_name: &str) -> Self {
        self.input_mapping
            .insert(event_path.to_string(), input_name.to_string());
        self
    }

    pub fn with_static_inputs(mut self, inputs: serde_json::Value) -> Self {
        self.static_inputs = inputs;
        self
    }

    pub fn with_rate_limit(mut self, max_executions: u32, window_seconds: u64) -> Self {
        self.rate_limit = Some(RateLimit {
            max_executions,
            window_seconds,
        });
        self
    }

    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Build workflow input from event
    pub fn build_input(&self, event: &TriggerEvent) -> serde_json::Value {
        let mut input = self.static_inputs.clone();

        // Apply input mappings
        for (event_path, input_name) in &self.input_mapping {
            if let Some(value) = get_json_path(&event.payload, event_path) {
                if let serde_json::Value::Object(ref mut map) = input {
                    map.insert(input_name.clone(), value.clone());
                } else {
                    let mut map = serde_json::Map::new();
                    map.insert(input_name.clone(), value.clone());
                    input = serde_json::Value::Object(map);
                }
            }
        }

        // Add event metadata
        if let serde_json::Value::Object(ref mut map) = input {
            map.insert(
                "_event_id".to_string(),
                serde_json::Value::String(event.id.clone()),
            );
            map.insert(
                "_event_type".to_string(),
                serde_json::Value::String(event.event_type.clone()),
            );
            if let Some(ref correlation_id) = event.correlation_id {
                map.insert(
                    "_correlation_id".to_string(),
                    serde_json::Value::String(correlation_id.clone()),
                );
            }
        }

        input
    }
}

/// Trigger repository trait
#[async_trait]
pub trait TriggerRepository: Send + Sync {
    async fn save(&self, trigger: &WorkflowTrigger) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<WorkflowTrigger>>;
    async fn list(&self) -> Result<Vec<WorkflowTrigger>>;
    async fn list_enabled(&self) -> Result<Vec<WorkflowTrigger>>;
    async fn list_by_workflow(&self, workflow_id: &str) -> Result<Vec<WorkflowTrigger>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn update(&self, trigger: &WorkflowTrigger) -> Result<()>;
}

/// In-memory trigger repository
pub struct InMemoryTriggerRepository {
    triggers: RwLock<HashMap<String, WorkflowTrigger>>,
}

impl InMemoryTriggerRepository {
    pub fn new() -> Self {
        Self {
            triggers: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryTriggerRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TriggerRepository for InMemoryTriggerRepository {
    async fn save(&self, trigger: &WorkflowTrigger) -> Result<()> {
        let mut triggers = self.triggers.write().await;
        triggers.insert(trigger.id.clone(), trigger.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<WorkflowTrigger>> {
        let triggers = self.triggers.read().await;
        Ok(triggers.get(id).cloned())
    }

    async fn list(&self) -> Result<Vec<WorkflowTrigger>> {
        let triggers = self.triggers.read().await;
        Ok(triggers.values().cloned().collect())
    }

    async fn list_enabled(&self) -> Result<Vec<WorkflowTrigger>> {
        let triggers = self.triggers.read().await;
        let mut enabled: Vec<_> = triggers.values().filter(|t| t.enabled).cloned().collect();
        enabled.sort_by_key(|t| t.priority);
        Ok(enabled)
    }

    async fn list_by_workflow(&self, workflow_id: &str) -> Result<Vec<WorkflowTrigger>> {
        let triggers = self.triggers.read().await;
        Ok(triggers
            .values()
            .filter(|t| t.workflow_id == workflow_id)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut triggers = self.triggers.write().await;
        triggers.remove(id);
        Ok(())
    }

    async fn update(&self, trigger: &WorkflowTrigger) -> Result<()> {
        let mut triggers = self.triggers.write().await;
        if triggers.contains_key(&trigger.id) {
            triggers.insert(trigger.id.clone(), trigger.clone());
            Ok(())
        } else {
            Err(WorkflowError::NotFound(trigger.id.clone()))
        }
    }
}

/// Rate limiter state
struct RateLimiterState {
    executions: Vec<DateTime<Utc>>,
}

/// Workflow definition provider trait for triggers
#[async_trait]
pub trait TriggerWorkflowProvider: Send + Sync {
    /// Get workflow definition by ID
    async fn get_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>>;
}

/// Event trigger manager
pub struct TriggerManager {
    repository: Arc<dyn TriggerRepository>,
    engine: Arc<WorkflowEngine>,
    provider: Arc<dyn TriggerWorkflowProvider>,
    rate_limiter: RwLock<HashMap<String, RateLimiterState>>,
}

impl TriggerManager {
    pub fn new(
        repository: Arc<dyn TriggerRepository>,
        engine: Arc<WorkflowEngine>,
        provider: Arc<dyn TriggerWorkflowProvider>,
    ) -> Self {
        Self {
            repository,
            engine,
            provider,
            rate_limiter: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new trigger
    pub async fn create(&self, trigger: WorkflowTrigger) -> Result<WorkflowTrigger> {
        self.repository.save(&trigger).await?;

        info!(
            trigger_id = %trigger.id,
            workflow_id = %trigger.workflow_id,
            "Created workflow trigger"
        );

        Ok(trigger)
    }

    /// Enable a trigger
    pub async fn enable(&self, id: &str) -> Result<()> {
        if let Some(mut trigger) = self.repository.get(id).await? {
            trigger.enabled = true;
            self.repository.update(&trigger).await?;
            info!(trigger_id = %id, "Enabled trigger");
            Ok(())
        } else {
            Err(WorkflowError::NotFound(id.to_string()))
        }
    }

    /// Disable a trigger
    pub async fn disable(&self, id: &str) -> Result<()> {
        if let Some(mut trigger) = self.repository.get(id).await? {
            trigger.enabled = false;
            self.repository.update(&trigger).await?;
            info!(trigger_id = %id, "Disabled trigger");
            Ok(())
        } else {
            Err(WorkflowError::NotFound(id.to_string()))
        }
    }

    /// Delete a trigger
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.repository.delete(id).await?;
        info!(trigger_id = %id, "Deleted trigger");
        Ok(())
    }

    /// Process an event and trigger matching workflows
    pub async fn process_event(&self, event: TriggerEvent) -> Result<Vec<String>> {
        let triggers = self.repository.list_enabled().await?;
        let mut triggered_workflows = Vec::new();

        debug!(
            event_id = %event.id,
            event_type = %event.event_type,
            triggers_count = triggers.len(),
            "Processing trigger event"
        );

        for trigger in triggers {
            // Check tenant match
            if let Some(ref trigger_tenant) = trigger.tenant_id {
                if event.tenant_id.as_ref() != Some(trigger_tenant) {
                    continue;
                }
            }

            // Check condition
            if !trigger.condition.matches(&event) {
                continue;
            }

            // Check rate limit
            if let Some(ref rate_limit) = trigger.rate_limit {
                if !self
                    .check_rate_limit(&trigger.id, rate_limit)
                    .await
                {
                    warn!(
                        trigger_id = %trigger.id,
                        "Trigger rate limited"
                    );
                    continue;
                }
            }

            // Build input and execute workflow
            let _input = trigger.build_input(&event);

            // Get workflow definition
            let definition = match self.provider.get_workflow(&trigger.workflow_id).await {
                Ok(Some(def)) => def,
                Ok(None) => {
                    error!(
                        trigger_id = %trigger.id,
                        workflow_id = %trigger.workflow_id,
                        "Workflow definition not found"
                    );
                    continue;
                }
                Err(e) => {
                    error!(
                        trigger_id = %trigger.id,
                        error = %e,
                        "Failed to get workflow definition"
                    );
                    continue;
                }
            };

            match self.engine.execute_workflow(definition).await {
                Ok(run_id) => {
                    info!(
                        trigger_id = %trigger.id,
                        workflow_id = %trigger.workflow_id,
                        run_id = %run_id,
                        event_id = %event.id,
                        "Triggered workflow from event"
                    );
                    triggered_workflows.push(run_id);

                    // Record execution for rate limiting
                    if trigger.rate_limit.is_some() {
                        self.record_execution(&trigger.id).await;
                    }
                }
                Err(e) => {
                    error!(
                        trigger_id = %trigger.id,
                        error = %e,
                        "Failed to trigger workflow"
                    );
                }
            }
        }

        Ok(triggered_workflows)
    }

    /// Check if execution is within rate limit
    async fn check_rate_limit(&self, trigger_id: &str, rate_limit: &RateLimit) -> bool {
        let mut limiter = self.rate_limiter.write().await;
        let now = Utc::now();
        let window_start = now - chrono::Duration::seconds(rate_limit.window_seconds as i64);

        let state = limiter.entry(trigger_id.to_string()).or_insert(RateLimiterState {
            executions: Vec::new(),
        });

        // Remove old executions
        state.executions.retain(|t| *t > window_start);

        // Check limit
        state.executions.len() < rate_limit.max_executions as usize
    }

    /// Record an execution for rate limiting
    async fn record_execution(&self, trigger_id: &str) {
        let mut limiter = self.rate_limiter.write().await;
        let state = limiter.entry(trigger_id.to_string()).or_insert(RateLimiterState {
            executions: Vec::new(),
        });
        state.executions.push(Utc::now());
    }
}

/// Event bus for publishing events
pub struct EventBus {
    sender: mpsc::Sender<TriggerEvent>,
}

impl EventBus {
    pub fn new(buffer_size: usize) -> (Self, EventBusProcessor) {
        let (sender, receiver) = mpsc::channel(buffer_size);
        (
            Self { sender },
            EventBusProcessor { receiver },
        )
    }

    /// Publish an event
    pub async fn publish(&self, event: TriggerEvent) -> Result<()> {
        self.sender
            .send(event)
            .await
            .map_err(|_| WorkflowError::InvalidDefinition("Event bus closed".to_string()))
    }

    /// Publish multiple events
    pub async fn publish_batch(&self, events: Vec<TriggerEvent>) -> Result<()> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }
}

/// Event bus processor
pub struct EventBusProcessor {
    receiver: mpsc::Receiver<TriggerEvent>,
}

impl EventBusProcessor {
    /// Run the processor with a trigger manager
    pub async fn run(mut self, manager: Arc<TriggerManager>) {
        info!("Starting event bus processor");

        while let Some(event) = self.receiver.recv().await {
            debug!(
                event_id = %event.id,
                event_type = %event.event_type,
                "Processing event from bus"
            );

            if let Err(e) = manager.process_event(event).await {
                error!(error = %e, "Error processing event");
            }
        }

        info!("Event bus processor stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_condition() {
        let event = TriggerEvent::new(
            "user.created",
            EventSource::Api,
            serde_json::json!({"user_id": "123"}),
        );

        let condition = TriggerCondition::EventType {
            value: "user.created".to_string(),
        };
        assert!(condition.matches(&event));

        let condition = TriggerCondition::EventType {
            value: "user.deleted".to_string(),
        };
        assert!(!condition.matches(&event));
    }

    #[test]
    fn test_payload_field_condition() {
        let event = TriggerEvent::new(
            "order.created",
            EventSource::Api,
            serde_json::json!({
                "order": {
                    "status": "pending",
                    "items": [{"id": "item-1"}]
                }
            }),
        );

        let condition = TriggerCondition::PayloadField {
            path: "order.status".to_string(),
            value: serde_json::json!("pending"),
        };
        assert!(condition.matches(&event));

        let condition = TriggerCondition::PayloadFieldExists {
            path: "order.items".to_string(),
        };
        assert!(condition.matches(&event));
    }

    #[test]
    fn test_composite_conditions() {
        let event = TriggerEvent::new(
            "user.created",
            EventSource::Api,
            serde_json::json!({"user_id": "123"}),
        )
        .with_tenant("tenant-1");

        // All conditions
        let condition = TriggerCondition::All {
            conditions: vec![
                TriggerCondition::EventType {
                    value: "user.created".to_string(),
                },
                TriggerCondition::Tenant {
                    tenant_id: "tenant-1".to_string(),
                },
            ],
        };
        assert!(condition.matches(&event));

        // Any condition
        let condition = TriggerCondition::Any {
            conditions: vec![
                TriggerCondition::EventType {
                    value: "user.deleted".to_string(),
                },
                TriggerCondition::EventType {
                    value: "user.created".to_string(),
                },
            ],
        };
        assert!(condition.matches(&event));

        // Not condition
        let condition = TriggerCondition::Not {
            condition: Box::new(TriggerCondition::EventType {
                value: "user.deleted".to_string(),
            }),
        };
        assert!(condition.matches(&event));
    }

    #[test]
    fn test_input_mapping() {
        let trigger = WorkflowTrigger::new(
            "User Trigger",
            "wf-1",
            TriggerCondition::EventType {
                value: "user.created".to_string(),
            },
        )
        .with_input_mapping("user.id", "user_id")
        .with_input_mapping("user.email", "email");

        let event = TriggerEvent::new(
            "user.created",
            EventSource::Api,
            serde_json::json!({
                "user": {
                    "id": "user-123",
                    "email": "test@example.com"
                }
            }),
        );

        let input = trigger.build_input(&event);
        assert_eq!(input["user_id"], "user-123");
        assert_eq!(input["email"], "test@example.com");
        assert!(input["_event_id"].is_string());
    }

    #[tokio::test]
    async fn test_trigger_repository() {
        let repo = InMemoryTriggerRepository::new();

        let trigger = WorkflowTrigger::new(
            "Test Trigger",
            "wf-1",
            TriggerCondition::EventType {
                value: "test".to_string(),
            },
        );

        repo.save(&trigger).await.unwrap();

        let retrieved = repo.get(&trigger.id).await.unwrap();
        assert!(retrieved.is_some());

        let enabled = repo.list_enabled().await.unwrap();
        assert_eq!(enabled.len(), 1);

        repo.delete(&trigger.id).await.unwrap();
        assert!(repo.get(&trigger.id).await.unwrap().is_none());
    }
}
