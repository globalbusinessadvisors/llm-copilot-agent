//! LLM-Memory-Graph Adapter
//!
//! Thin adapter for consuming LLM-Memory-Graph service.
//! Provides graph-based context and memory tracking.

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

/// Memory node representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: String,
    pub node_type: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Memory edge representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEdge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    pub weight: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Request to store a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMemoryRequest {
    pub session_id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub related_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    ShortTerm,
    LongTerm,
    Episodic,
    Semantic,
    Procedural,
}

/// Query for memory retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub session_id: String,
    pub query_text: Option<String>,
    pub query_embedding: Option<Vec<f32>>,
    pub memory_types: Option<Vec<MemoryType>>,
    pub max_results: usize,
    pub min_relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResponse {
    pub nodes: Vec<MemoryNode>,
    pub edges: Vec<MemoryEdge>,
    pub relevance_scores: HashMap<String, f32>,
}

/// Trait defining MemoryGraph adapter operations
#[async_trait]
pub trait MemoryGraphAdapter: Send + Sync {
    /// Store a new memory
    async fn store_memory(&self, request: StoreMemoryRequest) -> AdapterResult<MemoryNode>;

    /// Query memories
    async fn query_memories(&self, query: MemoryQuery) -> AdapterResult<MemoryQueryResponse>;

    /// Create edge between nodes
    async fn create_edge(&self, edge: MemoryEdge) -> AdapterResult<MemoryEdge>;

    /// Get memory graph stats
    async fn get_stats(&self, session_id: &str) -> AdapterResult<MemoryGraphStats>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGraphStats {
    pub total_nodes: u64,
    pub total_edges: u64,
    pub nodes_by_type: HashMap<String, u64>,
    pub avg_connections_per_node: f32,
}

/// HTTP client for LLM-Memory-Graph service
pub struct MemoryGraphClient {
    base_url: String,
    client: Client,
    circuit_breaker: CircuitBreaker,
}

impl MemoryGraphClient {
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
impl ModuleAdapter for MemoryGraphClient {
    async fn health_check(&self) -> AdapterResult<HealthStatus> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus::healthy("LLM-Memory-Graph is operational"))
            }
            Ok(response) => {
                Ok(HealthStatus::unhealthy(format!("Status: {}", response.status())))
            }
            Err(e) => Ok(HealthStatus::unhealthy(format!("Unreachable: {}", e))),
        }
    }

    async fn execute(&self, request: serde_json::Value) -> AdapterResult<serde_json::Value> {
        let store_request: StoreMemoryRequest = serde_json::from_value(request)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))?;
        let response = self.store_memory(store_request).await?;
        serde_json::to_value(response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    async fn get_capabilities(&self) -> AdapterResult<ModuleCapabilities> {
        Ok(ModuleCapabilities {
            name: "LLM-Memory-Graph".to_string(),
            version: "0.1.0".to_string(),
            features: vec![
                "graph_storage".to_string(),
                "semantic_search".to_string(),
                "memory_types".to_string(),
                "relationship_tracking".to_string(),
            ],
            endpoints: vec![
                "/memories".to_string(),
                "/memories/query".to_string(),
                "/edges".to_string(),
                "/stats/{session_id}".to_string(),
            ],
        })
    }
}

#[async_trait]
impl MemoryGraphAdapter for MemoryGraphClient {
    async fn store_memory(&self, request: StoreMemoryRequest) -> AdapterResult<MemoryNode> {
        info!("Storing memory for session: {}", request.session_id);
        self.send_request(reqwest::Method::POST, "/memories", Some(&request)).await
    }

    async fn query_memories(&self, query: MemoryQuery) -> AdapterResult<MemoryQueryResponse> {
        debug!("Querying memories for session: {}", query.session_id);
        self.send_request(reqwest::Method::POST, "/memories/query", Some(&query)).await
    }

    async fn create_edge(&self, edge: MemoryEdge) -> AdapterResult<MemoryEdge> {
        debug!("Creating edge: {} -> {}", edge.source_id, edge.target_id);
        self.send_request(reqwest::Method::POST, "/edges", Some(&edge)).await
    }

    async fn get_stats(&self, session_id: &str) -> AdapterResult<MemoryGraphStats> {
        debug!("Getting memory graph stats for session: {}", session_id);
        let path = format!("/stats/{}", session_id);
        self.send_request::<(), MemoryGraphStats>(reqwest::Method::GET, &path, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = MemoryGraphClient::new("http://localhost:8093");
        assert_eq!(client.base_url, "http://localhost:8093");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let client = MemoryGraphClient::new("http://localhost:8093");
        let caps = client.get_capabilities().await.unwrap();
        assert_eq!(caps.name, "LLM-Memory-Graph");
    }
}
