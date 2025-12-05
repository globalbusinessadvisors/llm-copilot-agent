//! LLM-Registry Adapter
//!
//! Thin adapter for consuming LLM-Registry service.
//! Provides asset, model, and prompt registry management.

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

/// Model registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistration {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub version: String,
    pub description: Option<String>,
    pub capabilities: Vec<String>,
    pub pricing: ModelPricing,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_cost_per_1k_tokens: f64,
    pub output_cost_per_1k_tokens: f64,
    pub currency: String,
}

/// Prompt template registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub version: String,
    pub template: String,
    pub variables: Vec<TemplateVariable>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub var_type: VariableType,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

/// Asset registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub asset_type: AssetType,
    pub name: String,
    pub version: String,
    pub status: AssetStatus,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Model,
    Prompt,
    Dataset,
    Embedding,
    FineTune,
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetStatus {
    Draft,
    Published,
    Deprecated,
    Archived,
}

/// Trait defining Registry adapter operations
#[async_trait]
pub trait RegistryAdapter: Send + Sync {
    /// Register a model
    async fn register_model(&self, model: ModelRegistration) -> AdapterResult<ModelRegistration>;

    /// List models
    async fn list_models(&self) -> AdapterResult<Vec<ModelRegistration>>;

    /// Register a prompt template
    async fn register_prompt(&self, prompt: PromptTemplate) -> AdapterResult<PromptTemplate>;

    /// List prompt templates
    async fn list_prompts(&self, tags: Option<Vec<String>>) -> AdapterResult<Vec<PromptTemplate>>;

    /// Get asset by ID
    async fn get_asset(&self, asset_id: &str) -> AdapterResult<Asset>;

    /// Search assets
    async fn search_assets(&self, query: &str, asset_type: Option<AssetType>) -> AdapterResult<Vec<Asset>>;
}

/// HTTP client for LLM-Registry service
pub struct RegistryClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl RegistryClient {
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
impl ModuleAdapter for RegistryClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Registry is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let query: String = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.search_assets(&query, None).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Registry".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "model_registry".to_string(),
                "prompt_registry".to_string(),
                "asset_management".to_string(),
                "versioning".to_string(),
            ],
            endpoints: vec![
                "/models".to_string(),
                "/prompts".to_string(),
                "/assets/{id}".to_string(),
                "/search".to_string(),
            ],
        })
    }
}

#[async_trait]
impl RegistryAdapter for RegistryClient {
    async fn register_model(&self, model: ModelRegistration) -> AdapterResult<ModelRegistration> {
        info!("Registering model: {}", model.name);
        self.send_request(reqwest::Method::POST, "/models", Some(&model)).await
    }

    async fn list_models(&self) -> AdapterResult<Vec<ModelRegistration>> {
        debug!("Listing models");
        self.send_request::<(), Vec<ModelRegistration>>(reqwest::Method::GET, "/models", None).await
    }

    async fn register_prompt(&self, prompt: PromptTemplate) -> AdapterResult<PromptTemplate> {
        info!("Registering prompt: {}", prompt.name);
        self.send_request(reqwest::Method::POST, "/prompts", Some(&prompt)).await
    }

    async fn list_prompts(&self, tags: Option<Vec<String>>) -> AdapterResult<Vec<PromptTemplate>> {
        debug!("Listing prompts");
        let path = match tags {
            Some(t) => format!("/prompts?tags={}", t.join(",")),
            None => "/prompts".to_string(),
        };
        self.send_request::<(), Vec<PromptTemplate>>(reqwest::Method::GET, &path, None).await
    }

    async fn get_asset(&self, asset_id: &str) -> AdapterResult<Asset> {
        debug!("Getting asset: {}", asset_id);
        let path = format!("/assets/{}", asset_id);
        self.send_request::<(), Asset>(reqwest::Method::GET, &path, None).await
    }

    async fn search_assets(&self, query: &str, asset_type: Option<AssetType>) -> AdapterResult<Vec<Asset>> {
        debug!("Searching assets: {}", query);
        let path = match asset_type {
            Some(t) => format!("/search?q={}&type={:?}", query, t),
            None => format!("/search?q={}", query),
        };
        self.send_request::<(), Vec<Asset>>(reqwest::Method::GET, &path, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = RegistryClient::new("http://localhost:8104");
        assert_eq!(client.base_url, "http://localhost:8104");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = RegistryClient::new("http://localhost:8104");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Registry");
    }
}
