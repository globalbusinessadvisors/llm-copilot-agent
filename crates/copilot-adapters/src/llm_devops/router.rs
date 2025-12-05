//! Router Adapter (Layer 2 Module)
//!
//! Thin adapter for consuming the Layer 2 routing service.
//! Provides intelligent request routing and load balancing.

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

/// Request to route to an appropriate backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRequest {
    pub request_type: String,
    pub payload: serde_json::Value,
    pub routing_hints: Option<RoutingHints>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingHints {
    pub preferred_provider: Option<String>,
    pub latency_priority: bool,
    pub cost_priority: bool,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResponse {
    pub route_id: String,
    pub selected_backend: String,
    pub latency_estimate_ms: u64,
    pub cost_estimate: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStatus {
    pub backend_id: String,
    pub name: String,
    pub healthy: bool,
    pub current_load: f32,
    pub avg_latency_ms: u64,
}

/// Trait defining Router adapter operations
#[async_trait]
pub trait RouterAdapter: Send + Sync {
    /// Route a request to the best available backend
    async fn route(&self, request: RouteRequest) -> AdapterResult<RouteResponse>;

    /// Get status of all backends
    async fn get_backends(&self) -> AdapterResult<Vec<BackendStatus>>;

    /// Update routing configuration
    async fn update_config(&self, config: RouterConfig) -> AdapterResult<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub load_balancing_strategy: LoadBalancingStrategy,
    pub health_check_interval_seconds: u64,
    pub circuit_breaker_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRandom,
    LatencyBased,
    CostOptimized,
}

/// HTTP client for Router service
pub struct RouterClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl RouterClient {
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
impl ModuleAdapter for RouterClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("Router is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let route_request: RouteRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.route(route_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "Router".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "intelligent_routing".to_string(),
                "load_balancing".to_string(),
                "latency_optimization".to_string(),
                "cost_optimization".to_string(),
            ],
            endpoints: vec![
                "/route".to_string(),
                "/backends".to_string(),
                "/config".to_string(),
            ],
        })
    }
}

#[async_trait]
impl RouterAdapter for RouterClient {
    async fn route(&self, request: RouteRequest) -> AdapterResult<RouteResponse> {
        info!("Routing request type: {}", request.request_type);
        self.send_request(reqwest::Method::POST, "/route", Some(&request)).await
    }

    async fn get_backends(&self) -> AdapterResult<Vec<BackendStatus>> {
        debug!("Getting backend status");
        self.send_request::<(), Vec<BackendStatus>>(reqwest::Method::GET, "/backends", None).await
    }

    async fn update_config(&self, config: RouterConfig) -> AdapterResult<()> {
        info!("Updating router configuration");
        self.send_request::<RouterConfig, ()>(reqwest::Method::PUT, "/config", Some(&config)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = RouterClient::new("http://localhost:8091");
        assert_eq!(client.base_url, "http://localhost:8091");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = RouterClient::new("http://localhost:8091");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "Router");
    }
}
