//! Event system abstractions for pub/sub messaging.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a domain event in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier
    pub id: Uuid,
    /// Event type (e.g., "conversation.created", "workflow.completed")
    pub event_type: String,
    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,
    /// Event payload
    pub payload: serde_json::Value,
    /// Metadata for the event
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Event {
    /// Create a new event with the given type and payload.
    pub fn new<T: Serialize>(event_type: impl Into<String>, payload: T) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.into(),
            timestamp: Utc::now(),
            payload: serde_json::to_value(payload).unwrap_or(serde_json::Value::Null),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the event.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if the event matches a given type pattern.
    pub fn matches(&self, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            return self.event_type.starts_with(prefix);
        }
        self.event_type == pattern
    }
}

/// Trait for publishing events to a messaging system.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Error type returned by publisher operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Publish a single event.
    async fn publish(&self, event: &Event) -> Result<(), Self::Error>;

    /// Publish a batch of events.
    async fn publish_batch(&self, events: &[Event]) -> Result<(), Self::Error>;
}

/// Trait for subscribing to events from a messaging system.
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Error type returned by subscriber operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Subscribe to events matching the given pattern.
    async fn subscribe(&self, pattern: &str) -> Result<(), Self::Error>;

    /// Unsubscribe from a pattern.
    async fn unsubscribe(&self, pattern: &str) -> Result<(), Self::Error>;

    /// Receive the next event (blocking).
    async fn next(&mut self) -> Result<Option<Event>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new("test.event", serde_json::json!({"key": "value"}));

        assert!(!event.id.is_nil());
        assert_eq!(event.event_type, "test.event");
        assert!(event.payload.get("key").is_some());
    }

    #[test]
    fn test_event_with_metadata() {
        let event = Event::new("test.event", serde_json::json!({}))
            .with_metadata("source", "test")
            .with_metadata("version", "1.0");

        assert_eq!(event.metadata.get("source"), Some(&"test".to_string()));
        assert_eq!(event.metadata.get("version"), Some(&"1.0".to_string()));
    }

    #[test]
    fn test_event_matches() {
        let event = Event::new("conversation.created", serde_json::json!({}));

        assert!(event.matches("*"));
        assert!(event.matches("conversation.created"));
        assert!(event.matches("conversation.*"));
        assert!(!event.matches("workflow.*"));
        assert!(!event.matches("conversation.updated"));
    }
}
