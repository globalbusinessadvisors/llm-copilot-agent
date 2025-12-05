//! LLM-CostOps Adapter
//!
//! Thin adapter for consuming LLM-CostOps service.
//! Provides cost tracking, optimization, and budget management.

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

/// Cost tracking request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTrackingRequest {
    pub tenant_id: String,
    pub model: String,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub request_metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTrackingResponse {
    pub tracking_id: String,
    pub cost_usd: f64,
    pub running_total_usd: f64,
    pub budget_remaining_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    pub tenant_id: String,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
    pub total_cost_usd: f64,
    pub cost_by_model: HashMap<String, f64>,
    pub request_count: u64,
    pub avg_cost_per_request: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    pub tenant_id: String,
    pub monthly_budget_usd: f64,
    pub alert_thresholds: Vec<f64>,
    pub hard_limit: bool,
}

/// Trait defining CostOps adapter operations
#[async_trait]
pub trait CostOpsAdapter: Send + Sync {
    /// Track cost for a request
    async fn track_cost(&self, request: CostTrackingRequest) -> AdapterResult<CostTrackingResponse>;

    /// Get cost report for a tenant
    async fn get_report(&self, tenant_id: &str, period_days: u32) -> AdapterResult<CostReport>;

    /// Set budget configuration
    async fn set_budget(&self, config: BudgetConfig) -> AdapterResult<()>;

    /// Get optimization recommendations
    async fn get_recommendations(&self, tenant_id: &str) -> AdapterResult<Vec<CostRecommendation>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecommendation {
    pub id: String,
    pub recommendation_type: String,
    pub description: String,
    pub estimated_savings_usd: f64,
    pub confidence: f32,
}

/// HTTP client for LLM-CostOps service
pub struct CostOpsClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl CostOpsClient {
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
impl ModuleAdapter for CostOpsClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-CostOps is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let track_request: CostTrackingRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.track_cost(track_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-CostOps".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "cost_tracking".to_string(),
                "budget_management".to_string(),
                "optimization_recommendations".to_string(),
                "multi_tenant".to_string(),
            ],
            endpoints: vec![
                "/track".to_string(),
                "/reports/{tenant_id}".to_string(),
                "/budgets".to_string(),
                "/recommendations/{tenant_id}".to_string(),
            ],
        })
    }
}

#[async_trait]
impl CostOpsAdapter for CostOpsClient {
    async fn track_cost(&self, request: CostTrackingRequest) -> AdapterResult<CostTrackingResponse> {
        info!("Tracking cost for tenant: {}", request.tenant_id);
        self.send_request(reqwest::Method::POST, "/track", Some(&request)).await
    }

    async fn get_report(&self, tenant_id: &str, period_days: u32) -> AdapterResult<CostReport> {
        debug!("Getting cost report for tenant: {}", tenant_id);
        let path = format!("/reports/{}?period_days={}", tenant_id, period_days);
        self.send_request::<(), CostReport>(reqwest::Method::GET, &path, None).await
    }

    async fn set_budget(&self, config: BudgetConfig) -> AdapterResult<()> {
        info!("Setting budget for tenant: {}", config.tenant_id);
        self.send_request::<BudgetConfig, ()>(reqwest::Method::POST, "/budgets", Some(&config)).await
    }

    async fn get_recommendations(&self, tenant_id: &str) -> AdapterResult<Vec<CostRecommendation>> {
        debug!("Getting recommendations for tenant: {}", tenant_id);
        let path = format!("/recommendations/{}", tenant_id);
        self.send_request::<(), Vec<CostRecommendation>>(reqwest::Method::GET, &path, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = CostOpsClient::new("http://localhost:8092");
        assert_eq!(client.base_url, "http://localhost:8092");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = CostOpsClient::new("http://localhost:8092");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-CostOps");
    }
}
