//! LLM-Observatory Adapter
//!
//! Thin adapter for consuming LLM-Observatory service.
//! Provides observability, monitoring, and tracing for LLM operations.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

use crate::{
    AdapterError, AdapterResult, HealthStatus, ModuleCapabilities, ModuleAdapter,
    circuit_breaker::CircuitBreaker,
    retry::with_retry,
};

/// LLM operation span for tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation: String,
    pub model: String,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub latency_ms: u64,
    pub status: SpanStatus,
    pub metadata: HashMap<String, serde_json::Value>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanStatus {
    Ok,
    Error { message: String },
    Timeout,
    RateLimited,
}

/// Query for traces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceQuery {
    pub trace_id: Option<String>,
    pub model: Option<String>,
    pub operation: Option<String>,
    pub status: Option<SpanStatus>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceQueryResponse {
    pub spans: Vec<LLMSpan>,
    pub total_count: u64,
}

/// Metrics query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMetricsQuery {
    pub metric_name: String,
    pub labels: HashMap<String, String>,
    pub aggregation: Aggregation,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub step: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Aggregation {
    Sum,
    Avg,
    Min,
    Max,
    Count,
    P50,
    P90,
    P99,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMetricsResponse {
    pub metric_name: String,
    pub values: Vec<MetricValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

/// Alert definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertDefinition {
    pub id: String,
    pub name: String,
    pub condition: String,
    pub threshold: f64,
    pub severity: AlertSeverity,
    pub channels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Trait defining LLMObservatory adapter operations
#[async_trait]
pub trait LLMObservatoryAdapter: Send + Sync {
    /// Record a span
    async fn record_span(&self, span: LLMSpan) -> AdapterResult<()>;

    /// Query traces
    async fn query_traces(&self, query: TraceQuery) -> AdapterResult<TraceQueryResponse>;

    /// Query metrics
    async fn query_metrics(&self, query: LLMMetricsQuery) -> AdapterResult<LLMMetricsResponse>;

    /// Create an alert
    async fn create_alert(&self, alert: AlertDefinition) -> AdapterResult<AlertDefinition>;

    /// Get dashboard data
    async fn get_dashboard(&self, dashboard_id: &str) -> AdapterResult<DashboardData>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub id: String,
    pub name: String,
    pub panels: Vec<DashboardPanel>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPanel {
    pub id: String,
    pub title: String,
    pub panel_type: String,
    pub data: serde_json::Value,
}

/// HTTP client for LLM-Observatory service
pub struct LLMObservatoryClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl LLMObservatoryClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
            circuit_breaker: CircuitBreaker::default(),
        }
    }

    async fn send_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&T>,
    ) -> AdapterResult<R> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Sending {} request to {}", method, url);

        let response = with_retry(3, || async {
            self.circuit_breaker.call(|| async {
                let mut request = self.client.request(method.clone(), &url);
                if let Some(body) = body {
                    request = request.json(body);
                }
                request
                    .send()
                    .await
                    .map_err(|e| AdapterError::RequestFailed(e.to_string()))
            }).await
        }).await?;

        if !response.status().is_success() {
            return Err(AdapterError::RequestFailed(format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        response
            .json::<R>()
            .await
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }
}

#[async_trait]
impl ModuleAdapter for LLMObservatoryClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Observatory is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let query: TraceQuery = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.query_traces(query).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Observatory".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "distributed_tracing".to_string(),
                "metrics".to_string(),
                "alerting".to_string(),
                "dashboards".to_string(),
            ],
            endpoints: vec![
                "/spans".to_string(),
                "/traces".to_string(),
                "/metrics".to_string(),
                "/alerts".to_string(),
                "/dashboards/{id}".to_string(),
            ],
        })
    }
}

#[async_trait]
impl LLMObservatoryAdapter for LLMObservatoryClient {
    async fn record_span(&self, span: LLMSpan) -> AdapterResult<()> {
        debug!("Recording span: {}", span.span_id);
        self.send_request::<LLMSpan, ()>(reqwest::Method::POST, "/spans", Some(&span)).await
    }

    async fn query_traces(&self, query: TraceQuery) -> AdapterResult<TraceQueryResponse> {
        debug!("Querying traces");
        self.send_request(reqwest::Method::POST, "/traces/query", Some(&query)).await
    }

    async fn query_metrics(&self, query: LLMMetricsQuery) -> AdapterResult<LLMMetricsResponse> {
        debug!("Querying metrics: {}", query.metric_name);
        self.send_request(reqwest::Method::POST, "/metrics/query", Some(&query)).await
    }

    async fn create_alert(&self, alert: AlertDefinition) -> AdapterResult<AlertDefinition> {
        info!("Creating alert: {}", alert.name);
        self.send_request(reqwest::Method::POST, "/alerts", Some(&alert)).await
    }

    async fn get_dashboard(&self, dashboard_id: &str) -> AdapterResult<DashboardData> {
        debug!("Getting dashboard: {}", dashboard_id);
        let path = format!("/dashboards/{}", dashboard_id);
        self.send_request::<(), DashboardData>(reqwest::Method::GET, &path, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = LLMObservatoryClient::new("http://localhost:8095");
        assert_eq!(client.base_url, "http://localhost:8095");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = LLMObservatoryClient::new("http://localhost:8095");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Observatory");
    }
}
