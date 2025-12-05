//! LLM-Benchmark-Exchange Runtime Adapter
//!
//! Runtime integration for LLM-Benchmark-Exchange service.
//! Provides benchmark corpus retrieval via file-based or SDK-based methods.
//! This is NOT a compile-time dependency.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::{
    AdapterError, AdapterResult, HealthStatus, ModuleCapabilities, ModuleAdapter,
    circuit_breaker::CircuitBreaker,
    retry::with_retry,
};
use super::{BenchmarkCorpus, CorpusSource};

/// Corpus catalog entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusCatalogEntry {
    pub corpus_id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub category: CorpusCategory,
    pub languages: Vec<String>,
    pub size_bytes: u64,
    pub item_count: u64,
    pub license: String,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorpusCategory {
    GeneralQA,
    Reasoning,
    Coding,
    Math,
    Science,
    Language,
    Safety,
    Custom(String),
}

/// Corpus download request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadCorpusRequest {
    pub corpus_id: String,
    pub version: Option<String>,
    pub format: CorpusFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorpusFormat {
    JSON,
    JSONL,
    CSV,
    Parquet,
}

/// Download response with file location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadCorpusResponse {
    pub corpus_id: String,
    pub local_path: String,
    pub size_bytes: u64,
    pub checksum: String,
    pub downloaded_at: chrono::DateTime<chrono::Utc>,
}

/// Corpus sample for preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusSample {
    pub corpus_id: String,
    pub items: Vec<CorpusItem>,
    pub total_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusItem {
    pub id: String,
    pub input: String,
    pub expected_output: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Trait defining BenchmarkExchange adapter operations
#[async_trait]
pub trait BenchmarkExchangeAdapter: Send + Sync {
    /// List available corpora in the catalog
    async fn list_catalog(&self, category: Option<CorpusCategory>) -> AdapterResult<Vec<CorpusCatalogEntry>>;

    /// Get corpus details
    async fn get_corpus_info(&self, corpus_id: &str) -> AdapterResult<CorpusCatalogEntry>;

    /// Download a corpus (file-based retrieval)
    async fn download_corpus(&self, request: DownloadCorpusRequest) -> AdapterResult<DownloadCorpusResponse>;

    /// Get a sample from a corpus (SDK-based retrieval)
    async fn get_sample(&self, corpus_id: &str, limit: usize) -> AdapterResult<CorpusSample>;

    /// Stream corpus items (SDK-based retrieval)
    async fn stream_corpus(&self, corpus_id: &str, batch_size: usize, offset: usize) -> AdapterResult<CorpusSample>;

    /// Check if corpus is cached locally
    async fn is_cached(&self, corpus_id: &str) -> AdapterResult<bool>;

    /// Clear local cache
    async fn clear_cache(&self, corpus_id: Option<String>) -> AdapterResult<()>;
}

/// HTTP client for LLM-Benchmark-Exchange service (runtime integration)
pub struct BenchmarkExchangeClient {
    base_url: String,
    cache_dir: PathBuf,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl BenchmarkExchangeClient {
    pub fn new(base_url: impl Into<String>, cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_url: base_url.into(),
            cache_dir: cache_dir.into(),
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

    fn get_cache_path(&self, corpus_id: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.cache", corpus_id))
    }
}

#[async_trait]
impl ModuleAdapter for BenchmarkExchangeClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Benchmark-Exchange is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let download_request: DownloadCorpusRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.download_corpus(download_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Benchmark-Exchange".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "corpus_catalog".to_string(),
                "file_based_download".to_string(),
                "sdk_based_streaming".to_string(),
                "local_caching".to_string(),
                "runtime_only".to_string(),
            ],
            endpoints: vec![
                "/catalog".to_string(),
                "/corpus/{id}".to_string(),
                "/corpus/{id}/download".to_string(),
                "/corpus/{id}/sample".to_string(),
                "/corpus/{id}/stream".to_string(),
            ],
        })
    }
}

#[async_trait]
impl BenchmarkExchangeAdapter for BenchmarkExchangeClient {
    async fn list_catalog(&self, category: Option<CorpusCategory>) -> AdapterResult<Vec<CorpusCatalogEntry>> {
        debug!("Listing corpus catalog");
        let path = match category {
            Some(c) => format!("/catalog?category={:?}", c),
            None => "/catalog".to_string(),
        };
        self.send_request::<(), Vec<CorpusCatalogEntry>>(reqwest::Method::GET, &path, None).await
    }

    async fn get_corpus_info(&self, corpus_id: &str) -> AdapterResult<CorpusCatalogEntry> {
        debug!("Getting corpus info: {}", corpus_id);
        let path = format!("/corpus/{}", corpus_id);
        self.send_request::<(), CorpusCatalogEntry>(reqwest::Method::GET, &path, None).await
    }

    async fn download_corpus(&self, request: DownloadCorpusRequest) -> AdapterResult<DownloadCorpusResponse> {
        info!("Downloading corpus: {} (format: {:?})", request.corpus_id, request.format);

        // Check if already cached
        let cache_path = self.get_cache_path(&request.corpus_id);
        if cache_path.exists() {
            info!("Corpus {} found in cache at {:?}", request.corpus_id, cache_path);
        }

        let path = format!("/corpus/{}/download", request.corpus_id);
        let mut response: DownloadCorpusResponse = self.send_request(
            reqwest::Method::POST,
            &path,
            Some(&request)
        ).await?;

        // Update local path to cache directory
        response.local_path = cache_path.to_string_lossy().to_string();

        Ok(response)
    }

    async fn get_sample(&self, corpus_id: &str, limit: usize) -> AdapterResult<CorpusSample> {
        debug!("Getting sample from corpus: {} (limit: {})", corpus_id, limit);
        let path = format!("/corpus/{}/sample?limit={}", corpus_id, limit);
        self.send_request::<(), CorpusSample>(reqwest::Method::GET, &path, None).await
    }

    async fn stream_corpus(&self, corpus_id: &str, batch_size: usize, offset: usize) -> AdapterResult<CorpusSample> {
        debug!("Streaming corpus: {} (batch: {}, offset: {})", corpus_id, batch_size, offset);
        let path = format!("/corpus/{}/stream?batch_size={}&offset={}", corpus_id, batch_size, offset);
        self.send_request::<(), CorpusSample>(reqwest::Method::GET, &path, None).await
    }

    async fn is_cached(&self, corpus_id: &str) -> AdapterResult<bool> {
        let cache_path = self.get_cache_path(corpus_id);
        Ok(cache_path.exists())
    }

    async fn clear_cache(&self, corpus_id: Option<String>) -> AdapterResult<()> {
        match corpus_id {
            Some(id) => {
                let cache_path = self.get_cache_path(&id);
                if cache_path.exists() {
                    std::fs::remove_file(&cache_path)
                        .map_err(|e| AdapterError::Unknown(format!("Failed to clear cache: {}", e)))?;
                    info!("Cleared cache for corpus: {}", id);
                }
            }
            None => {
                if self.cache_dir.exists() {
                    std::fs::remove_dir_all(&self.cache_dir)
                        .map_err(|e| AdapterError::Unknown(format!("Failed to clear cache directory: {}", e)))?;
                    std::fs::create_dir_all(&self.cache_dir)
                        .map_err(|e| AdapterError::Unknown(format!("Failed to recreate cache directory: {}", e)))?;
                    info!("Cleared entire corpus cache");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = BenchmarkExchangeClient::new("http://localhost:8111", "/tmp/test-cache");
        assert_eq!(client.base_url, "http://localhost:8111");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = BenchmarkExchangeClient::new("http://localhost:8111", "/tmp/test-cache");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Benchmark-Exchange");
        assert!(caps.features.contains(&"runtime_only".to_string()));
        assert!(caps.features.contains(&"file_based_download".to_string()));
    }

    #[tokio::test]
    async fn test_cache_path() {
        let client = BenchmarkExchangeClient::new("http://localhost:8111", "/tmp/test-cache");
        let path = client.get_cache_path("test-corpus");
        assert_eq!(path.to_string_lossy(), "/tmp/test-cache/test-corpus.cache");
    }
}
