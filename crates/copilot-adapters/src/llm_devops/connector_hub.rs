//! LLM-Connector-Hub Adapter
//!
//! Thin adapter for consuming LLM-Connector-Hub service.
//! Provides unified access to multiple LLM providers.

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

/// Unified LLM request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedLLMRequest {
    pub provider: Option<String>,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub parameters: LLMParameters,
    pub routing_preference: Option<RoutingPreference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMParameters {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingPreference {
    LowestCost,
    LowestLatency,
    HighestQuality,
    Specific { provider: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedLLMResponse {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub content: String,
    pub usage: TokenUsage,
    pub latency_ms: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_id: String,
    pub name: String,
    pub api_endpoint: String,
    pub auth_type: AuthType,
    pub enabled: bool,
    pub rate_limits: RateLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    ApiKey,
    OAuth2,
    IAM,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
}

/// Trait defining ConnectorHub adapter operations
#[async_trait]
pub trait ConnectorHubAdapter: Send + Sync {
    /// Send a unified LLM request
    async fn complete(&self, request: UnifiedLLMRequest) -> AdapterResult<UnifiedLLMResponse>;

    /// List available providers
    async fn list_providers(&self) -> AdapterResult<Vec<ProviderConfig>>;

    /// Get provider status
    async fn get_provider_status(&self, provider_id: &str) -> AdapterResult<ProviderStatus>;

    /// Configure a provider
    async fn configure_provider(&self, config: ProviderConfig) -> AdapterResult<ProviderConfig>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider_id: String,
    pub healthy: bool,
    pub current_load: f32,
    pub avg_latency_ms: u64,
    pub error_rate: f32,
    pub available_models: Vec<String>,
}

/// HTTP client for LLM-Connector-Hub service
pub struct ConnectorHubClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl ConnectorHubClient {
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
impl ModuleAdapter for ConnectorHubClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Connector-Hub is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let llm_request: UnifiedLLMRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.complete(llm_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Connector-Hub".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "unified_api".to_string(),
                "multi_provider".to_string(),
                "smart_routing".to_string(),
                "failover".to_string(),
            ],
            endpoints: vec![
                "/complete".to_string(),
                "/providers".to_string(),
                "/providers/{id}/status".to_string(),
            ],
        })
    }
}

#[async_trait]
impl ConnectorHubAdapter for ConnectorHubClient {
    async fn complete(&self, request: UnifiedLLMRequest) -> AdapterResult<UnifiedLLMResponse> {
        info!("Sending completion request for model: {}", request.model);
        self.send_request(reqwest::Method::POST, "/complete", Some(&request)).await
    }

    async fn list_providers(&self) -> AdapterResult<Vec<ProviderConfig>> {
        debug!("Listing providers");
        self.send_request::<(), Vec<ProviderConfig>>(reqwest::Method::GET, "/providers", None).await
    }

    async fn get_provider_status(&self, provider_id: &str) -> AdapterResult<ProviderStatus> {
        debug!("Getting provider status: {}", provider_id);
        let path = format!("/providers/{}/status", provider_id);
        self.send_request::<(), ProviderStatus>(reqwest::Method::GET, &path, None).await
    }

    async fn configure_provider(&self, config: ProviderConfig) -> AdapterResult<ProviderConfig> {
        info!("Configuring provider: {}", config.provider_id);
        self.send_request(reqwest::Method::PUT, "/providers", Some(&config)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = ConnectorHubClient::new("http://localhost:8098");
        assert_eq!(client.base_url, "http://localhost:8098");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = ConnectorHubClient::new("http://localhost:8098");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Connector-Hub");
    }
}
