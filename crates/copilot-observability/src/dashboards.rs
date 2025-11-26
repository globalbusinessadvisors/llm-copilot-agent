//! Dashboard data providers
//!
//! Provides data structures and aggregations for dashboards.

use crate::analytics::{AnalyticsEventType, AnalyticsService};
use crate::sla::{SlaMonitor, SlaSummary};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Dashboard time range
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DashboardTimeRange {
    /// Last hour
    LastHour,
    /// Last 24 hours
    Last24Hours,
    /// Last 7 days
    Last7Days,
    /// Last 30 days
    Last30Days,
    /// Custom range
    Custom { start: DateTime<Utc>, end: DateTime<Utc> },
}

impl DashboardTimeRange {
    pub fn to_duration(&self) -> Duration {
        match self {
            Self::LastHour => Duration::hours(1),
            Self::Last24Hours => Duration::hours(24),
            Self::Last7Days => Duration::days(7),
            Self::Last30Days => Duration::days(30),
            Self::Custom { start, end } => *end - *start,
        }
    }

    pub fn start_time(&self) -> DateTime<Utc> {
        match self {
            Self::LastHour => Utc::now() - Duration::hours(1),
            Self::Last24Hours => Utc::now() - Duration::hours(24),
            Self::Last7Days => Utc::now() - Duration::days(7),
            Self::Last30Days => Utc::now() - Duration::days(30),
            Self::Custom { start, .. } => *start,
        }
    }

    pub fn end_time(&self) -> DateTime<Utc> {
        match self {
            Self::Custom { end, .. } => *end,
            _ => Utc::now(),
        }
    }
}

/// Overview dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewDashboard {
    /// Time range
    pub time_range: String,
    /// Total API calls
    pub total_api_calls: u64,
    /// Total chat messages
    pub total_chat_messages: u64,
    /// Total workflow executions
    pub total_workflow_executions: u64,
    /// Active tenants count
    pub active_tenants: u64,
    /// Active users count
    pub active_users: u64,
    /// Average response time (ms)
    pub avg_response_time_ms: f64,
    /// Error rate (percentage)
    pub error_rate: f64,
    /// SLA summary
    pub sla_summary: Option<SlaSummary>,
    /// Timestamp
    pub generated_at: DateTime<Utc>,
}

/// Tenant dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantDashboard {
    /// Tenant ID
    pub tenant_id: String,
    /// Time range
    pub time_range: String,
    /// API calls
    pub api_calls: u64,
    /// Chat messages
    pub chat_messages: u64,
    /// Workflow executions
    pub workflow_executions: u64,
    /// Successful workflows
    pub successful_workflows: u64,
    /// Failed workflows
    pub failed_workflows: u64,
    /// Total tokens used
    pub total_tokens: u64,
    /// Average response time
    pub avg_response_time_ms: f64,
    /// Top users
    pub top_users: Vec<UserActivity>,
    /// Usage over time
    pub usage_timeline: Vec<TimelineDataPoint>,
    /// Timestamp
    pub generated_at: DateTime<Utc>,
}

/// User activity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserActivity {
    /// User ID
    pub user_id: String,
    /// Event count
    pub event_count: u64,
    /// Last active timestamp
    pub last_active: DateTime<Utc>,
}

/// Timeline data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineDataPoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Value
    pub value: f64,
    /// Label
    pub label: Option<String>,
}

/// Performance dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDashboard {
    /// Time range
    pub time_range: String,
    /// Response time percentiles
    pub response_times: ResponseTimeMetrics,
    /// Throughput metrics
    pub throughput: ThroughputMetrics,
    /// Error metrics
    pub errors: ErrorMetrics,
    /// Resource utilization
    pub resources: ResourceMetrics,
    /// Timeline data
    pub timeline: Vec<PerformanceTimelinePoint>,
    /// Timestamp
    pub generated_at: DateTime<Utc>,
}

/// Response time metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTimeMetrics {
    /// P50 response time (ms)
    pub p50: f64,
    /// P95 response time (ms)
    pub p95: f64,
    /// P99 response time (ms)
    pub p99: f64,
    /// Average response time (ms)
    pub avg: f64,
    /// Max response time (ms)
    pub max: f64,
}

/// Throughput metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// Requests per second
    pub requests_per_second: f64,
    /// Total requests
    pub total_requests: u64,
    /// Peak requests per second
    pub peak_rps: f64,
}

/// Error metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Total errors
    pub total_errors: u64,
    /// Error rate (percentage)
    pub error_rate: f64,
    /// Errors by type
    pub by_type: HashMap<String, u64>,
    /// Errors by endpoint
    pub by_endpoint: HashMap<String, u64>,
}

/// Resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage percentage
    pub memory_usage: f64,
    /// Active connections
    pub active_connections: u64,
    /// Database pool utilization
    pub db_pool_utilization: f64,
}

/// Performance timeline point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTimelinePoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Response time (ms)
    pub response_time_ms: f64,
    /// Request count
    pub request_count: u64,
    /// Error count
    pub error_count: u64,
}

/// Dashboard service
pub struct DashboardService {
    analytics: Arc<AnalyticsService>,
    sla_monitor: Arc<SlaMonitor>,
}

impl DashboardService {
    pub fn new(analytics: Arc<AnalyticsService>, sla_monitor: Arc<SlaMonitor>) -> Self {
        Self {
            analytics,
            sla_monitor,
        }
    }

    /// Generate overview dashboard
    pub fn generate_overview(&self, time_range: DashboardTimeRange) -> OverviewDashboard {
        let counts = self.analytics.count_by_type(None);

        let total_api_calls = counts.get(&AnalyticsEventType::ApiCall).copied().unwrap_or(0);
        let total_chat_messages = counts.get(&AnalyticsEventType::ChatMessage).copied().unwrap_or(0);
        let workflow_complete = counts.get(&AnalyticsEventType::WorkflowComplete).copied().unwrap_or(0);
        let workflow_failed = counts.get(&AnalyticsEventType::WorkflowFailed).copied().unwrap_or(0);

        let sla_summary = Some(self.sla_monitor.get_status_summary());

        OverviewDashboard {
            time_range: format!("{:?}", time_range),
            total_api_calls,
            total_chat_messages,
            total_workflow_executions: workflow_complete + workflow_failed,
            active_tenants: 0, // Would need to count unique tenants
            active_users: 0,   // Would need to count unique users
            avg_response_time_ms: 0.0,
            error_rate: if total_api_calls > 0 {
                let errors = counts.get(&AnalyticsEventType::Error).copied().unwrap_or(0);
                (errors as f64 / total_api_calls as f64) * 100.0
            } else {
                0.0
            },
            sla_summary,
            generated_at: Utc::now(),
        }
    }

    /// Generate tenant dashboard
    pub fn generate_tenant_dashboard(
        &self,
        tenant_id: &str,
        time_range: DashboardTimeRange,
    ) -> TenantDashboard {
        let counts = self.analytics.count_by_type(Some(tenant_id));
        let top_users = self.analytics.top_users(tenant_id, 10);

        let api_calls = counts.get(&AnalyticsEventType::ApiCall).copied().unwrap_or(0);
        let chat_messages = counts.get(&AnalyticsEventType::ChatMessage).copied().unwrap_or(0);
        let workflow_complete = counts.get(&AnalyticsEventType::WorkflowComplete).copied().unwrap_or(0);
        let workflow_failed = counts.get(&AnalyticsEventType::WorkflowFailed).copied().unwrap_or(0);

        let top_user_activities: Vec<UserActivity> = top_users
            .into_iter()
            .map(|(user_id, count)| UserActivity {
                user_id,
                event_count: count,
                last_active: Utc::now(), // Would need actual last activity time
            })
            .collect();

        TenantDashboard {
            tenant_id: tenant_id.to_string(),
            time_range: format!("{:?}", time_range),
            api_calls,
            chat_messages,
            workflow_executions: workflow_complete + workflow_failed,
            successful_workflows: workflow_complete,
            failed_workflows: workflow_failed,
            total_tokens: 0, // Would sum from events
            avg_response_time_ms: 0.0,
            top_users: top_user_activities,
            usage_timeline: Vec::new(),
            generated_at: Utc::now(),
        }
    }

    /// Generate performance dashboard
    pub fn generate_performance_dashboard(
        &self,
        time_range: DashboardTimeRange,
    ) -> PerformanceDashboard {
        PerformanceDashboard {
            time_range: format!("{:?}", time_range),
            response_times: ResponseTimeMetrics {
                p50: 0.0,
                p95: 0.0,
                p99: 0.0,
                avg: 0.0,
                max: 0.0,
            },
            throughput: ThroughputMetrics {
                requests_per_second: 0.0,
                total_requests: 0,
                peak_rps: 0.0,
            },
            errors: ErrorMetrics {
                total_errors: 0,
                error_rate: 0.0,
                by_type: HashMap::new(),
                by_endpoint: HashMap::new(),
            },
            resources: ResourceMetrics {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                active_connections: 0,
                db_pool_utilization: 0.0,
            },
            timeline: Vec::new(),
            generated_at: Utc::now(),
        }
    }
}

/// Grafana dashboard JSON generator
pub struct GrafanaDashboardGenerator;

impl GrafanaDashboardGenerator {
    /// Generate Grafana dashboard JSON for overview
    pub fn generate_overview_dashboard() -> serde_json::Value {
        serde_json::json!({
            "title": "CoPilot Agent Overview",
            "uid": "copilot-overview",
            "editable": true,
            "panels": [
                {
                    "title": "Total API Calls",
                    "type": "stat",
                    "gridPos": { "h": 4, "w": 6, "x": 0, "y": 0 },
                    "targets": [{
                        "expr": "sum(rate(copilot_api_calls_total[5m]))",
                        "legendFormat": "API Calls/s"
                    }]
                },
                {
                    "title": "Response Time (P99)",
                    "type": "stat",
                    "gridPos": { "h": 4, "w": 6, "x": 6, "y": 0 },
                    "targets": [{
                        "expr": "histogram_quantile(0.99, rate(copilot_http_request_duration_seconds_bucket[5m]))",
                        "legendFormat": "P99 Latency"
                    }]
                },
                {
                    "title": "Error Rate",
                    "type": "stat",
                    "gridPos": { "h": 4, "w": 6, "x": 12, "y": 0 },
                    "targets": [{
                        "expr": "sum(rate(copilot_http_requests_total{status=~\"5..\"}[5m])) / sum(rate(copilot_http_requests_total[5m])) * 100",
                        "legendFormat": "Error Rate %"
                    }]
                },
                {
                    "title": "Active Connections",
                    "type": "stat",
                    "gridPos": { "h": 4, "w": 6, "x": 18, "y": 0 },
                    "targets": [{
                        "expr": "copilot_active_connections",
                        "legendFormat": "Connections"
                    }]
                },
                {
                    "title": "Request Rate",
                    "type": "timeseries",
                    "gridPos": { "h": 8, "w": 12, "x": 0, "y": 4 },
                    "targets": [{
                        "expr": "sum(rate(copilot_http_requests_total[1m]))",
                        "legendFormat": "Requests/s"
                    }]
                },
                {
                    "title": "Response Time Distribution",
                    "type": "timeseries",
                    "gridPos": { "h": 8, "w": 12, "x": 12, "y": 4 },
                    "targets": [
                        {
                            "expr": "histogram_quantile(0.50, rate(copilot_http_request_duration_seconds_bucket[5m]))",
                            "legendFormat": "P50"
                        },
                        {
                            "expr": "histogram_quantile(0.95, rate(copilot_http_request_duration_seconds_bucket[5m]))",
                            "legendFormat": "P95"
                        },
                        {
                            "expr": "histogram_quantile(0.99, rate(copilot_http_request_duration_seconds_bucket[5m]))",
                            "legendFormat": "P99"
                        }
                    ]
                }
            ],
            "time": { "from": "now-1h", "to": "now" },
            "refresh": "5s"
        })
    }

    /// Generate Grafana dashboard JSON for tenant
    pub fn generate_tenant_dashboard(tenant_id: &str) -> serde_json::Value {
        serde_json::json!({
            "title": format!("CoPilot Agent - Tenant {}", tenant_id),
            "uid": format!("copilot-tenant-{}", tenant_id),
            "editable": true,
            "templating": {
                "list": [{
                    "name": "tenant_id",
                    "type": "constant",
                    "current": { "value": tenant_id }
                }]
            },
            "panels": [
                {
                    "title": "API Calls",
                    "type": "stat",
                    "gridPos": { "h": 4, "w": 6, "x": 0, "y": 0 },
                    "targets": [{
                        "expr": format!("sum(rate(copilot_api_calls_total{{tenant_id=\"{}\"}}[5m]))", tenant_id),
                        "legendFormat": "API Calls/s"
                    }]
                },
                {
                    "title": "Token Usage",
                    "type": "stat",
                    "gridPos": { "h": 4, "w": 6, "x": 6, "y": 0 },
                    "targets": [{
                        "expr": format!("sum(copilot_tokens_total{{tenant_id=\"{}\"}})", tenant_id),
                        "legendFormat": "Total Tokens"
                    }]
                },
                {
                    "title": "Workflow Success Rate",
                    "type": "gauge",
                    "gridPos": { "h": 4, "w": 6, "x": 12, "y": 0 },
                    "targets": [{
                        "expr": format!("sum(copilot_workflow_success_total{{tenant_id=\"{}\"}}) / sum(copilot_workflow_total{{tenant_id=\"{}\"}}) * 100", tenant_id, tenant_id),
                        "legendFormat": "Success Rate"
                    }]
                },
                {
                    "title": "Usage Over Time",
                    "type": "timeseries",
                    "gridPos": { "h": 8, "w": 24, "x": 0, "y": 4 },
                    "targets": [{
                        "expr": format!("sum(rate(copilot_api_calls_total{{tenant_id=\"{}\"}}[5m]))", tenant_id),
                        "legendFormat": "API Calls"
                    }]
                }
            ],
            "time": { "from": "now-24h", "to": "now" },
            "refresh": "30s"
        })
    }

    /// Generate Grafana dashboard JSON for SLA monitoring
    pub fn generate_sla_dashboard() -> serde_json::Value {
        serde_json::json!({
            "title": "CoPilot Agent - SLA Monitoring",
            "uid": "copilot-sla",
            "editable": true,
            "panels": [
                {
                    "title": "Availability",
                    "type": "gauge",
                    "gridPos": { "h": 6, "w": 8, "x": 0, "y": 0 },
                    "fieldConfig": {
                        "defaults": {
                            "thresholds": {
                                "steps": [
                                    { "value": 0, "color": "red" },
                                    { "value": 99.5, "color": "yellow" },
                                    { "value": 99.9, "color": "green" }
                                ]
                            },
                            "unit": "percent",
                            "max": 100
                        }
                    },
                    "targets": [{
                        "expr": "(1 - sum(rate(copilot_http_requests_total{status=~\"5..\"}[5m])) / sum(rate(copilot_http_requests_total[5m]))) * 100",
                        "legendFormat": "Availability"
                    }]
                },
                {
                    "title": "P99 Response Time",
                    "type": "gauge",
                    "gridPos": { "h": 6, "w": 8, "x": 8, "y": 0 },
                    "fieldConfig": {
                        "defaults": {
                            "thresholds": {
                                "steps": [
                                    { "value": 0, "color": "green" },
                                    { "value": 80, "color": "yellow" },
                                    { "value": 100, "color": "red" }
                                ]
                            },
                            "unit": "ms",
                            "max": 200
                        }
                    },
                    "targets": [{
                        "expr": "histogram_quantile(0.99, rate(copilot_http_request_duration_seconds_bucket[5m])) * 1000",
                        "legendFormat": "P99"
                    }]
                },
                {
                    "title": "Error Rate",
                    "type": "gauge",
                    "gridPos": { "h": 6, "w": 8, "x": 16, "y": 0 },
                    "fieldConfig": {
                        "defaults": {
                            "thresholds": {
                                "steps": [
                                    { "value": 0, "color": "green" },
                                    { "value": 0.5, "color": "yellow" },
                                    { "value": 1, "color": "red" }
                                ]
                            },
                            "unit": "percent",
                            "max": 5
                        }
                    },
                    "targets": [{
                        "expr": "sum(rate(copilot_http_requests_total{status=~\"5..\"}[5m])) / sum(rate(copilot_http_requests_total[5m])) * 100",
                        "legendFormat": "Error Rate"
                    }]
                },
                {
                    "title": "SLA Compliance History",
                    "type": "timeseries",
                    "gridPos": { "h": 8, "w": 24, "x": 0, "y": 6 },
                    "targets": [
                        {
                            "expr": "(1 - sum(rate(copilot_http_requests_total{status=~\"5..\"}[5m])) / sum(rate(copilot_http_requests_total[5m]))) * 100",
                            "legendFormat": "Availability %"
                        }
                    ],
                    "fieldConfig": {
                        "defaults": {
                            "custom": {
                                "thresholdsStyle": {
                                    "mode": "line"
                                }
                            },
                            "thresholds": {
                                "steps": [
                                    { "value": 99.9, "color": "green" }
                                ]
                            }
                        }
                    }
                }
            ],
            "time": { "from": "now-7d", "to": "now" },
            "refresh": "1m"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range() {
        let range = DashboardTimeRange::Last24Hours;
        let duration = range.to_duration();
        assert_eq!(duration, Duration::hours(24));
    }

    #[test]
    fn test_grafana_dashboard_generation() {
        let overview = GrafanaDashboardGenerator::generate_overview_dashboard();
        assert!(overview["title"].as_str().is_some());
        assert!(overview["panels"].as_array().is_some());

        let tenant = GrafanaDashboardGenerator::generate_tenant_dashboard("tenant-123");
        assert!(tenant["title"].as_str().unwrap().contains("tenant-123"));

        let sla = GrafanaDashboardGenerator::generate_sla_dashboard();
        assert!(sla["title"].as_str().unwrap().contains("SLA"));
    }

    #[test]
    fn test_dashboard_service() {
        let analytics = Arc::new(AnalyticsService::new(1000));
        let sla_monitor = Arc::new(SlaMonitor::with_standard_targets());
        let service = DashboardService::new(analytics, sla_monitor);

        let overview = service.generate_overview(DashboardTimeRange::Last24Hours);
        assert!(overview.generated_at <= Utc::now());

        let tenant_dashboard = service.generate_tenant_dashboard("tenant-1", DashboardTimeRange::Last7Days);
        assert_eq!(tenant_dashboard.tenant_id, "tenant-1");
    }
}
