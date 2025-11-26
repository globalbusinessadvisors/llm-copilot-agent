//! Business analytics and metrics
//!
//! Provides custom business metrics and usage analytics.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::debug;

/// Analytics event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsEventType {
    /// User session started
    SessionStart,
    /// User session ended
    SessionEnd,
    /// Chat message sent
    ChatMessage,
    /// Chat response received
    ChatResponse,
    /// Workflow started
    WorkflowStart,
    /// Workflow completed
    WorkflowComplete,
    /// Workflow failed
    WorkflowFailed,
    /// Context search performed
    ContextSearch,
    /// Document uploaded
    DocumentUpload,
    /// API call made
    ApiCall,
    /// Error occurred
    Error,
    /// Feature used
    FeatureUsed,
    /// Custom event
    Custom,
}

/// Analytics event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    /// Event ID
    pub id: String,
    /// Event type
    pub event_type: AnalyticsEventType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Tenant ID
    pub tenant_id: Option<String>,
    /// User ID
    pub user_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Event properties
    pub properties: HashMap<String, serde_json::Value>,
    /// Duration in milliseconds (for timed events)
    pub duration_ms: Option<u64>,
    /// Success flag
    pub success: Option<bool>,
}

impl AnalyticsEvent {
    /// Create a new analytics event
    pub fn new(event_type: AnalyticsEventType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            timestamp: Utc::now(),
            tenant_id: None,
            user_id: None,
            session_id: None,
            properties: HashMap::new(),
            duration_ms: None,
            success: None,
        }
    }

    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.tenant_id = Some(tenant_id.to_string());
        self
    }

    pub fn with_user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    pub fn with_property(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.properties.insert(key.to_string(), value.into());
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = Some(success);
        self
    }
}

/// Time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Value
    pub value: f64,
    /// Labels/dimensions
    pub labels: HashMap<String, String>,
}

impl DataPoint {
    pub fn new(value: f64) -> Self {
        Self {
            timestamp: Utc::now(),
            value,
            labels: HashMap::new(),
        }
    }

    pub fn with_label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }
}

/// Analytics aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsAggregate {
    /// Time bucket start
    pub bucket_start: DateTime<Utc>,
    /// Time bucket end
    pub bucket_end: DateTime<Utc>,
    /// Event count
    pub count: u64,
    /// Sum of values
    pub sum: f64,
    /// Average value
    pub avg: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Grouping key
    pub group_key: Option<String>,
}

impl Default for AnalyticsAggregate {
    fn default() -> Self {
        Self {
            bucket_start: Utc::now(),
            bucket_end: Utc::now(),
            count: 0,
            sum: 0.0,
            avg: 0.0,
            min: f64::MAX,
            max: f64::MIN,
        group_key: None,
        }
    }
}

impl AnalyticsAggregate {
    pub fn new(bucket_start: DateTime<Utc>, bucket_end: DateTime<Utc>) -> Self {
        Self {
            bucket_start,
            bucket_end,
            ..Default::default()
        }
    }

    pub fn add(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        self.avg = self.sum / self.count as f64;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }
}

/// Analytics query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsQuery {
    /// Event types to include
    pub event_types: Option<Vec<AnalyticsEventType>>,
    /// Tenant filter
    pub tenant_id: Option<String>,
    /// User filter
    pub user_id: Option<String>,
    /// Start time
    pub start_time: Option<DateTime<Utc>>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Group by field
    pub group_by: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
}

impl Default for AnalyticsQuery {
    fn default() -> Self {
        Self {
            event_types: None,
            tenant_id: None,
            user_id: None,
            start_time: Some(Utc::now() - Duration::days(7)),
            end_time: Some(Utc::now()),
            group_by: None,
            limit: Some(1000),
        }
    }
}

/// Analytics service
pub struct AnalyticsService {
    /// Events storage
    events: Arc<RwLock<Vec<AnalyticsEvent>>>,
    /// Data points storage
    data_points: Arc<RwLock<HashMap<String, Vec<DataPoint>>>>,
    /// Maximum events to store
    max_events: usize,
}

impl Default for AnalyticsService {
    fn default() -> Self {
        Self::new(100_000)
    }
}

impl AnalyticsService {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            data_points: Arc::new(RwLock::new(HashMap::new())),
            max_events,
        }
    }

    /// Track an analytics event
    pub fn track(&self, event: AnalyticsEvent) {
        let mut events = self.events.write();
        events.push(event);

        // Trim if over limit
        if events.len() > self.max_events {
            let drain_count = events.len() - self.max_events;
            events.drain(0..drain_count);
        }
    }

    /// Track a chat message
    pub fn track_chat(&self, tenant_id: &str, user_id: &str, tokens: u64, duration_ms: u64) {
        self.track(
            AnalyticsEvent::new(AnalyticsEventType::ChatMessage)
                .with_tenant(tenant_id)
                .with_user(user_id)
                .with_property("tokens", tokens)
                .with_duration(duration_ms)
                .with_success(true),
        );
    }

    /// Track a workflow execution
    pub fn track_workflow(
        &self,
        tenant_id: &str,
        workflow_id: &str,
        success: bool,
        duration_ms: u64,
    ) {
        let event_type = if success {
            AnalyticsEventType::WorkflowComplete
        } else {
            AnalyticsEventType::WorkflowFailed
        };

        self.track(
            AnalyticsEvent::new(event_type)
                .with_tenant(tenant_id)
                .with_property("workflow_id", workflow_id)
                .with_duration(duration_ms)
                .with_success(success),
        );
    }

    /// Track an API call
    pub fn track_api_call(
        &self,
        tenant_id: &str,
        endpoint: &str,
        method: &str,
        status_code: u16,
        duration_ms: u64,
    ) {
        self.track(
            AnalyticsEvent::new(AnalyticsEventType::ApiCall)
                .with_tenant(tenant_id)
                .with_property("endpoint", endpoint)
                .with_property("method", method)
                .with_property("status_code", status_code)
                .with_duration(duration_ms)
                .with_success(status_code < 400),
        );
    }

    /// Record a data point
    pub fn record_data_point(&self, metric_name: &str, point: DataPoint) {
        let mut data_points = self.data_points.write();
        data_points
            .entry(metric_name.to_string())
            .or_default()
            .push(point);
    }

    /// Query events
    pub fn query(&self, query: &AnalyticsQuery) -> Vec<AnalyticsEvent> {
        let events = self.events.read();

        events
            .iter()
            .filter(|e| {
                // Filter by event type
                if let Some(ref types) = query.event_types {
                    if !types.contains(&e.event_type) {
                        return false;
                    }
                }

                // Filter by tenant
                if let Some(ref tenant_id) = query.tenant_id {
                    if e.tenant_id.as_ref() != Some(tenant_id) {
                        return false;
                    }
                }

                // Filter by user
                if let Some(ref user_id) = query.user_id {
                    if e.user_id.as_ref() != Some(user_id) {
                        return false;
                    }
                }

                // Filter by time range
                if let Some(start) = query.start_time {
                    if e.timestamp < start {
                        return false;
                    }
                }

                if let Some(end) = query.end_time {
                    if e.timestamp > end {
                        return false;
                    }
                }

                true
            })
            .take(query.limit.unwrap_or(1000))
            .cloned()
            .collect()
    }

    /// Get event counts by type
    pub fn count_by_type(&self, tenant_id: Option<&str>) -> HashMap<AnalyticsEventType, u64> {
        let events = self.events.read();
        let mut counts = HashMap::new();

        for event in events.iter() {
            if let Some(tid) = tenant_id {
                if event.tenant_id.as_ref() != Some(&tid.to_string()) {
                    continue;
                }
            }

            *counts.entry(event.event_type).or_insert(0) += 1;
        }

        counts
    }

    /// Get time series aggregates
    pub fn aggregate_time_series(
        &self,
        event_type: AnalyticsEventType,
        bucket_minutes: i64,
        tenant_id: Option<&str>,
    ) -> Vec<AnalyticsAggregate> {
        let events = self.events.read();
        let mut buckets: HashMap<i64, AnalyticsAggregate> = HashMap::new();

        let bucket_duration = Duration::minutes(bucket_minutes);

        for event in events.iter() {
            if event.event_type != event_type {
                continue;
            }

            if let Some(tid) = tenant_id {
                if event.tenant_id.as_ref() != Some(&tid.to_string()) {
                    continue;
                }
            }

            let bucket_start = event.timestamp.timestamp() / (bucket_minutes * 60) * (bucket_minutes * 60);
            let bucket_start_dt = DateTime::from_timestamp(bucket_start, 0).unwrap_or(Utc::now());

            let agg = buckets.entry(bucket_start).or_insert_with(|| {
                AnalyticsAggregate::new(bucket_start_dt, bucket_start_dt + bucket_duration)
            });

            let value = event.duration_ms.unwrap_or(1) as f64;
            agg.add(value);
        }

        let mut result: Vec<_> = buckets.into_values().collect();
        result.sort_by_key(|a| a.bucket_start);
        result
    }

    /// Get top users by event count
    pub fn top_users(&self, tenant_id: &str, limit: usize) -> Vec<(String, u64)> {
        let events = self.events.read();
        let mut user_counts: HashMap<String, u64> = HashMap::new();

        for event in events.iter() {
            if event.tenant_id.as_ref() != Some(&tenant_id.to_string()) {
                continue;
            }

            if let Some(ref user_id) = event.user_id {
                *user_counts.entry(user_id.clone()).or_insert(0) += 1;
            }
        }

        let mut sorted: Vec<_> = user_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(limit);
        sorted
    }

    /// Get success rate for an event type
    pub fn success_rate(&self, event_type: AnalyticsEventType, tenant_id: Option<&str>) -> f64 {
        let events = self.events.read();
        let mut total = 0u64;
        let mut success = 0u64;

        for event in events.iter() {
            if event.event_type != event_type {
                continue;
            }

            if let Some(tid) = tenant_id {
                if event.tenant_id.as_ref() != Some(&tid.to_string()) {
                    continue;
                }
            }

            total += 1;
            if event.success == Some(true) {
                success += 1;
            }
        }

        if total == 0 {
            100.0
        } else {
            (success as f64 / total as f64) * 100.0
        }
    }

    /// Clear old events
    pub fn cleanup(&self, max_age: Duration) {
        let cutoff = Utc::now() - max_age;
        let mut events = self.events.write();
        let before = events.len();
        events.retain(|e| e.timestamp > cutoff);
        let after = events.len();
        debug!(removed = before - after, "Cleaned up old analytics events");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_event() {
        let event = AnalyticsEvent::new(AnalyticsEventType::ChatMessage)
            .with_tenant("tenant-1")
            .with_user("user-1")
            .with_property("tokens", 100)
            .with_duration(500)
            .with_success(true);

        assert_eq!(event.event_type, AnalyticsEventType::ChatMessage);
        assert_eq!(event.tenant_id, Some("tenant-1".to_string()));
        assert_eq!(event.duration_ms, Some(500));
        assert_eq!(event.success, Some(true));
    }

    #[test]
    fn test_analytics_service() {
        let service = AnalyticsService::new(1000);

        service.track_chat("tenant-1", "user-1", 100, 500);
        service.track_chat("tenant-1", "user-1", 200, 600);
        service.track_chat("tenant-1", "user-2", 150, 400);

        let events = service.query(&AnalyticsQuery {
            tenant_id: Some("tenant-1".to_string()),
            ..Default::default()
        });

        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_count_by_type() {
        let service = AnalyticsService::new(1000);

        service.track_chat("tenant-1", "user-1", 100, 500);
        service.track_workflow("tenant-1", "wf-1", true, 1000);
        service.track_workflow("tenant-1", "wf-2", false, 500);

        let counts = service.count_by_type(Some("tenant-1"));

        assert_eq!(counts.get(&AnalyticsEventType::ChatMessage), Some(&1));
        assert_eq!(counts.get(&AnalyticsEventType::WorkflowComplete), Some(&1));
        assert_eq!(counts.get(&AnalyticsEventType::WorkflowFailed), Some(&1));
    }

    #[test]
    fn test_success_rate() {
        let service = AnalyticsService::new(1000);

        service.track_workflow("tenant-1", "wf-1", true, 1000);
        service.track_workflow("tenant-1", "wf-2", true, 1000);
        service.track_workflow("tenant-1", "wf-3", false, 500);

        let rate = service.success_rate(AnalyticsEventType::WorkflowComplete, Some("tenant-1"));
        assert_eq!(rate, 100.0); // Only completed workflows

        // For all workflow events (complete + failed), we need different logic
    }

    #[test]
    fn test_top_users() {
        let service = AnalyticsService::new(1000);

        for _ in 0..5 {
            service.track_chat("tenant-1", "user-1", 100, 500);
        }
        for _ in 0..3 {
            service.track_chat("tenant-1", "user-2", 100, 500);
        }
        service.track_chat("tenant-1", "user-3", 100, 500);

        let top = service.top_users("tenant-1", 2);

        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "user-1");
        assert_eq!(top[0].1, 5);
        assert_eq!(top[1].0, "user-2");
        assert_eq!(top[1].1, 3);
    }

    #[test]
    fn test_data_point() {
        let point = DataPoint::new(42.0)
            .with_label("endpoint", "/api/chat")
            .with_label("method", "POST");

        assert_eq!(point.value, 42.0);
        assert_eq!(point.labels.get("endpoint"), Some(&"/api/chat".to_string()));
    }
}
