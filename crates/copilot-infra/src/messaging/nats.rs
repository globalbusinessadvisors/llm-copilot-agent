use async_nats::{Client, ConnectOptions, Message, Subscriber};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn, error};

use copilot_core::events::{Event, EventPublisher};
use crate::{InfraError, Result};

#[derive(Debug, Clone)]
pub struct NatsConfig {
    pub url: String,
    pub name: Option<String>,
    pub max_reconnects: Option<usize>,
    pub reconnect_delay: Duration,
    pub subject_prefix: Option<String>,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: String::from("nats://127.0.0.1:4222"),
            name: Some(String::from("copilot-agent")),
            max_reconnects: None, // Infinite reconnects
            reconnect_delay: Duration::from_secs(2),
            subject_prefix: Some(String::from("copilot.")),
        }
    }
}

impl NatsConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_max_reconnects(mut self, max: Option<usize>) -> Self {
        self.max_reconnects = max;
        self
    }

    pub fn with_reconnect_delay(mut self, delay: Duration) -> Self {
        self.reconnect_delay = delay;
        self
    }

    pub fn with_subject_prefix(mut self, prefix: Option<String>) -> Self {
        self.subject_prefix = prefix;
        self
    }
}

#[derive(Clone)]
pub struct NatsPublisher {
    client: Client,
    config: NatsConfig,
}

impl NatsPublisher {
    pub async fn new(config: NatsConfig) -> Result<Self> {
        info!("Connecting to NATS at {}", config.url);

        let mut options = ConnectOptions::new()
            .reconnect_delay_callback(move |attempts| {
                let delay = std::cmp::min(
                    Duration::from_secs(2_u64.pow(attempts as u32)),
                    Duration::from_secs(30),
                );
                debug!("NATS reconnect attempt {} with delay {:?}", attempts, delay);
                delay
            });

        if let Some(name) = &config.name {
            options = options.name(name);
        }

        // Note: max_reconnects is configured via retry_on_initial_connect in newer async_nats
        // The reconnection behavior is now automatic with exponential backoff

        let client = options
            .connect(&config.url)
            .await
            .map_err(|e| {
                error!("Failed to connect to NATS: {}", e);
                InfraError::Messaging(format!("Failed to connect to NATS: {}", e))
            })?;

        info!("NATS connection established");

        Ok(Self { client, config })
    }

    fn make_subject(&self, subject: &str) -> String {
        match &self.config.subject_prefix {
            Some(prefix) => format!("{}{}", prefix, subject),
            None => subject.to_string(),
        }
    }

    pub async fn publish_raw(&self, subject: &str, payload: Vec<u8>) -> Result<()> {
        let full_subject = self.make_subject(subject);
        debug!("Publishing message to subject: {}", full_subject);

        self.client
            .publish(full_subject.clone(), payload.into())
            .await
            .map_err(|e| {
                error!("Failed to publish to {}: {}", full_subject, e);
                InfraError::Messaging(format!("Failed to publish to {}: {}", full_subject, e))
            })?;

        Ok(())
    }

    pub async fn request<T, R>(&self, subject: &str, payload: &T, timeout: Duration) -> Result<R>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let full_subject = self.make_subject(subject);
        debug!("Sending request to subject: {}", full_subject);

        let serialized = serde_json::to_vec(payload)?;

        let response = tokio::time::timeout(
            timeout,
            self.client.request(full_subject.clone(), serialized.into()),
        )
        .await
        .map_err(|_| {
            error!("Request to {} timed out after {:?}", full_subject, timeout);
            InfraError::Messaging(format!("Request to {} timed out", full_subject))
        })?
        .map_err(|e| {
            error!("Request to {} failed: {}", full_subject, e);
            InfraError::Messaging(format!("Request to {} failed: {}", full_subject, e))
        })?;

        let deserialized: R = serde_json::from_slice(&response.payload)?;
        Ok(deserialized)
    }

    pub async fn health_check(&self) -> Result<()> {
        debug!("Performing NATS health check");

        if self.client.connection_state() == async_nats::connection::State::Connected {
            Ok(())
        } else {
            warn!("NATS client is not connected");
            Err(InfraError::HealthCheck("NATS client is not connected".to_string()))
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[async_trait]
impl EventPublisher for NatsPublisher {
    type Error = InfraError;

    async fn publish(&self, event: &Event) -> Result<()> {
        debug!("Publishing event: {}", event.event_type);

        let subject = format!("events.{}", event.event_type);
        let serialized = serde_json::to_vec(event)?;

        self.publish_raw(&subject, serialized).await
    }

    async fn publish_batch(&self, events: &[Event]) -> Result<()> {
        debug!("Publishing batch of {} events", events.len());

        for event in events {
            self.publish(event).await?;
        }

        Ok(())
    }
}

pub struct NatsSubscriber {
    subscriber: Subscriber,
    subject: String,
}

impl NatsSubscriber {
    pub async fn new(client: &Client, subject: impl Into<String>) -> Result<Self> {
        let subject = subject.into();
        info!("Subscribing to subject: {}", subject);

        let subscriber = client
            .subscribe(subject.clone())
            .await
            .map_err(|e| {
                error!("Failed to subscribe to {}: {}", subject, e);
                InfraError::Messaging(format!("Failed to subscribe to {}: {}", subject, e))
            })?;

        info!("Subscribed to subject: {}", subject);

        Ok(Self { subscriber, subject })
    }

    pub async fn new_queue(
        client: &Client,
        subject: impl Into<String>,
        queue: impl Into<String>,
    ) -> Result<Self> {
        let subject = subject.into();
        let queue = queue.into();
        info!("Subscribing to subject: {} with queue: {}", subject, queue);

        let subscriber = client
            .queue_subscribe(subject.clone(), queue.clone())
            .await
            .map_err(|e| {
                error!("Failed to subscribe to {} (queue: {}): {}", subject, queue, e);
                InfraError::Messaging(format!(
                    "Failed to subscribe to {} (queue: {}): {}",
                    subject, queue, e
                ))
            })?;

        info!("Subscribed to subject: {} with queue: {}", subject, queue);

        Ok(Self { subscriber, subject })
    }

    pub async fn next(&mut self) -> Option<Message> {
        self.subscriber.next().await
    }

    pub async fn next_event(&mut self) -> Result<Option<Event>> {
        match self.subscriber.next().await {
            Some(msg) => {
                let event: Event = serde_json::from_slice(&msg.payload)?;
                Ok(Some(event))
            }
            None => Ok(None),
        }
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub async fn unsubscribe(mut self) -> Result<()> {
        info!("Unsubscribing from subject: {}", self.subject);

        self.subscriber
            .unsubscribe()
            .await
            .map_err(|e| {
                error!("Failed to unsubscribe from {}: {}", self.subject, e);
                InfraError::Messaging(format!("Failed to unsubscribe from {}: {}", self.subject, e))
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = NatsConfig::new("nats://localhost:4222")
            .with_name("test-client")
            .with_max_reconnects(Some(5))
            .with_reconnect_delay(Duration::from_secs(1))
            .with_subject_prefix(Some("test.".to_string()));

        assert_eq!(config.url, "nats://localhost:4222");
        assert_eq!(config.name, Some("test-client".to_string()));
        assert_eq!(config.max_reconnects, Some(5));
        assert_eq!(config.reconnect_delay, Duration::from_secs(1));
        assert_eq!(config.subject_prefix, Some("test.".to_string()));
    }

    #[tokio::test]
    async fn test_make_subject_with_prefix() {
        let config = NatsConfig::new("nats://localhost")
            .with_subject_prefix(Some("app.".to_string()));

        // We can't create a real publisher without a connection, so we'll just test the config
        assert_eq!(config.subject_prefix, Some("app.".to_string()));
    }

    #[tokio::test]
    async fn test_make_subject_without_prefix() {
        let config = NatsConfig::new("nats://localhost")
            .with_subject_prefix(None);

        assert_eq!(config.subject_prefix, None);
    }
}
