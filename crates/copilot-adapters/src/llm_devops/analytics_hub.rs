//! LLM-Analytics-Hub Adapter
//!
//! Thin adapter for consuming LLM-Analytics-Hub service.
//! Provides analytics, insights, and data visualization.

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

/// Analytics query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsQuery {
    pub query_type: QueryType,
    pub dimensions: Vec<String>,
    pub metrics: Vec<String>,
    pub filters: Vec<QueryFilter>,
    pub time_range: TimeRange,
    pub granularity: Granularity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    TimeSeries,
    Aggregation,
    Distribution,
    Comparison,
    Funnel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    In,
    GreaterThan,
    LessThan,
    Between,
    Contains,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Granularity {
    Minute,
    Hour,
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    pub query_id: String,
    pub data: Vec<DataPoint>,
    pub metadata: QueryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub dimensions: HashMap<String, String>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetadata {
    pub total_rows: u64,
    pub execution_time_ms: u64,
    pub cached: bool,
}

/// Insight definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: String,
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    pub severity: InsightSeverity,
    pub data: serde_json::Value,
    pub recommendations: Vec<String>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightType {
    CostSpike,
    UsageAnomaly,
    PerformanceDegradation,
    ModelDrift,
    Opportunity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightSeverity {
    Info,
    Warning,
    Critical,
}

/// Trait defining AnalyticsHub adapter operations
#[async_trait]
pub trait AnalyticsHubAdapter: Send + Sync {
    /// Execute an analytics query
    async fn query(&self, query: AnalyticsQuery) -> AdapterResult<AnalyticsResponse>;

    /// Get insights
    async fn get_insights(&self, limit: usize) -> AdapterResult<Vec<Insight>>;

    /// Export data
    async fn export(&self, query: AnalyticsQuery, format: ExportFormat) -> AdapterResult<ExportResponse>;

    /// Create a saved query
    async fn save_query(&self, name: &str, query: AnalyticsQuery) -> AdapterResult<SavedQuery>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    CSV,
    JSON,
    Parquet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    pub export_id: String,
    pub download_url: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub row_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedQuery {
    pub id: String,
    pub name: String,
    pub query: AnalyticsQuery,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// HTTP client for LLM-Analytics-Hub service
pub struct AnalyticsHubClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl AnalyticsHubClient {
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
impl ModuleAdapter for AnalyticsHubClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Analytics-Hub is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let query: AnalyticsQuery = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.query(query).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Analytics-Hub".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "analytics_queries".to_string(),
                "insights".to_string(),
                "data_export".to_string(),
                "saved_queries".to_string(),
            ],
            endpoints: vec![
                "/query".to_string(),
                "/insights".to_string(),
                "/export".to_string(),
                "/queries/saved".to_string(),
            ],
        })
    }
}

#[async_trait]
impl AnalyticsHubAdapter for AnalyticsHubClient {
    async fn query(&self, query: AnalyticsQuery) -> AdapterResult<AnalyticsResponse> {
        info!("Executing {:?} query", query.query_type);
        self.send_request(reqwest::Method::POST, "/query", Some(&query)).await
    }

    async fn get_insights(&self, limit: usize) -> AdapterResult<Vec<Insight>> {
        debug!("Getting insights (limit: {})", limit);
        let path = format!("/insights?limit={}", limit);
        self.send_request::<(), Vec<Insight>>(reqwest::Method::GET, &path, None).await
    }

    async fn export(&self, query: AnalyticsQuery, format: ExportFormat) -> AdapterResult<ExportResponse> {
        info!("Exporting data in {:?} format", format);
        let request = serde_json::json!({
            "query": query,
            "format": format
        });
        self.send_request(reqwest::Method::POST, "/export", Some(&request)).await
    }

    async fn save_query(&self, name: &str, query: AnalyticsQuery) -> AdapterResult<SavedQuery> {
        info!("Saving query: {}", name);
        let request = serde_json::json!({
            "name": name,
            "query": query
        });
        self.send_request(reqwest::Method::POST, "/queries/saved", Some(&request)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = AnalyticsHubClient::new("http://localhost:8103");
        assert_eq!(client.base_url, "http://localhost:8103");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = AnalyticsHubClient::new("http://localhost:8103");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Analytics-Hub");
    }
}
