//! Reranking Module for Advanced Context Management
//!
//! This module provides reranking capabilities to improve search result quality
//! by using cross-encoder models to score query-document relevance.
//!
//! # Features
//!
//! - Cross-encoder based reranking
//! - Diversity-aware reranking (MMR)
//! - Score normalization and calibration
//! - Batch processing for efficiency
//! - Async support for external model APIs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ContextError;

/// Result type for reranking operations
pub type Result<T> = std::result::Result<T, ContextError>;

/// A document to be reranked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankDocument {
    /// Unique document identifier
    pub id: String,
    /// Document content/text
    pub content: String,
    /// Original retrieval score (optional)
    pub original_score: Option<f32>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl RerankDocument {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            original_score: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_score(mut self, score: f32) -> Self {
        self.original_score = Some(score);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Result from reranking operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerResult {
    /// Document ID
    pub id: String,
    /// Document content
    pub content: String,
    /// Reranked score (relevance to query)
    pub score: f32,
    /// Original position before reranking
    pub original_rank: usize,
    /// New position after reranking
    pub new_rank: usize,
    /// Original retrieval score
    pub original_score: Option<f32>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Configuration for the reranker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerConfig {
    /// Maximum number of documents to rerank
    pub max_documents: usize,
    /// Batch size for processing
    pub batch_size: usize,
    /// Whether to normalize scores to [0, 1]
    pub normalize_scores: bool,
    /// Minimum score threshold for results
    pub score_threshold: Option<f32>,
    /// Whether to use diversity-aware reranking (MMR)
    pub use_mmr: bool,
    /// Lambda parameter for MMR (0 = diversity, 1 = relevance)
    pub mmr_lambda: f32,
    /// Model name/identifier
    pub model_name: String,
    /// API endpoint (for external models)
    pub api_endpoint: Option<String>,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for RerankerConfig {
    fn default() -> Self {
        Self {
            max_documents: 100,
            batch_size: 32,
            normalize_scores: true,
            score_threshold: None,
            use_mmr: false,
            mmr_lambda: 0.7,
            model_name: "cross-encoder/ms-marco-MiniLM-L-6-v2".to_string(),
            api_endpoint: None,
            timeout_ms: 30000,
        }
    }
}

impl RerankerConfig {
    pub fn with_max_documents(mut self, max: usize) -> Self {
        self.max_documents = max;
        self
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    pub fn with_mmr(mut self, lambda: f32) -> Self {
        self.use_mmr = true;
        self.mmr_lambda = lambda.clamp(0.0, 1.0);
        self
    }

    pub fn with_score_threshold(mut self, threshold: f32) -> Self {
        self.score_threshold = Some(threshold);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_name = model.into();
        self
    }

    pub fn with_api_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.api_endpoint = Some(endpoint.into());
        self
    }
}

/// Provider trait for cross-encoder scoring
#[async_trait]
pub trait RerankerProvider: Send + Sync {
    /// Score a batch of query-document pairs
    /// Returns scores in the same order as documents
    async fn score_pairs(
        &self,
        query: &str,
        documents: &[&str],
    ) -> Result<Vec<f32>>;

    /// Get the model name
    fn model_name(&self) -> &str;
}

/// Mock reranker provider for testing
pub struct MockRerankerProvider {
    model_name: String,
}

impl MockRerankerProvider {
    pub fn new() -> Self {
        Self {
            model_name: "mock-cross-encoder".to_string(),
        }
    }
}

impl Default for MockRerankerProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RerankerProvider for MockRerankerProvider {
    async fn score_pairs(
        &self,
        query: &str,
        documents: &[&str],
    ) -> Result<Vec<f32>> {
        // Simple mock: score based on keyword overlap
        let query_words: std::collections::HashSet<_> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let scores: Vec<f32> = documents
            .iter()
            .map(|doc| {
                let doc_words: std::collections::HashSet<_> = doc
                    .to_lowercase()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                let overlap = query_words.intersection(&doc_words).count();
                let total = query_words.len().max(1);
                (overlap as f32 / total as f32).clamp(0.0, 1.0)
            })
            .collect();

        Ok(scores)
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Cross-encoder based reranker
pub struct CrossEncoderReranker {
    config: RerankerConfig,
    provider: Arc<dyn RerankerProvider>,
    cache: Arc<RwLock<HashMap<String, f32>>>,
}

impl CrossEncoderReranker {
    pub fn new(config: RerankerConfig, provider: Arc<dyn RerankerProvider>) -> Self {
        Self {
            config,
            provider,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with mock provider for testing
    pub fn mock() -> Self {
        Self::new(
            RerankerConfig::default(),
            Arc::new(MockRerankerProvider::new()),
        )
    }

    /// Generate cache key for query-document pair
    fn cache_key(query: &str, doc_id: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        query.hash(&mut hasher);
        doc_id.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Rerank documents for a given query
    pub async fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
    ) -> Result<Vec<RerankerResult>> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        // Limit documents
        let docs: Vec<_> = documents
            .into_iter()
            .take(self.config.max_documents)
            .collect();

        // Score documents in batches
        let mut scored_docs = Vec::with_capacity(docs.len());

        for (batch_idx, batch) in docs.chunks(self.config.batch_size).enumerate() {
            // Check cache first
            let mut uncached_indices = Vec::new();
            let mut cached_scores = vec![None; batch.len()];

            {
                let cache = self.cache.read().await;
                for (i, doc) in batch.iter().enumerate() {
                    let key = Self::cache_key(query, &doc.id);
                    if let Some(&score) = cache.get(&key) {
                        cached_scores[i] = Some(score);
                    } else {
                        uncached_indices.push(i);
                    }
                }
            }

            // Score uncached documents
            if !uncached_indices.is_empty() {
                let uncached_contents: Vec<&str> = uncached_indices
                    .iter()
                    .map(|&i| batch[i].content.as_str())
                    .collect();

                let scores = self.provider.score_pairs(query, &uncached_contents).await?;

                // Update cache
                let mut cache = self.cache.write().await;
                for (score_idx, &doc_idx) in uncached_indices.iter().enumerate() {
                    let key = Self::cache_key(query, &batch[doc_idx].id);
                    cache.insert(key, scores[score_idx]);
                    cached_scores[doc_idx] = Some(scores[score_idx]);
                }
            }

            // Collect scored documents
            for (i, doc) in batch.iter().enumerate() {
                let global_idx = batch_idx * self.config.batch_size + i;
                scored_docs.push((
                    global_idx,
                    doc.clone(),
                    cached_scores[i].unwrap_or(0.0),
                ));
            }
        }

        // Apply MMR if enabled
        let results = if self.config.use_mmr {
            self.apply_mmr(query, scored_docs).await?
        } else {
            self.standard_rerank(scored_docs)
        };

        // Normalize scores if configured
        let mut results = if self.config.normalize_scores {
            self.normalize_results(results)
        } else {
            results
        };

        // Apply score threshold
        if let Some(threshold) = self.config.score_threshold {
            results.retain(|r| r.score >= threshold);
        }

        Ok(results)
    }

    /// Standard reranking by score
    fn standard_rerank(
        &self,
        mut scored_docs: Vec<(usize, RerankDocument, f32)>,
    ) -> Vec<RerankerResult> {
        // Sort by score descending
        scored_docs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        scored_docs
            .into_iter()
            .enumerate()
            .map(|(new_rank, (original_rank, doc, score))| RerankerResult {
                id: doc.id,
                content: doc.content,
                score,
                original_rank,
                new_rank,
                original_score: doc.original_score,
                metadata: doc.metadata,
            })
            .collect()
    }

    /// Apply Maximal Marginal Relevance for diversity
    async fn apply_mmr(
        &self,
        _query: &str,
        scored_docs: Vec<(usize, RerankDocument, f32)>,
    ) -> Result<Vec<RerankerResult>> {
        let lambda = self.config.mmr_lambda;
        let mut selected: Vec<(usize, RerankDocument, f32)> = Vec::new();
        let mut remaining: Vec<_> = scored_docs;

        while !remaining.is_empty() && selected.len() < self.config.max_documents {
            let mut best_idx = 0;
            let mut best_mmr = f32::NEG_INFINITY;

            for (i, (_, doc, rel_score)) in remaining.iter().enumerate() {
                // Calculate diversity score (max similarity to already selected)
                let diversity_score = if selected.is_empty() {
                    0.0
                } else {
                    selected
                        .iter()
                        .map(|(_, sel_doc, _)| self.content_similarity(&doc.content, &sel_doc.content))
                        .fold(f32::NEG_INFINITY, f32::max)
                };

                // MMR score: lambda * relevance - (1 - lambda) * max_similarity
                let mmr_score = lambda * rel_score - (1.0 - lambda) * diversity_score;

                if mmr_score > best_mmr {
                    best_mmr = mmr_score;
                    best_idx = i;
                }
            }

            selected.push(remaining.remove(best_idx));
        }

        Ok(selected
            .into_iter()
            .enumerate()
            .map(|(new_rank, (original_rank, doc, score))| RerankerResult {
                id: doc.id,
                content: doc.content,
                score,
                original_rank,
                new_rank,
                original_score: doc.original_score,
                metadata: doc.metadata,
            })
            .collect())
    }

    /// Simple content similarity (Jaccard on words)
    fn content_similarity(&self, a: &str, b: &str) -> f32 {
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();
        let words_a: std::collections::HashSet<&str> = a_lower.split_whitespace().collect();
        let words_b: std::collections::HashSet<&str> = b_lower.split_whitespace().collect();

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Normalize scores to [0, 1] range
    fn normalize_results(&self, mut results: Vec<RerankerResult>) -> Vec<RerankerResult> {
        if results.is_empty() {
            return results;
        }

        let min_score = results
            .iter()
            .map(|r| r.score)
            .fold(f32::INFINITY, f32::min);
        let max_score = results
            .iter()
            .map(|r| r.score)
            .fold(f32::NEG_INFINITY, f32::max);

        let range = max_score - min_score;
        if range > f32::EPSILON {
            for result in &mut results {
                result.score = (result.score - min_score) / range;
            }
        } else {
            // All scores are the same
            for result in &mut results {
                result.score = 1.0;
            }
        }

        results
    }

    /// Clear the score cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        (cache.len(), cache.capacity())
    }
}

/// Main Reranker trait for polymorphism
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Rerank documents for a query
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
    ) -> Result<Vec<RerankerResult>>;

    /// Get the reranker name/type
    fn name(&self) -> &str;
}

#[async_trait]
impl Reranker for CrossEncoderReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
    ) -> Result<Vec<RerankerResult>> {
        self.rerank(query, documents).await
    }

    fn name(&self) -> &str {
        self.provider.model_name()
    }
}

/// Cohere-style reranker for external API integration
pub struct CohereReranker {
    config: RerankerConfig,
    api_key: String,
}

impl CohereReranker {
    pub fn new(api_key: impl Into<String>, config: RerankerConfig) -> Self {
        Self {
            config,
            api_key: api_key.into(),
        }
    }

    #[allow(dead_code)]
    fn api_url(&self) -> String {
        self.config
            .api_endpoint
            .clone()
            .unwrap_or_else(|| "https://api.cohere.ai/v1/rerank".to_string())
    }
}

#[async_trait]
impl Reranker for CohereReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
    ) -> Result<Vec<RerankerResult>> {
        // In a real implementation, this would call the Cohere API
        // For now, we'll use a mock implementation
        let _ = (&self.api_key, query);

        // Mock: return documents in original order with decreasing scores
        let results: Vec<RerankerResult> = documents
            .into_iter()
            .enumerate()
            .map(|(i, doc)| RerankerResult {
                id: doc.id,
                content: doc.content,
                score: 1.0 - (i as f32 * 0.1).min(0.9),
                original_rank: i,
                new_rank: i,
                original_score: doc.original_score,
                metadata: doc.metadata,
            })
            .collect();

        Ok(results)
    }

    fn name(&self) -> &str {
        &self.config.model_name
    }
}

/// Ensemble reranker that combines multiple rerankers
pub struct EnsembleReranker {
    rerankers: Vec<(Arc<dyn Reranker>, f32)>, // (reranker, weight)
}

impl EnsembleReranker {
    pub fn new() -> Self {
        Self {
            rerankers: Vec::new(),
        }
    }

    pub fn add_reranker(mut self, reranker: Arc<dyn Reranker>, weight: f32) -> Self {
        self.rerankers.push((reranker, weight));
        self
    }

    /// Combine scores from multiple rerankers using weighted average
    fn combine_scores(
        &self,
        results_per_reranker: Vec<Vec<RerankerResult>>,
    ) -> Vec<RerankerResult> {
        if results_per_reranker.is_empty() {
            return Vec::new();
        }

        // Collect all document IDs
        let mut doc_scores: HashMap<String, (RerankDocument, f32, f32)> = HashMap::new();

        for (reranker_idx, results) in results_per_reranker.into_iter().enumerate() {
            let weight = self.rerankers.get(reranker_idx).map(|(_, w)| *w).unwrap_or(1.0);

            for result in results {
                let entry = doc_scores
                    .entry(result.id.clone())
                    .or_insert((
                        RerankDocument {
                            id: result.id.clone(),
                            content: result.content.clone(),
                            original_score: result.original_score,
                            metadata: result.metadata.clone(),
                        },
                        0.0,
                        0.0,
                    ));
                entry.1 += result.score * weight;
                entry.2 += weight;
            }
        }

        // Compute weighted average and sort
        let mut combined: Vec<_> = doc_scores
            .into_iter()
            .map(|(id, (doc, score_sum, weight_sum))| {
                let avg_score = if weight_sum > 0.0 {
                    score_sum / weight_sum
                } else {
                    0.0
                };
                (id, doc, avg_score)
            })
            .collect();

        combined.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        combined
            .into_iter()
            .enumerate()
            .map(|(new_rank, (id, doc, score))| RerankerResult {
                id,
                content: doc.content,
                score,
                original_rank: 0, // Not meaningful for ensemble
                new_rank,
                original_score: doc.original_score,
                metadata: doc.metadata,
            })
            .collect()
    }
}

impl Default for EnsembleReranker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Reranker for EnsembleReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
    ) -> Result<Vec<RerankerResult>> {
        if self.rerankers.is_empty() {
            return Ok(Vec::new());
        }

        // Run all rerankers in parallel
        let mut handles = Vec::new();
        for (reranker, _) in &self.rerankers {
            let reranker = reranker.clone();
            let query = query.to_string();
            let docs = documents.clone();
            handles.push(tokio::spawn(async move {
                reranker.rerank(&query, docs).await
            }));
        }

        let mut results_per_reranker = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(results)) => results_per_reranker.push(results),
                Ok(Err(e)) => {
                    tracing::warn!("Reranker failed: {}", e);
                }
                Err(e) => {
                    tracing::warn!("Reranker task failed: {}", e);
                }
            }
        }

        Ok(self.combine_scores(results_per_reranker))
    }

    fn name(&self) -> &str {
        "ensemble"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_reranker_provider() {
        let provider = MockRerankerProvider::new();
        let scores = provider
            .score_pairs(
                "rust programming",
                &["rust is a systems programming language", "python is easy to learn"],
            )
            .await
            .unwrap();

        assert_eq!(scores.len(), 2);
        assert!(scores[0] > scores[1]); // Rust doc should score higher
    }

    #[tokio::test]
    async fn test_cross_encoder_reranker() {
        let reranker = CrossEncoderReranker::mock();

        let docs = vec![
            RerankDocument::new("doc1", "rust is a systems programming language"),
            RerankDocument::new("doc2", "python is easy to learn"),
            RerankDocument::new("doc3", "rust memory safety is guaranteed"),
        ];

        let results = reranker.rerank("rust programming", docs).await.unwrap();

        assert_eq!(results.len(), 3);
        // Results should be sorted by score
        for i in 1..results.len() {
            assert!(results[i - 1].score >= results[i].score);
        }
    }

    #[tokio::test]
    async fn test_reranker_with_threshold() {
        let config = RerankerConfig::default().with_score_threshold(0.5);
        let reranker = CrossEncoderReranker::new(config, Arc::new(MockRerankerProvider::new()));

        let docs = vec![
            RerankDocument::new("doc1", "rust programming language"),
            RerankDocument::new("doc2", "completely unrelated content about cooking"),
        ];

        let results = reranker.rerank("rust programming", docs).await.unwrap();

        // Low-scoring documents should be filtered
        for result in &results {
            assert!(result.score >= 0.5);
        }
    }

    #[tokio::test]
    async fn test_reranker_with_mmr() {
        let config = RerankerConfig::default().with_mmr(0.5);
        let reranker = CrossEncoderReranker::new(config, Arc::new(MockRerankerProvider::new()));

        let docs = vec![
            RerankDocument::new("doc1", "rust programming language"),
            RerankDocument::new("doc2", "rust programming tutorial"),
            RerankDocument::new("doc3", "python web development"),
        ];

        let results = reranker.rerank("programming", docs).await.unwrap();

        assert_eq!(results.len(), 3);
        // MMR should promote diversity
    }

    #[tokio::test]
    async fn test_reranker_caching() {
        let reranker = CrossEncoderReranker::mock();

        let docs = vec![
            RerankDocument::new("doc1", "rust programming"),
            RerankDocument::new("doc2", "python programming"),
        ];

        // First call
        let _ = reranker.rerank("programming", docs.clone()).await.unwrap();
        let (size1, _) = reranker.cache_stats().await;

        // Second call with same query - should use cache
        let _ = reranker.rerank("programming", docs).await.unwrap();
        let (size2, _) = reranker.cache_stats().await;

        assert_eq!(size1, size2); // Cache size should not change

        // Clear cache
        reranker.clear_cache().await;
        let (size3, _) = reranker.cache_stats().await;
        assert_eq!(size3, 0);
    }

    #[tokio::test]
    async fn test_score_normalization() {
        let config = RerankerConfig {
            normalize_scores: true,
            ..Default::default()
        };
        let reranker = CrossEncoderReranker::new(config, Arc::new(MockRerankerProvider::new()));

        let docs = vec![
            RerankDocument::new("doc1", "exact match query terms"),
            RerankDocument::new("doc2", "partial match"),
            RerankDocument::new("doc3", "no match whatsoever xyz"),
        ];

        let results = reranker.rerank("exact match query terms", docs).await.unwrap();

        // All scores should be in [0, 1] range
        for result in &results {
            assert!(result.score >= 0.0 && result.score <= 1.0);
        }

        // Best result should have score 1.0 after normalization
        assert!((results[0].score - 1.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_rerank_document_builder() {
        let doc = RerankDocument::new("test", "test content")
            .with_score(0.8)
            .with_metadata("source", serde_json::json!("wikipedia"));

        assert_eq!(doc.id, "test");
        assert_eq!(doc.original_score, Some(0.8));
        assert!(doc.metadata.contains_key("source"));
    }

    #[tokio::test]
    async fn test_empty_documents() {
        let reranker = CrossEncoderReranker::mock();
        let results = reranker.rerank("query", vec![]).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_ensemble_reranker() {
        let reranker1 = Arc::new(CrossEncoderReranker::mock()) as Arc<dyn Reranker>;
        let reranker2 = Arc::new(CrossEncoderReranker::mock()) as Arc<dyn Reranker>;

        let ensemble = EnsembleReranker::new()
            .add_reranker(reranker1, 1.0)
            .add_reranker(reranker2, 0.5);

        let docs = vec![
            RerankDocument::new("doc1", "rust programming"),
            RerankDocument::new("doc2", "python programming"),
        ];

        let results = ensemble.rerank("programming", docs).await.unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_reranker_config_builder() {
        let config = RerankerConfig::default()
            .with_max_documents(50)
            .with_batch_size(16)
            .with_mmr(0.6)
            .with_score_threshold(0.3)
            .with_model("custom-model");

        assert_eq!(config.max_documents, 50);
        assert_eq!(config.batch_size, 16);
        assert!(config.use_mmr);
        assert!((config.mmr_lambda - 0.6).abs() < f32::EPSILON);
        assert_eq!(config.score_threshold, Some(0.3));
        assert_eq!(config.model_name, "custom-model");
    }

    #[tokio::test]
    async fn test_cohere_reranker() {
        let config = RerankerConfig::default().with_model("rerank-english-v2.0");
        let reranker = CohereReranker::new("test-api-key", config);

        let docs = vec![
            RerankDocument::new("doc1", "test content"),
        ];

        let results = reranker.rerank("query", docs).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(reranker.name(), "rerank-english-v2.0");
    }
}
