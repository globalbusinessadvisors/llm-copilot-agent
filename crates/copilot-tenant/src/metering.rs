//! Usage metering for billing
//!
//! Provides detailed usage tracking for billing purposes.

use crate::{Result, TenantError};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Metered resource types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeteredResource {
    /// API calls
    ApiCalls,
    /// Input tokens
    InputTokens,
    /// Output tokens
    OutputTokens,
    /// Total tokens
    TotalTokens,
    /// Embedding tokens
    EmbeddingTokens,
    /// Storage bytes
    StorageBytes,
    /// Compute seconds
    ComputeSeconds,
    /// Workflow executions
    WorkflowExecutions,
    /// Sandbox executions
    SandboxExecutions,
}

impl MeteredResource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ApiCalls => "api_calls",
            Self::InputTokens => "input_tokens",
            Self::OutputTokens => "output_tokens",
            Self::TotalTokens => "total_tokens",
            Self::EmbeddingTokens => "embedding_tokens",
            Self::StorageBytes => "storage_bytes",
            Self::ComputeSeconds => "compute_seconds",
            Self::WorkflowExecutions => "workflow_executions",
            Self::SandboxExecutions => "sandbox_executions",
        }
    }

    /// Get unit price in cents (example pricing)
    pub fn unit_price_cents(&self) -> f64 {
        match self {
            Self::ApiCalls => 0.001,           // $0.00001 per call
            Self::InputTokens => 0.0003,       // $0.000003 per token
            Self::OutputTokens => 0.0006,      // $0.000006 per token
            Self::TotalTokens => 0.0004,       // $0.000004 per token
            Self::EmbeddingTokens => 0.0001,   // $0.000001 per token
            Self::StorageBytes => 0.00000001,  // $0.10 per GB
            Self::ComputeSeconds => 0.01,      // $0.0001 per second
            Self::WorkflowExecutions => 0.1,   // $0.001 per execution
            Self::SandboxExecutions => 1.0,    // $0.01 per execution
        }
    }
}

/// Usage event for metering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    /// Unique event ID
    pub id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// User ID (if applicable)
    pub user_id: Option<String>,
    /// Resource type
    pub resource: MeteredResource,
    /// Quantity consumed
    pub quantity: u64,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl UsageEvent {
    /// Create a new usage event
    pub fn new(tenant_id: &str, resource: MeteredResource, quantity: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            user_id: None,
            resource,
            quantity,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Set user ID
    pub fn with_user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }

    /// Calculate cost for this event
    pub fn cost_cents(&self) -> f64 {
        self.quantity as f64 * self.resource.unit_price_cents()
    }
}

/// Billing period
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingPeriod {
    /// Daily billing
    Daily,
    /// Weekly billing
    Weekly,
    /// Monthly billing
    Monthly,
    /// Yearly billing
    Yearly,
}

impl Default for BillingPeriod {
    fn default() -> Self {
        Self::Monthly
    }
}

impl BillingPeriod {
    /// Get period start for a given timestamp
    pub fn period_start(&self, ts: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Daily => Utc
                .with_ymd_and_hms(ts.year(), ts.month(), ts.day(), 0, 0, 0)
                .unwrap(),
            Self::Weekly => {
                let days_from_monday = ts.weekday().num_days_from_monday() as i64;
                let start = ts - Duration::days(days_from_monday);
                Utc.with_ymd_and_hms(start.year(), start.month(), start.day(), 0, 0, 0)
                    .unwrap()
            }
            Self::Monthly => Utc
                .with_ymd_and_hms(ts.year(), ts.month(), 1, 0, 0, 0)
                .unwrap(),
            Self::Yearly => Utc
                .with_ymd_and_hms(ts.year(), 1, 1, 0, 0, 0)
                .unwrap(),
        }
    }

    /// Get period end for a given timestamp
    pub fn period_end(&self, ts: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Daily => self.period_start(ts) + Duration::days(1),
            Self::Weekly => self.period_start(ts) + Duration::weeks(1),
            Self::Monthly => {
                let start = self.period_start(ts);
                if start.month() == 12 {
                    Utc.with_ymd_and_hms(start.year() + 1, 1, 1, 0, 0, 0)
                        .unwrap()
                } else {
                    Utc.with_ymd_and_hms(start.year(), start.month() + 1, 1, 0, 0, 0)
                        .unwrap()
                }
            }
            Self::Yearly => Utc
                .with_ymd_and_hms(ts.year() + 1, 1, 1, 0, 0, 0)
                .unwrap(),
        }
    }
}

/// Usage summary for a period
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageSummary {
    /// Tenant ID
    pub tenant_id: String,
    /// Period start
    pub period_start: DateTime<Utc>,
    /// Period end
    pub period_end: DateTime<Utc>,
    /// Usage by resource type
    pub usage: HashMap<MeteredResource, u64>,
    /// Total cost in cents
    pub total_cost_cents: f64,
    /// Number of events
    pub event_count: u64,
}

impl UsageSummary {
    /// Create a new summary
    pub fn new(tenant_id: &str, period_start: DateTime<Utc>, period_end: DateTime<Utc>) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            period_start,
            period_end,
            usage: HashMap::new(),
            total_cost_cents: 0.0,
            event_count: 0,
        }
    }

    /// Add usage
    pub fn add_usage(&mut self, resource: MeteredResource, quantity: u64) {
        *self.usage.entry(resource).or_insert(0) += quantity;
        self.total_cost_cents += quantity as f64 * resource.unit_price_cents();
        self.event_count += 1;
    }

    /// Get usage for a resource
    pub fn get_usage(&self, resource: &MeteredResource) -> u64 {
        self.usage.get(resource).copied().unwrap_or(0)
    }

    /// Get total cost in dollars
    pub fn total_cost_dollars(&self) -> f64 {
        self.total_cost_cents / 100.0
    }
}

/// Metering service for tracking usage
pub struct MeteringService {
    /// Usage events (in-memory for now)
    events: Arc<RwLock<Vec<UsageEvent>>>,
    /// Billing period
    billing_period: BillingPeriod,
}

impl Default for MeteringService {
    fn default() -> Self {
        Self::new(BillingPeriod::Monthly)
    }
}

impl MeteringService {
    pub fn new(billing_period: BillingPeriod) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            billing_period,
        }
    }

    /// Record a usage event
    pub fn record(&self, event: UsageEvent) {
        debug!(
            tenant_id = %event.tenant_id,
            resource = ?event.resource,
            quantity = event.quantity,
            "Recording usage event"
        );
        self.events.write().push(event);
    }

    /// Record API call
    pub fn record_api_call(&self, tenant_id: &str, user_id: Option<&str>) {
        let mut event = UsageEvent::new(tenant_id, MeteredResource::ApiCalls, 1);
        if let Some(uid) = user_id {
            event = event.with_user(uid);
        }
        self.record(event);
    }

    /// Record token usage
    pub fn record_tokens(
        &self,
        tenant_id: &str,
        input_tokens: u64,
        output_tokens: u64,
        user_id: Option<&str>,
    ) {
        if input_tokens > 0 {
            let mut event = UsageEvent::new(tenant_id, MeteredResource::InputTokens, input_tokens);
            if let Some(uid) = user_id {
                event = event.with_user(uid);
            }
            self.record(event);
        }

        if output_tokens > 0 {
            let mut event = UsageEvent::new(tenant_id, MeteredResource::OutputTokens, output_tokens);
            if let Some(uid) = user_id {
                event = event.with_user(uid);
            }
            self.record(event);
        }
    }

    /// Record workflow execution
    pub fn record_workflow_execution(
        &self,
        tenant_id: &str,
        workflow_id: &str,
        duration_seconds: u64,
    ) {
        let event = UsageEvent::new(tenant_id, MeteredResource::WorkflowExecutions, 1)
            .with_metadata("workflow_id", workflow_id)
            .with_metadata("duration_seconds", duration_seconds);
        self.record(event);

        if duration_seconds > 0 {
            let compute_event =
                UsageEvent::new(tenant_id, MeteredResource::ComputeSeconds, duration_seconds)
                    .with_metadata("workflow_id", workflow_id);
            self.record(compute_event);
        }
    }

    /// Record sandbox execution
    pub fn record_sandbox_execution(
        &self,
        tenant_id: &str,
        sandbox_id: &str,
        duration_seconds: u64,
    ) {
        let event = UsageEvent::new(tenant_id, MeteredResource::SandboxExecutions, 1)
            .with_metadata("sandbox_id", sandbox_id)
            .with_metadata("duration_seconds", duration_seconds);
        self.record(event);

        if duration_seconds > 0 {
            let compute_event =
                UsageEvent::new(tenant_id, MeteredResource::ComputeSeconds, duration_seconds)
                    .with_metadata("sandbox_id", sandbox_id);
            self.record(compute_event);
        }
    }

    /// Get usage summary for a tenant
    pub fn get_summary(&self, tenant_id: &str) -> UsageSummary {
        let now = Utc::now();
        let period_start = self.billing_period.period_start(now);
        let period_end = self.billing_period.period_end(now);

        self.get_summary_for_period(tenant_id, period_start, period_end)
    }

    /// Get usage summary for a specific period
    pub fn get_summary_for_period(
        &self,
        tenant_id: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> UsageSummary {
        let events = self.events.read();
        let mut summary = UsageSummary::new(tenant_id, period_start, period_end);

        for event in events.iter() {
            if event.tenant_id == tenant_id
                && event.timestamp >= period_start
                && event.timestamp < period_end
            {
                summary.add_usage(event.resource, event.quantity);
            }
        }

        summary
    }

    /// Get usage breakdown by user
    pub fn get_user_breakdown(&self, tenant_id: &str) -> HashMap<String, UsageSummary> {
        let now = Utc::now();
        let period_start = self.billing_period.period_start(now);
        let period_end = self.billing_period.period_end(now);

        let events = self.events.read();
        let mut breakdown: HashMap<String, UsageSummary> = HashMap::new();

        for event in events.iter() {
            if event.tenant_id == tenant_id
                && event.timestamp >= period_start
                && event.timestamp < period_end
            {
                let user_id = event.user_id.clone().unwrap_or_else(|| "unknown".to_string());
                let summary = breakdown
                    .entry(user_id.clone())
                    .or_insert_with(|| UsageSummary::new(&user_id, period_start, period_end));
                summary.add_usage(event.resource, event.quantity);
            }
        }

        breakdown
    }

    /// Export events for a period
    pub fn export_events(
        &self,
        tenant_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<UsageEvent> {
        self.events
            .read()
            .iter()
            .filter(|e| e.tenant_id == tenant_id && e.timestamp >= start && e.timestamp < end)
            .cloned()
            .collect()
    }

    /// Clear old events (for maintenance)
    pub fn clear_events_before(&self, before: DateTime<Utc>) -> usize {
        let mut events = self.events.write();
        let original_len = events.len();
        events.retain(|e| e.timestamp >= before);
        let removed = original_len - events.len();
        info!(removed = removed, "Cleared old metering events");
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_usage_event() {
        let event = UsageEvent::new("tenant-1", MeteredResource::ApiCalls, 100)
            .with_user("user-1")
            .with_metadata("endpoint", "/api/chat");

        assert_eq!(event.tenant_id, "tenant-1");
        assert_eq!(event.user_id, Some("user-1".to_string()));
        assert_eq!(event.quantity, 100);
        assert!(event.cost_cents() > 0.0);
    }

    #[test]
    fn test_billing_period_monthly() {
        let period = BillingPeriod::Monthly;
        let ts = Utc.with_ymd_and_hms(2024, 6, 15, 12, 30, 0).unwrap();

        let start = period.period_start(ts);
        let end = period.period_end(ts);

        assert_eq!(start.day(), 1);
        assert_eq!(start.month(), 6);
        assert_eq!(end.day(), 1);
        assert_eq!(end.month(), 7);
    }

    #[test]
    fn test_billing_period_daily() {
        let period = BillingPeriod::Daily;
        let ts = Utc.with_ymd_and_hms(2024, 6, 15, 12, 30, 0).unwrap();

        let start = period.period_start(ts);
        let end = period.period_end(ts);

        assert_eq!(start.hour(), 0);
        assert_eq!(start.day(), 15);
        assert_eq!(end.day(), 16);
    }

    #[test]
    fn test_usage_summary() {
        let mut summary = UsageSummary::new(
            "tenant-1",
            Utc::now() - Duration::days(30),
            Utc::now(),
        );

        summary.add_usage(MeteredResource::ApiCalls, 1000);
        summary.add_usage(MeteredResource::InputTokens, 50000);
        summary.add_usage(MeteredResource::OutputTokens, 25000);

        assert_eq!(summary.get_usage(&MeteredResource::ApiCalls), 1000);
        assert_eq!(summary.get_usage(&MeteredResource::InputTokens), 50000);
        assert!(summary.total_cost_cents > 0.0);
        assert_eq!(summary.event_count, 3);
    }

    #[test]
    fn test_metering_service() {
        let service = MeteringService::new(BillingPeriod::Monthly);

        service.record_api_call("tenant-1", Some("user-1"));
        service.record_tokens("tenant-1", 100, 50, Some("user-1"));
        service.record_api_call("tenant-1", Some("user-2"));

        let summary = service.get_summary("tenant-1");
        assert_eq!(summary.get_usage(&MeteredResource::ApiCalls), 2);
        assert_eq!(summary.get_usage(&MeteredResource::InputTokens), 100);
        assert_eq!(summary.get_usage(&MeteredResource::OutputTokens), 50);
    }

    #[test]
    fn test_user_breakdown() {
        let service = MeteringService::new(BillingPeriod::Monthly);

        service.record_api_call("tenant-1", Some("user-1"));
        service.record_api_call("tenant-1", Some("user-1"));
        service.record_api_call("tenant-1", Some("user-2"));

        let breakdown = service.get_user_breakdown("tenant-1");
        assert!(breakdown.contains_key("user-1"));
        assert!(breakdown.contains_key("user-2"));
        assert_eq!(breakdown["user-1"].get_usage(&MeteredResource::ApiCalls), 2);
        assert_eq!(breakdown["user-2"].get_usage(&MeteredResource::ApiCalls), 1);
    }

    #[test]
    fn test_workflow_recording() {
        let service = MeteringService::new(BillingPeriod::Monthly);

        service.record_workflow_execution("tenant-1", "workflow-1", 30);

        let summary = service.get_summary("tenant-1");
        assert_eq!(summary.get_usage(&MeteredResource::WorkflowExecutions), 1);
        assert_eq!(summary.get_usage(&MeteredResource::ComputeSeconds), 30);
    }
}
