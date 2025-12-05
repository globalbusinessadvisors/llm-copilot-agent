//! LLM-Data-Vault Adapter
//!
//! Thin adapter for consuming LLM-Data-Vault service.
//! Provides secure data storage, encryption, and anonymization.

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

/// Request to store data securely
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreDataRequest {
    pub data_id: Option<String>,
    pub data: serde_json::Value,
    pub classification: DataClassification,
    pub encryption_options: EncryptionOptions,
    pub retention_policy: Option<RetentionPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionOptions {
    pub algorithm: EncryptionAlgorithm,
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    AES256GCM,
    ChaCha20Poly1305,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub retention_days: u32,
    pub auto_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreDataResponse {
    pub data_id: String,
    pub version: u32,
    pub encrypted: bool,
    pub stored_at: chrono::DateTime<chrono::Utc>,
}

/// Request to anonymize data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizeRequest {
    pub data: serde_json::Value,
    pub anonymization_rules: Vec<AnonymizationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationRule {
    pub field_path: String,
    pub technique: AnonymizationTechnique,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnonymizationTechnique {
    Redact,
    Hash,
    Tokenize,
    Generalize { level: u32 },
    Pseudonymize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizeResponse {
    pub anonymized_data: serde_json::Value,
    pub applied_rules: Vec<String>,
    pub token_mappings: Option<HashMap<String, String>>,
}

/// Trait defining DataVault adapter operations
#[async_trait]
pub trait DataVaultAdapter: Send + Sync {
    /// Store data securely
    async fn store(&self, request: StoreDataRequest) -> AdapterResult<StoreDataResponse>;

    /// Retrieve stored data
    async fn retrieve(&self, data_id: &str) -> AdapterResult<serde_json::Value>;

    /// Anonymize data
    async fn anonymize(&self, request: AnonymizeRequest) -> AdapterResult<AnonymizeResponse>;

    /// Delete data
    async fn delete(&self, data_id: &str) -> AdapterResult<()>;

    /// Get audit log for data
    async fn get_audit_log(&self, data_id: &str) -> AdapterResult<Vec<AuditEntry>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub action: String,
    pub actor: String,
    pub details: Option<serde_json::Value>,
}

/// HTTP client for LLM-Data-Vault service
pub struct DataVaultClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl DataVaultClient {
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
impl ModuleAdapter for DataVaultClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Data-Vault is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let store_request: StoreDataRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.store(store_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Data-Vault".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "secure_storage".to_string(),
                "encryption".to_string(),
                "anonymization".to_string(),
                "audit_logging".to_string(),
            ],
            endpoints: vec![
                "/data".to_string(),
                "/data/{id}".to_string(),
                "/anonymize".to_string(),
                "/audit/{id}".to_string(),
            ],
        })
    }
}

#[async_trait]
impl DataVaultAdapter for DataVaultClient {
    async fn store(&self, request: StoreDataRequest) -> AdapterResult<StoreDataResponse> {
        info!("Storing data with classification: {:?}", request.classification);
        self.send_request(reqwest::Method::POST, "/data", Some(&request)).await
    }

    async fn retrieve(&self, data_id: &str) -> AdapterResult<serde_json::Value> {
        debug!("Retrieving data: {}", data_id);
        let path = format!("/data/{}", data_id);
        self.send_request::<(), serde_json::Value>(reqwest::Method::GET, &path, None).await
    }

    async fn anonymize(&self, request: AnonymizeRequest) -> AdapterResult<AnonymizeResponse> {
        debug!("Anonymizing data with {} rules", request.anonymization_rules.len());
        self.send_request(reqwest::Method::POST, "/anonymize", Some(&request)).await
    }

    async fn delete(&self, data_id: &str) -> AdapterResult<()> {
        info!("Deleting data: {}", data_id);
        let path = format!("/data/{}", data_id);
        self.send_request::<(), ()>(reqwest::Method::DELETE, &path, None).await
    }

    async fn get_audit_log(&self, data_id: &str) -> AdapterResult<Vec<AuditEntry>> {
        debug!("Getting audit log for: {}", data_id);
        let path = format!("/audit/{}", data_id);
        self.send_request::<(), Vec<AuditEntry>>(reqwest::Method::GET, &path, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = DataVaultClient::new("http://localhost:8099");
        assert_eq!(client.base_url, "http://localhost:8099");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = DataVaultClient::new("http://localhost:8099");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Data-Vault");
    }
}
