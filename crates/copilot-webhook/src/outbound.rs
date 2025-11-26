//! Outbound webhook delivery
//!
//! Handles sending webhooks to external endpoints with retry support.

use crate::{
    delivery::{DeliveryAttempt, DeliveryStatus, WebhookDelivery},
    events::{WebhookEvent, WebhookEventType},
    signature::WebhookSigner,
    Result, WebhookError,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Webhook endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpoint {
    /// Endpoint ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Target URL
    pub url: String,
    /// Secret for signature generation
    pub secret: String,
    /// Events to subscribe to
    pub events: Vec<WebhookEventType>,
    /// Whether endpoint is active
    pub enabled: bool,
    /// Custom headers to include
    pub headers: Vec<(String, String)>,
    /// Tenant ID (if applicable)
    pub tenant_id: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
    /// API version
    pub api_version: String,
    /// Metadata
    pub metadata: serde_json::Value,
}

impl WebhookEndpoint {
    /// Create a new webhook endpoint
    pub fn new(name: &str, url: &str, secret: &str) -> Self {
        let now = Utc::now();
        Self {
            id: format!("we_{}", Uuid::new_v4().to_string().replace('-', "")),
            name: name.to_string(),
            url: url.to_string(),
            secret: secret.to_string(),
            events: Vec::new(),
            enabled: true,
            headers: Vec::new(),
            tenant_id: None,
            created_at: now,
            updated_at: now,
            api_version: "2024-01-01".to_string(),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_events(mut self, events: Vec<WebhookEventType>) -> Self {
        self.events = events;
        self
    }

    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = headers;
        self
    }

    /// Check if endpoint subscribes to an event type
    pub fn subscribes_to(&self, event_type: &WebhookEventType) -> bool {
        self.events.is_empty() || self.events.contains(event_type)
    }
}

/// Retry configuration for webhook delivery
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Jitter factor (0.0 - 1.0)
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::seconds(1),
            max_delay: Duration::hours(1),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a given attempt
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::zero();
        }

        let base_delay = self.initial_delay.num_milliseconds() as f64
            * self.backoff_multiplier.powi(attempt as i32 - 1);

        // Apply jitter
        let jitter = if self.jitter_factor > 0.0 {
            let range = base_delay * self.jitter_factor;
            (rand::random::<f64>() - 0.5) * 2.0 * range
        } else {
            0.0
        };

        let delay_ms = (base_delay + jitter).min(self.max_delay.num_milliseconds() as f64);
        Duration::milliseconds(delay_ms as i64)
    }
}

/// Webhook dispatcher for sending events to endpoints
pub struct WebhookDispatcher {
    client: Client,
    endpoints: Arc<DashMap<String, WebhookEndpoint>>,
    retry_config: RetryConfig,
    delivery_sender: mpsc::Sender<WebhookDelivery>,
}

impl WebhookDispatcher {
    /// Create a new dispatcher
    pub fn new(
        delivery_sender: mpsc::Sender<WebhookDelivery>,
        retry_config: RetryConfig,
    ) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            endpoints: Arc::new(DashMap::new()),
            retry_config,
            delivery_sender,
        }
    }

    /// Register a webhook endpoint
    pub fn register_endpoint(&self, endpoint: WebhookEndpoint) {
        info!(endpoint_id = %endpoint.id, url = %endpoint.url, "Registering webhook endpoint");
        self.endpoints.insert(endpoint.id.clone(), endpoint);
    }

    /// Unregister a webhook endpoint
    pub fn unregister_endpoint(&self, endpoint_id: &str) {
        info!(endpoint_id = %endpoint_id, "Unregistering webhook endpoint");
        self.endpoints.remove(endpoint_id);
    }

    /// Get an endpoint by ID
    pub fn get_endpoint(&self, endpoint_id: &str) -> Option<WebhookEndpoint> {
        self.endpoints.get(endpoint_id).map(|e| e.clone())
    }

    /// List all endpoints
    pub fn list_endpoints(&self) -> Vec<WebhookEndpoint> {
        self.endpoints.iter().map(|e| e.clone()).collect()
    }

    /// List endpoints for a tenant
    pub fn list_tenant_endpoints(&self, tenant_id: &str) -> Vec<WebhookEndpoint> {
        self.endpoints
            .iter()
            .filter(|e| e.tenant_id.as_deref() == Some(tenant_id))
            .map(|e| e.clone())
            .collect()
    }

    /// Dispatch an event to all subscribed endpoints
    pub async fn dispatch(&self, event: WebhookEvent) -> Result<Vec<WebhookDelivery>> {
        let mut deliveries = Vec::new();

        for endpoint_ref in self.endpoints.iter() {
            let endpoint = endpoint_ref.clone();

            // Skip disabled endpoints
            if !endpoint.enabled {
                continue;
            }

            // Check if endpoint subscribes to this event
            if !endpoint.subscribes_to(&event.event_type) {
                continue;
            }

            // Check tenant filter
            if let Some(ref tenant_id) = endpoint.tenant_id {
                if event.tenant_id.as_ref() != Some(tenant_id) {
                    continue;
                }
            }

            let delivery = self.deliver_to_endpoint(&endpoint, &event).await;
            deliveries.push(delivery);
        }

        Ok(deliveries)
    }

    /// Deliver event to a specific endpoint
    async fn deliver_to_endpoint(
        &self,
        endpoint: &WebhookEndpoint,
        event: &WebhookEvent,
    ) -> WebhookDelivery {
        let delivery_id = format!("whd_{}", Uuid::new_v4().to_string().replace('-', ""));
        let payload = serde_json::to_vec(event).unwrap_or_default();

        let mut delivery = WebhookDelivery {
            id: delivery_id.clone(),
            endpoint_id: endpoint.id.clone(),
            event_id: event.id.clone(),
            event_type: event.event_type,
            status: DeliveryStatus::Pending,
            attempts: Vec::new(),
            payload_size: payload.len(),
            created_at: Utc::now(),
            completed_at: None,
            next_retry_at: None,
        };

        // Attempt delivery with retries
        for attempt_num in 0..=self.retry_config.max_attempts {
            if attempt_num > 0 {
                let delay = self.retry_config.calculate_delay(attempt_num);
                debug!(
                    delivery_id = %delivery_id,
                    attempt = attempt_num,
                    delay_ms = delay.num_milliseconds(),
                    "Waiting before retry"
                );
                tokio::time::sleep(delay.to_std().unwrap_or_default()).await;
            }

            let attempt = self
                .attempt_delivery(endpoint, &payload, attempt_num)
                .await;

            let success = matches!(attempt.status, DeliveryStatus::Delivered);
            delivery.attempts.push(attempt);

            if success {
                delivery.status = DeliveryStatus::Delivered;
                delivery.completed_at = Some(Utc::now());

                info!(
                    delivery_id = %delivery_id,
                    endpoint_id = %endpoint.id,
                    attempts = delivery.attempts.len(),
                    "Webhook delivered successfully"
                );
                break;
            }

            if attempt_num == self.retry_config.max_attempts {
                delivery.status = DeliveryStatus::Failed;
                delivery.completed_at = Some(Utc::now());

                error!(
                    delivery_id = %delivery_id,
                    endpoint_id = %endpoint.id,
                    attempts = delivery.attempts.len(),
                    "Webhook delivery failed after all retries"
                );
            }
        }

        // Send delivery record to tracking channel
        let _ = self.delivery_sender.send(delivery.clone()).await;

        delivery
    }

    /// Attempt a single delivery
    async fn attempt_delivery(
        &self,
        endpoint: &WebhookEndpoint,
        payload: &[u8],
        attempt_num: u32,
    ) -> DeliveryAttempt {
        let started_at = Utc::now();
        let signer = WebhookSigner::new(&endpoint.secret);
        let signature_headers = signer.get_headers(payload);

        let mut request = self
            .client
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "CoPilot-Webhook/1.0");

        // Add signature headers
        for (key, value) in &signature_headers {
            request = request.header(key.as_str(), value.as_str());
        }

        // Add custom headers
        for (key, value) in &endpoint.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        let result = request.body(payload.to_vec()).send().await;

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        match result {
            Ok(response) => {
                let status_code = response.status();
                let response_body = response.text().await.ok();

                let delivery_status = if status_code.is_success() {
                    DeliveryStatus::Delivered
                } else if status_code.is_server_error() || status_code == StatusCode::TOO_MANY_REQUESTS {
                    DeliveryStatus::Retrying
                } else {
                    DeliveryStatus::Failed
                };

                if !status_code.is_success() {
                    warn!(
                        url = %endpoint.url,
                        status = %status_code,
                        attempt = attempt_num,
                        "Webhook delivery received non-success response"
                    );
                }

                DeliveryAttempt {
                    attempt_number: attempt_num,
                    status: delivery_status,
                    status_code: Some(status_code.as_u16()),
                    response_body,
                    error_message: None,
                    started_at,
                    completed_at,
                    duration_ms,
                }
            }
            Err(e) => {
                warn!(
                    url = %endpoint.url,
                    error = %e,
                    attempt = attempt_num,
                    "Webhook delivery failed"
                );

                DeliveryAttempt {
                    attempt_number: attempt_num,
                    status: DeliveryStatus::Retrying,
                    status_code: None,
                    response_body: None,
                    error_message: Some(e.to_string()),
                    started_at,
                    completed_at,
                    duration_ms,
                }
            }
        }
    }
}

/// Webhook endpoint repository trait
#[async_trait]
pub trait WebhookEndpointRepository: Send + Sync {
    /// Store an endpoint
    async fn save(&self, endpoint: &WebhookEndpoint) -> Result<()>;

    /// Get endpoint by ID
    async fn get(&self, id: &str) -> Result<Option<WebhookEndpoint>>;

    /// List all endpoints
    async fn list(&self) -> Result<Vec<WebhookEndpoint>>;

    /// List endpoints for a tenant
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<WebhookEndpoint>>;

    /// Delete an endpoint
    async fn delete(&self, id: &str) -> Result<()>;

    /// Update an endpoint
    async fn update(&self, endpoint: &WebhookEndpoint) -> Result<()>;
}

/// In-memory webhook endpoint repository
pub struct InMemoryEndpointRepository {
    endpoints: DashMap<String, WebhookEndpoint>,
}

impl InMemoryEndpointRepository {
    pub fn new() -> Self {
        Self {
            endpoints: DashMap::new(),
        }
    }
}

impl Default for InMemoryEndpointRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookEndpointRepository for InMemoryEndpointRepository {
    async fn save(&self, endpoint: &WebhookEndpoint) -> Result<()> {
        self.endpoints.insert(endpoint.id.clone(), endpoint.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<WebhookEndpoint>> {
        Ok(self.endpoints.get(id).map(|e| e.clone()))
    }

    async fn list(&self) -> Result<Vec<WebhookEndpoint>> {
        Ok(self.endpoints.iter().map(|e| e.clone()).collect())
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<WebhookEndpoint>> {
        Ok(self
            .endpoints
            .iter()
            .filter(|e| e.tenant_id.as_deref() == Some(tenant_id))
            .map(|e| e.clone())
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        self.endpoints.remove(id);
        Ok(())
    }

    async fn update(&self, endpoint: &WebhookEndpoint) -> Result<()> {
        if self.endpoints.contains_key(&endpoint.id) {
            self.endpoints.insert(endpoint.id.clone(), endpoint.clone());
            Ok(())
        } else {
            Err(WebhookError::NotFound(endpoint.id.clone()))
        }
    }
}

/// Webhook event queue for async processing
pub struct WebhookEventQueue {
    sender: mpsc::Sender<(WebhookEvent, Option<String>)>,
}

impl WebhookEventQueue {
    /// Create a new event queue with a processor
    pub fn new(
        dispatcher: Arc<WebhookDispatcher>,
        buffer_size: usize,
    ) -> (Self, WebhookEventProcessor) {
        let (sender, receiver) = mpsc::channel(buffer_size);

        let processor = WebhookEventProcessor {
            dispatcher,
            receiver,
        };

        (Self { sender }, processor)
    }

    /// Queue an event for delivery
    pub async fn queue(&self, event: WebhookEvent) -> Result<()> {
        self.sender
            .send((event, None))
            .await
            .map_err(|_| WebhookError::DeliveryFailed("Queue full".to_string()))
    }

    /// Queue an event for a specific endpoint
    pub async fn queue_for_endpoint(&self, event: WebhookEvent, endpoint_id: &str) -> Result<()> {
        self.sender
            .send((event, Some(endpoint_id.to_string())))
            .await
            .map_err(|_| WebhookError::DeliveryFailed("Queue full".to_string()))
    }
}

/// Processes queued webhook events
pub struct WebhookEventProcessor {
    dispatcher: Arc<WebhookDispatcher>,
    receiver: mpsc::Receiver<(WebhookEvent, Option<String>)>,
}

impl WebhookEventProcessor {
    /// Run the processor (blocking)
    pub async fn run(mut self) {
        info!("Starting webhook event processor");

        while let Some((event, endpoint_id)) = self.receiver.recv().await {
            if let Some(endpoint_id) = endpoint_id {
                // Deliver to specific endpoint
                if let Some(endpoint) = self.dispatcher.get_endpoint(&endpoint_id) {
                    let _ = self.dispatcher.deliver_to_endpoint(&endpoint, &event).await;
                }
            } else {
                // Broadcast to all subscribed endpoints
                let _ = self.dispatcher.dispatch(event).await;
            }
        }

        info!("Webhook event processor stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_creation() {
        let endpoint = WebhookEndpoint::new("Test", "https://example.com/webhook", "secret123")
            .with_events(vec![WebhookEventType::ConversationCreated])
            .with_tenant("tenant-1");

        assert!(endpoint.id.starts_with("we_"));
        assert_eq!(endpoint.name, "Test");
        assert!(endpoint.enabled);
        assert_eq!(endpoint.tenant_id, Some("tenant-1".to_string()));
    }

    #[test]
    fn test_endpoint_subscription() {
        let endpoint = WebhookEndpoint::new("Test", "https://example.com", "secret")
            .with_events(vec![
                WebhookEventType::ConversationCreated,
                WebhookEventType::MessageCreated,
            ]);

        assert!(endpoint.subscribes_to(&WebhookEventType::ConversationCreated));
        assert!(endpoint.subscribes_to(&WebhookEventType::MessageCreated));
        assert!(!endpoint.subscribes_to(&WebhookEventType::UserCreated));
    }

    #[test]
    fn test_endpoint_all_events() {
        let endpoint = WebhookEndpoint::new("Test", "https://example.com", "secret");

        // Empty events list means subscribe to all
        assert!(endpoint.subscribes_to(&WebhookEventType::ConversationCreated));
        assert!(endpoint.subscribes_to(&WebhookEventType::UserCreated));
    }

    #[test]
    fn test_retry_config_delay() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_delay: Duration::seconds(1),
            max_delay: Duration::hours(1),
            backoff_multiplier: 2.0,
            jitter_factor: 0.0, // No jitter for testing
        };

        assert_eq!(config.calculate_delay(0), Duration::zero());
        assert_eq!(config.calculate_delay(1), Duration::seconds(1));
        assert_eq!(config.calculate_delay(2), Duration::seconds(2));
        assert_eq!(config.calculate_delay(3), Duration::seconds(4));
    }

    #[tokio::test]
    async fn test_in_memory_repository() {
        let repo = InMemoryEndpointRepository::new();

        let endpoint = WebhookEndpoint::new("Test", "https://example.com", "secret")
            .with_tenant("tenant-1");

        repo.save(&endpoint).await.unwrap();

        let retrieved = repo.get(&endpoint.id).await.unwrap();
        assert!(retrieved.is_some());

        let tenant_endpoints = repo.list_by_tenant("tenant-1").await.unwrap();
        assert_eq!(tenant_endpoints.len(), 1);

        repo.delete(&endpoint.id).await.unwrap();
        let deleted = repo.get(&endpoint.id).await.unwrap();
        assert!(deleted.is_none());
    }
}
