//! Webhook delivery tracking
//!
//! Provides delivery status tracking, retry management, and delivery history.

use crate::{events::WebhookEventType, Result, WebhookError};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tracing::info;

/// Delivery status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    /// Pending delivery
    Pending,
    /// Currently being delivered
    InProgress,
    /// Delivered successfully
    Delivered,
    /// Retrying after failure
    Retrying,
    /// Failed permanently
    Failed,
    /// Skipped (e.g., endpoint disabled)
    Skipped,
}

/// Single delivery attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryAttempt {
    /// Attempt number (0-indexed)
    pub attempt_number: u32,
    /// Result status
    pub status: DeliveryStatus,
    /// HTTP status code (if any)
    pub status_code: Option<u16>,
    /// Response body (truncated)
    pub response_body: Option<String>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Attempt started at
    pub started_at: DateTime<Utc>,
    /// Attempt completed at
    pub completed_at: DateTime<Utc>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Webhook delivery record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    /// Delivery ID
    pub id: String,
    /// Endpoint ID
    pub endpoint_id: String,
    /// Event ID
    pub event_id: String,
    /// Event type
    pub event_type: WebhookEventType,
    /// Current status
    pub status: DeliveryStatus,
    /// Delivery attempts
    pub attempts: Vec<DeliveryAttempt>,
    /// Payload size in bytes
    pub payload_size: usize,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Completed at (success or final failure)
    pub completed_at: Option<DateTime<Utc>>,
    /// Next retry scheduled
    pub next_retry_at: Option<DateTime<Utc>>,
}

impl WebhookDelivery {
    /// Get total duration across all attempts
    pub fn total_duration_ms(&self) -> u64 {
        self.attempts.iter().map(|a| a.duration_ms).sum()
    }

    /// Get number of attempts
    pub fn attempt_count(&self) -> usize {
        self.attempts.len()
    }

    /// Check if delivery succeeded
    pub fn is_success(&self) -> bool {
        self.status == DeliveryStatus::Delivered
    }

    /// Get last attempt
    pub fn last_attempt(&self) -> Option<&DeliveryAttempt> {
        self.attempts.last()
    }
}

/// Delivery repository trait
#[async_trait]
pub trait DeliveryRepository: Send + Sync {
    /// Save delivery record
    async fn save(&self, delivery: &WebhookDelivery) -> Result<()>;

    /// Get delivery by ID
    async fn get(&self, id: &str) -> Result<Option<WebhookDelivery>>;

    /// List deliveries for an endpoint
    async fn list_by_endpoint(
        &self,
        endpoint_id: &str,
        limit: usize,
    ) -> Result<Vec<WebhookDelivery>>;

    /// List deliveries for an event
    async fn list_by_event(&self, event_id: &str) -> Result<Vec<WebhookDelivery>>;

    /// List failed deliveries pending retry
    async fn list_pending_retries(&self, limit: usize) -> Result<Vec<WebhookDelivery>>;

    /// Update delivery status
    async fn update_status(&self, id: &str, status: DeliveryStatus) -> Result<()>;

    /// Delete old deliveries
    async fn cleanup(&self, older_than: DateTime<Utc>) -> Result<u64>;
}

/// In-memory delivery repository with size limits
pub struct InMemoryDeliveryRepository {
    deliveries: DashMap<String, WebhookDelivery>,
    by_endpoint: DashMap<String, VecDeque<String>>,
    by_event: DashMap<String, Vec<String>>,
    max_per_endpoint: usize,
    max_total: usize,
    total_count: RwLock<usize>,
}

impl InMemoryDeliveryRepository {
    pub fn new(max_per_endpoint: usize, max_total: usize) -> Self {
        Self {
            deliveries: DashMap::new(),
            by_endpoint: DashMap::new(),
            by_event: DashMap::new(),
            max_per_endpoint,
            max_total,
            total_count: RwLock::new(0),
        }
    }

    fn prune_endpoint(&self, endpoint_id: &str) {
        if let Some(mut ids) = self.by_endpoint.get_mut(endpoint_id) {
            while ids.len() > self.max_per_endpoint {
                if let Some(old_id) = ids.pop_front() {
                    self.deliveries.remove(&old_id);
                    let mut count = self.total_count.write();
                    *count = count.saturating_sub(1);
                }
            }
        }
    }

    fn prune_global(&self) {
        let count = *self.total_count.read();
        if count <= self.max_total {
            return;
        }

        // Simple pruning: remove oldest entries
        let to_remove = count - self.max_total;
        let mut removed = 0;

        let delivery_ids: Vec<_> = self.deliveries.iter().map(|d| d.id.clone()).collect();

        for id in delivery_ids {
            if removed >= to_remove {
                break;
            }
            if let Some((_, delivery)) = self.deliveries.remove(&id) {
                // Remove from indexes
                if let Some(mut ids) = self.by_endpoint.get_mut(&delivery.endpoint_id) {
                    ids.retain(|i| i != &id);
                }
                if let Some(mut ids) = self.by_event.get_mut(&delivery.event_id) {
                    ids.retain(|i| i != &id);
                }
                removed += 1;
            }
        }

        let mut count = self.total_count.write();
        *count = count.saturating_sub(removed);
    }
}

impl Default for InMemoryDeliveryRepository {
    fn default() -> Self {
        Self::new(1000, 100_000)
    }
}

#[async_trait]
impl DeliveryRepository for InMemoryDeliveryRepository {
    async fn save(&self, delivery: &WebhookDelivery) -> Result<()> {
        let id = delivery.id.clone();
        let endpoint_id = delivery.endpoint_id.clone();
        let event_id = delivery.event_id.clone();

        let is_new = !self.deliveries.contains_key(&id);

        self.deliveries.insert(id.clone(), delivery.clone());

        if is_new {
            // Update indexes
            self.by_endpoint
                .entry(endpoint_id.clone())
                .or_insert_with(VecDeque::new)
                .push_back(id.clone());

            self.by_event
                .entry(event_id)
                .or_insert_with(Vec::new)
                .push(id);

            {
                let mut count = self.total_count.write();
                *count += 1;
            }

            // Prune if needed
            self.prune_endpoint(&endpoint_id);
            self.prune_global();
        }

        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<WebhookDelivery>> {
        Ok(self.deliveries.get(id).map(|d| d.clone()))
    }

    async fn list_by_endpoint(
        &self,
        endpoint_id: &str,
        limit: usize,
    ) -> Result<Vec<WebhookDelivery>> {
        let ids = self
            .by_endpoint
            .get(endpoint_id)
            .map(|ids| ids.iter().rev().take(limit).cloned().collect::<Vec<_>>())
            .unwrap_or_default();

        let deliveries = ids
            .iter()
            .filter_map(|id| self.deliveries.get(id).map(|d| d.clone()))
            .collect();

        Ok(deliveries)
    }

    async fn list_by_event(&self, event_id: &str) -> Result<Vec<WebhookDelivery>> {
        let ids = self
            .by_event
            .get(event_id)
            .map(|ids| ids.clone())
            .unwrap_or_default();

        let deliveries = ids
            .iter()
            .filter_map(|id| self.deliveries.get(id).map(|d| d.clone()))
            .collect();

        Ok(deliveries)
    }

    async fn list_pending_retries(&self, limit: usize) -> Result<Vec<WebhookDelivery>> {
        let now = Utc::now();

        let deliveries: Vec<_> = self
            .deliveries
            .iter()
            .filter(|d| {
                d.status == DeliveryStatus::Retrying
                    && d.next_retry_at.map(|t| t <= now).unwrap_or(true)
            })
            .take(limit)
            .map(|d| d.clone())
            .collect();

        Ok(deliveries)
    }

    async fn update_status(&self, id: &str, status: DeliveryStatus) -> Result<()> {
        if let Some(mut delivery) = self.deliveries.get_mut(id) {
            delivery.status = status;
            if matches!(status, DeliveryStatus::Delivered | DeliveryStatus::Failed) {
                delivery.completed_at = Some(Utc::now());
            }
            Ok(())
        } else {
            Err(WebhookError::NotFound(id.to_string()))
        }
    }

    async fn cleanup(&self, older_than: DateTime<Utc>) -> Result<u64> {
        let mut removed = 0u64;

        let to_remove: Vec<_> = self
            .deliveries
            .iter()
            .filter(|d| d.created_at < older_than)
            .map(|d| d.id.clone())
            .collect();

        for id in to_remove {
            if let Some((_, delivery)) = self.deliveries.remove(&id) {
                if let Some(mut ids) = self.by_endpoint.get_mut(&delivery.endpoint_id) {
                    ids.retain(|i| i != &id);
                }
                if let Some(mut ids) = self.by_event.get_mut(&delivery.event_id) {
                    ids.retain(|i| i != &id);
                }
                removed += 1;
            }
        }

        {
            let mut count = self.total_count.write();
            *count = count.saturating_sub(removed as usize);
        }

        info!(removed = removed, "Cleaned up old delivery records");

        Ok(removed)
    }
}

/// Delivery statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeliveryStats {
    /// Total deliveries
    pub total: u64,
    /// Successful deliveries
    pub delivered: u64,
    /// Failed deliveries
    pub failed: u64,
    /// Pending deliveries
    pub pending: u64,
    /// Average latency in ms
    pub avg_latency_ms: f64,
    /// P95 latency in ms
    pub p95_latency_ms: f64,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    /// Average attempts per delivery
    pub avg_attempts: f64,
}

/// Delivery tracker for statistics
pub struct DeliveryTracker {
    repository: Arc<dyn DeliveryRepository>,
    stats: RwLock<DeliveryStats>,
    latencies: RwLock<VecDeque<u64>>,
    max_latency_samples: usize,
}

impl DeliveryTracker {
    pub fn new(repository: Arc<dyn DeliveryRepository>) -> Self {
        Self {
            repository,
            stats: RwLock::new(DeliveryStats::default()),
            latencies: RwLock::new(VecDeque::new()),
            max_latency_samples: 10000,
        }
    }

    /// Record a delivery
    pub async fn record(&self, delivery: &WebhookDelivery) -> Result<()> {
        // Save to repository
        self.repository.save(delivery).await?;

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.total += 1;

            match delivery.status {
                DeliveryStatus::Delivered => stats.delivered += 1,
                DeliveryStatus::Failed => stats.failed += 1,
                DeliveryStatus::Pending | DeliveryStatus::InProgress | DeliveryStatus::Retrying => {
                    stats.pending += 1
                }
                DeliveryStatus::Skipped => {}
            }

            // Update success rate
            let completed = stats.delivered + stats.failed;
            if completed > 0 {
                stats.success_rate = stats.delivered as f64 / completed as f64;
            }
        }

        // Record latency
        if delivery.status == DeliveryStatus::Delivered || delivery.status == DeliveryStatus::Failed
        {
            let latency = delivery.total_duration_ms();
            let mut latencies = self.latencies.write();
            latencies.push_back(latency);
            if latencies.len() > self.max_latency_samples {
                latencies.pop_front();
            }

            // Recalculate latency stats
            let mut stats = self.stats.write();
            if !latencies.is_empty() {
                stats.avg_latency_ms =
                    latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;

                let mut sorted: Vec<_> = latencies.iter().copied().collect();
                sorted.sort();
                let p95_idx = (sorted.len() as f64 * 0.95) as usize;
                stats.p95_latency_ms = sorted.get(p95_idx).copied().unwrap_or(0) as f64;
            }
        }

        Ok(())
    }

    /// Get current stats
    pub fn stats(&self) -> DeliveryStats {
        self.stats.read().clone()
    }

    /// Get delivery by ID
    pub async fn get(&self, id: &str) -> Result<Option<WebhookDelivery>> {
        self.repository.get(id).await
    }

    /// List deliveries for endpoint
    pub async fn list_by_endpoint(
        &self,
        endpoint_id: &str,
        limit: usize,
    ) -> Result<Vec<WebhookDelivery>> {
        self.repository.list_by_endpoint(endpoint_id, limit).await
    }

    /// List deliveries for event
    pub async fn list_by_event(&self, event_id: &str) -> Result<Vec<WebhookDelivery>> {
        self.repository.list_by_event(event_id).await
    }

    /// Cleanup old records
    pub async fn cleanup(&self, retention_days: i64) -> Result<u64> {
        let cutoff = Utc::now() - Duration::days(retention_days);
        self.repository.cleanup(cutoff).await
    }
}

/// Delivery report for an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointDeliveryReport {
    /// Endpoint ID
    pub endpoint_id: String,
    /// Time range start
    pub from: DateTime<Utc>,
    /// Time range end
    pub to: DateTime<Utc>,
    /// Total deliveries in range
    pub total: u64,
    /// Successful deliveries
    pub successful: u64,
    /// Failed deliveries
    pub failed: u64,
    /// Success rate
    pub success_rate: f64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Event type breakdown
    pub by_event_type: Vec<EventTypeStats>,
}

/// Stats per event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTypeStats {
    pub event_type: WebhookEventType,
    pub count: u64,
    pub success_rate: f64,
}

/// Report generator
pub struct DeliveryReportGenerator {
    repository: Arc<dyn DeliveryRepository>,
}

impl DeliveryReportGenerator {
    pub fn new(repository: Arc<dyn DeliveryRepository>) -> Self {
        Self { repository }
    }

    /// Generate report for an endpoint
    pub async fn generate_endpoint_report(
        &self,
        endpoint_id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<EndpointDeliveryReport> {
        let deliveries = self
            .repository
            .list_by_endpoint(endpoint_id, 10000)
            .await?;

        let filtered: Vec<_> = deliveries
            .into_iter()
            .filter(|d| d.created_at >= from && d.created_at <= to)
            .collect();

        let total = filtered.len() as u64;
        let successful = filtered
            .iter()
            .filter(|d| d.status == DeliveryStatus::Delivered)
            .count() as u64;
        let failed = filtered
            .iter()
            .filter(|d| d.status == DeliveryStatus::Failed)
            .count() as u64;

        let success_rate = if total > 0 {
            successful as f64 / total as f64
        } else {
            0.0
        };

        let total_latency: u64 = filtered.iter().map(|d| d.total_duration_ms()).sum();
        let avg_latency_ms = if total > 0 {
            total_latency as f64 / total as f64
        } else {
            0.0
        };

        // Group by event type
        let mut event_stats: std::collections::HashMap<WebhookEventType, (u64, u64)> =
            std::collections::HashMap::new();

        for delivery in &filtered {
            let entry = event_stats.entry(delivery.event_type).or_insert((0, 0));
            entry.0 += 1;
            if delivery.status == DeliveryStatus::Delivered {
                entry.1 += 1;
            }
        }

        let by_event_type: Vec<_> = event_stats
            .into_iter()
            .map(|(event_type, (count, success))| EventTypeStats {
                event_type,
                count,
                success_rate: if count > 0 {
                    success as f64 / count as f64
                } else {
                    0.0
                },
            })
            .collect();

        Ok(EndpointDeliveryReport {
            endpoint_id: endpoint_id.to_string(),
            from,
            to,
            total,
            successful,
            failed,
            success_rate,
            avg_latency_ms,
            by_event_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::WebhookEventType;

    fn create_test_delivery(id: &str, endpoint_id: &str, status: DeliveryStatus) -> WebhookDelivery {
        WebhookDelivery {
            id: id.to_string(),
            endpoint_id: endpoint_id.to_string(),
            event_id: "evt_123".to_string(),
            event_type: WebhookEventType::ConversationCreated,
            status,
            attempts: vec![DeliveryAttempt {
                attempt_number: 0,
                status,
                status_code: Some(200),
                response_body: None,
                error_message: None,
                started_at: Utc::now(),
                completed_at: Utc::now(),
                duration_ms: 100,
            }],
            payload_size: 256,
            created_at: Utc::now(),
            completed_at: Some(Utc::now()),
            next_retry_at: None,
        }
    }

    #[tokio::test]
    async fn test_delivery_repository() {
        let repo = InMemoryDeliveryRepository::default();

        let delivery = create_test_delivery("d1", "ep1", DeliveryStatus::Delivered);
        repo.save(&delivery).await.unwrap();

        let retrieved = repo.get("d1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "d1");

        let by_endpoint = repo.list_by_endpoint("ep1", 10).await.unwrap();
        assert_eq!(by_endpoint.len(), 1);
    }

    #[tokio::test]
    async fn test_delivery_tracker() {
        let repo = Arc::new(InMemoryDeliveryRepository::default());
        let tracker = DeliveryTracker::new(repo);

        // Record successful delivery
        let delivery1 = create_test_delivery("d1", "ep1", DeliveryStatus::Delivered);
        tracker.record(&delivery1).await.unwrap();

        // Record failed delivery
        let delivery2 = create_test_delivery("d2", "ep1", DeliveryStatus::Failed);
        tracker.record(&delivery2).await.unwrap();

        let stats = tracker.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.delivered, 1);
        assert_eq!(stats.failed, 1);
        assert!((stats.success_rate - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_delivery_metrics() {
        let mut delivery = create_test_delivery("d1", "ep1", DeliveryStatus::Delivered);
        delivery.attempts.push(DeliveryAttempt {
            attempt_number: 1,
            status: DeliveryStatus::Delivered,
            status_code: Some(200),
            response_body: None,
            error_message: None,
            started_at: Utc::now(),
            completed_at: Utc::now(),
            duration_ms: 50,
        });

        assert_eq!(delivery.attempt_count(), 2);
        assert_eq!(delivery.total_duration_ms(), 150);
        assert!(delivery.is_success());
    }

    #[tokio::test]
    async fn test_cleanup() {
        let repo = InMemoryDeliveryRepository::default();

        // Add old delivery
        let mut old_delivery = create_test_delivery("d_old", "ep1", DeliveryStatus::Delivered);
        old_delivery.created_at = Utc::now() - Duration::days(60);
        repo.save(&old_delivery).await.unwrap();

        // Add recent delivery
        let new_delivery = create_test_delivery("d_new", "ep1", DeliveryStatus::Delivered);
        repo.save(&new_delivery).await.unwrap();

        // Cleanup older than 30 days
        let cutoff = Utc::now() - Duration::days(30);
        let removed = repo.cleanup(cutoff).await.unwrap();

        assert_eq!(removed, 1);
        assert!(repo.get("d_old").await.unwrap().is_none());
        assert!(repo.get("d_new").await.unwrap().is_some());
    }
}
