//! Hybrid search combining vector and keyword search
//!
//! Provides advanced search capabilities that combine dense (vector) and sparse
//! (keyword/BM25) retrieval methods for improved accuracy.

use crate::{ContextError, MemoryItem, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

/// Embedding vector type
pub type Embedding = Vec<f32>;

/// Embedding provider trait
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for text
    async fn embed(&self, text: &str) -> Result<Embedding>;

    /// Generate embeddings for multiple texts (batch)
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>>;

    /// Get embedding dimension
    fn dimension(&self) -> usize;
}

/// Simple mock embedding provider for testing
pub struct MockEmbeddingProvider {
    dimension: usize,
}

impl MockEmbeddingProvider {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Embedding> {
        // Generate deterministic "embedding" based on text hash
        let hash = text.bytes().fold(0u64, |acc, b| {
            acc.wrapping_mul(31).wrapping_add(b as u64)
        });

        let mut embedding = vec![0.0f32; self.dimension];
        for (i, val) in embedding.iter_mut().enumerate() {
            *val = ((hash.wrapping_add(i as u64) % 1000) as f32 / 1000.0) - 0.5;
        }

        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Vector similarity metrics
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SimilarityMetric {
    /// Cosine similarity (normalized dot product)
    #[default]
    Cosine,
    /// Euclidean distance (L2)
    Euclidean,
    /// Dot product
    DotProduct,
}

impl SimilarityMetric {
    /// Calculate similarity between two vectors
    pub fn calculate(&self, a: &Embedding, b: &Embedding) -> f32 {
        match self {
            SimilarityMetric::Cosine => {
                let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
                let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
                let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm_a > 0.0 && norm_b > 0.0 {
                    dot / (norm_a * norm_b)
                } else {
                    0.0
                }
            }
            SimilarityMetric::Euclidean => {
                let dist: f32 = a
                    .iter()
                    .zip(b.iter())
                    .map(|(x, y)| (x - y).powi(2))
                    .sum::<f32>()
                    .sqrt();
                // Convert distance to similarity (1 / (1 + dist))
                1.0 / (1.0 + dist)
            }
            SimilarityMetric::DotProduct => {
                a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
            }
        }
    }
}

/// BM25 sparse retrieval parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BM25Config {
    /// Term frequency saturation parameter (default: 1.2)
    pub k1: f32,
    /// Length normalization parameter (default: 0.75)
    pub b: f32,
    /// Average document length (computed automatically)
    #[serde(skip)]
    pub avg_doc_length: f32,
    /// Total documents (computed automatically)
    #[serde(skip)]
    pub total_docs: usize,
}

impl Default for BM25Config {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            avg_doc_length: 0.0,
            total_docs: 0,
        }
    }
}

/// BM25 sparse scorer
pub struct BM25Scorer {
    config: BM25Config,
    /// Inverted index: term -> [(doc_id, term_freq)]
    inverted_index: HashMap<String, Vec<(String, usize)>>,
    /// Document lengths
    doc_lengths: HashMap<String, usize>,
    /// IDF cache
    idf_cache: HashMap<String, f32>,
}

impl BM25Scorer {
    pub fn new(config: BM25Config) -> Self {
        Self {
            config,
            inverted_index: HashMap::new(),
            doc_lengths: HashMap::new(),
            idf_cache: HashMap::new(),
        }
    }

    /// Index a document
    pub fn index(&mut self, doc_id: &str, content: &str) {
        let tokens = tokenize(content);
        let doc_length = tokens.len();

        // Count term frequencies
        let mut term_freqs: HashMap<String, usize> = HashMap::new();
        for token in tokens {
            *term_freqs.entry(token).or_default() += 1;
        }

        // Update inverted index
        for (term, freq) in term_freqs {
            self.inverted_index
                .entry(term)
                .or_default()
                .push((doc_id.to_string(), freq));
        }

        self.doc_lengths.insert(doc_id.to_string(), doc_length);

        // Update statistics
        self.config.total_docs += 1;
        let total_length: usize = self.doc_lengths.values().sum();
        self.config.avg_doc_length = total_length as f32 / self.config.total_docs as f32;

        // Invalidate IDF cache
        self.idf_cache.clear();
    }

    /// Remove a document from the index
    pub fn remove(&mut self, doc_id: &str) {
        self.doc_lengths.remove(doc_id);

        for postings in self.inverted_index.values_mut() {
            postings.retain(|(id, _)| id != doc_id);
        }

        // Update statistics
        if self.config.total_docs > 0 {
            self.config.total_docs -= 1;
            if self.config.total_docs > 0 {
                let total_length: usize = self.doc_lengths.values().sum();
                self.config.avg_doc_length = total_length as f32 / self.config.total_docs as f32;
            }
        }

        self.idf_cache.clear();
    }

    /// Calculate IDF for a term
    fn idf(&mut self, term: &str) -> f32 {
        if let Some(&cached) = self.idf_cache.get(term) {
            return cached;
        }

        let doc_freq = self.inverted_index.get(term).map(|v| v.len()).unwrap_or(0);
        let n = self.config.total_docs as f32;
        let df = doc_freq as f32;

        let idf = if doc_freq > 0 {
            ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
        } else {
            0.0
        };

        self.idf_cache.insert(term.to_string(), idf);
        idf
    }

    /// Score a query against all documents
    pub fn score(&mut self, query: &str) -> Vec<(String, f32)> {
        let query_tokens = tokenize(query);
        let mut scores: HashMap<String, f32> = HashMap::new();

        for term in query_tokens {
            let idf = self.idf(&term);

            if let Some(postings) = self.inverted_index.get(&term) {
                for (doc_id, tf) in postings {
                    let doc_length = *self.doc_lengths.get(doc_id).unwrap_or(&1) as f32;
                    let tf = *tf as f32;

                    // BM25 scoring formula
                    let numerator = tf * (self.config.k1 + 1.0);
                    let denominator = tf
                        + self.config.k1
                            * (1.0 - self.config.b
                                + self.config.b * (doc_length / self.config.avg_doc_length));

                    let score = idf * (numerator / denominator);
                    *scores.entry(doc_id.clone()).or_default() += score;
                }
            }
        }

        let mut results: Vec<_> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

/// Simple tokenizer
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 2)
        .map(String::from)
        .collect()
}

/// Hybrid search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchConfig {
    /// Weight for vector (dense) search (0.0 - 1.0)
    pub vector_weight: f32,
    /// Weight for keyword (sparse) search (0.0 - 1.0)
    pub keyword_weight: f32,
    /// Similarity metric for vectors
    pub similarity_metric: SimilarityMetric,
    /// BM25 parameters
    pub bm25_config: BM25Config,
    /// Number of candidates from each retriever before fusion
    pub candidates_per_retriever: usize,
    /// Enable Reciprocal Rank Fusion
    pub use_rrf: bool,
    /// RRF constant k (default: 60)
    pub rrf_k: usize,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            vector_weight: 0.7,
            keyword_weight: 0.3,
            similarity_metric: SimilarityMetric::Cosine,
            bm25_config: BM25Config::default(),
            candidates_per_retriever: 100,
            use_rrf: true,
            rrf_k: 60,
        }
    }
}

/// Hybrid search result
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    /// Document ID
    pub doc_id: String,
    /// Combined score
    pub score: f32,
    /// Vector similarity score
    pub vector_score: Option<f32>,
    /// Keyword (BM25) score
    pub keyword_score: Option<f32>,
}

/// Hybrid search engine
pub struct HybridSearchEngine {
    config: HybridSearchConfig,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    bm25_scorer: BM25Scorer,
    /// Document embeddings cache
    doc_embeddings: HashMap<String, Embedding>,
    /// Document contents for retrieval
    doc_contents: HashMap<String, String>,
}

impl HybridSearchEngine {
    pub fn new(config: HybridSearchConfig, embedding_provider: Arc<dyn EmbeddingProvider>) -> Self {
        let bm25_scorer = BM25Scorer::new(config.bm25_config.clone());
        Self {
            config,
            embedding_provider,
            bm25_scorer,
            doc_embeddings: HashMap::new(),
            doc_contents: HashMap::new(),
        }
    }

    /// Index a document
    pub async fn index(&mut self, doc_id: &str, content: &str) -> Result<()> {
        // Index for BM25
        self.bm25_scorer.index(doc_id, content);

        // Generate and store embedding
        let embedding = self.embedding_provider.embed(content).await?;
        self.doc_embeddings.insert(doc_id.to_string(), embedding);
        self.doc_contents
            .insert(doc_id.to_string(), content.to_string());

        debug!(doc_id = %doc_id, "Indexed document for hybrid search");

        Ok(())
    }

    /// Index multiple documents
    pub async fn index_batch(&mut self, documents: Vec<(&str, &str)>) -> Result<()> {
        for (doc_id, content) in documents {
            self.index(doc_id, content).await?;
        }
        Ok(())
    }

    /// Remove a document
    pub fn remove(&mut self, doc_id: &str) {
        self.bm25_scorer.remove(doc_id);
        self.doc_embeddings.remove(doc_id);
        self.doc_contents.remove(doc_id);
    }

    /// Search using hybrid retrieval
    pub async fn search(&mut self, query: &str, limit: usize) -> Result<Vec<HybridSearchResult>> {
        // Get vector search results
        let vector_results = self.vector_search(query).await?;

        // Get keyword search results
        let keyword_results = self.bm25_scorer.score(query);

        // Fuse results
        let fused = if self.config.use_rrf {
            self.reciprocal_rank_fusion(vector_results, keyword_results)
        } else {
            self.weighted_fusion(vector_results, keyword_results)
        };

        // Take top results
        let results: Vec<_> = fused.into_iter().take(limit).collect();

        info!(
            query_len = query.len(),
            result_count = results.len(),
            "Hybrid search completed"
        );

        Ok(results)
    }

    /// Vector similarity search
    async fn vector_search(&self, query: &str) -> Result<Vec<(String, f32)>> {
        let query_embedding = self.embedding_provider.embed(query).await?;

        let mut results: Vec<(String, f32)> = self
            .doc_embeddings
            .iter()
            .map(|(doc_id, doc_embedding)| {
                let similarity = self.config.similarity_metric.calculate(&query_embedding, doc_embedding);
                (doc_id.clone(), similarity)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(self.config.candidates_per_retriever);

        Ok(results)
    }

    /// Reciprocal Rank Fusion
    fn reciprocal_rank_fusion(
        &self,
        vector_results: Vec<(String, f32)>,
        keyword_results: Vec<(String, f32)>,
    ) -> Vec<HybridSearchResult> {
        let mut scores: HashMap<String, (f32, Option<f32>, Option<f32>)> = HashMap::new();

        // Add vector results
        for (rank, (doc_id, score)) in vector_results.into_iter().enumerate() {
            let rrf_score = 1.0 / (self.config.rrf_k + rank + 1) as f32;
            let weighted_score = rrf_score * self.config.vector_weight;
            let entry = scores.entry(doc_id).or_insert((0.0, None, None));
            entry.0 += weighted_score;
            entry.1 = Some(score);
        }

        // Add keyword results
        for (rank, (doc_id, score)) in keyword_results.into_iter().enumerate() {
            let rrf_score = 1.0 / (self.config.rrf_k + rank + 1) as f32;
            let weighted_score = rrf_score * self.config.keyword_weight;
            let entry = scores.entry(doc_id).or_insert((0.0, None, None));
            entry.0 += weighted_score;
            entry.2 = Some(score);
        }

        // Convert to results
        let mut results: Vec<HybridSearchResult> = scores
            .into_iter()
            .map(|(doc_id, (score, vector_score, keyword_score))| HybridSearchResult {
                doc_id,
                score,
                vector_score,
                keyword_score,
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Weighted score fusion
    fn weighted_fusion(
        &self,
        vector_results: Vec<(String, f32)>,
        keyword_results: Vec<(String, f32)>,
    ) -> Vec<HybridSearchResult> {
        let mut scores: HashMap<String, (f32, Option<f32>, Option<f32>)> = HashMap::new();

        // Normalize vector scores
        let max_vector = vector_results.first().map(|(_, s)| *s).unwrap_or(1.0);

        for (doc_id, score) in vector_results {
            let normalized = if max_vector > 0.0 {
                score / max_vector
            } else {
                0.0
            };
            let weighted = normalized * self.config.vector_weight;
            let entry = scores.entry(doc_id).or_insert((0.0, None, None));
            entry.0 += weighted;
            entry.1 = Some(score);
        }

        // Normalize keyword scores
        let max_keyword = keyword_results.first().map(|(_, s)| *s).unwrap_or(1.0);

        for (doc_id, score) in keyword_results {
            let normalized = if max_keyword > 0.0 {
                score / max_keyword
            } else {
                0.0
            };
            let weighted = normalized * self.config.keyword_weight;
            let entry = scores.entry(doc_id).or_insert((0.0, None, None));
            entry.0 += weighted;
            entry.2 = Some(score);
        }

        // Convert to results
        let mut results: Vec<HybridSearchResult> = scores
            .into_iter()
            .map(|(doc_id, (score, vector_score, keyword_score))| HybridSearchResult {
                doc_id,
                score,
                vector_score,
                keyword_score,
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Get document content by ID
    pub fn get_content(&self, doc_id: &str) -> Option<&String> {
        self.doc_contents.get(doc_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity_metrics() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        assert!((SimilarityMetric::Cosine.calculate(&a, &b) - 1.0).abs() < 0.001);
        assert!(SimilarityMetric::Cosine.calculate(&a, &c).abs() < 0.001);
    }

    #[test]
    fn test_bm25_scorer() {
        let config = BM25Config::default();
        let mut scorer = BM25Scorer::new(config);

        scorer.index("doc1", "the quick brown fox jumps over the lazy dog");
        scorer.index("doc2", "the lazy cat sleeps all day");
        scorer.index("doc3", "rust programming language is fast");

        let results = scorer.score("lazy");
        assert!(!results.is_empty());

        // Both doc1 and doc2 contain "lazy"
        let doc_ids: Vec<_> = results.iter().map(|(id, _)| id.as_str()).collect();
        assert!(doc_ids.contains(&"doc1"));
        assert!(doc_ids.contains(&"doc2"));
    }

    #[test]
    fn test_tokenizer() {
        let tokens = tokenize("Hello, World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(!tokens.contains(&"a".to_string())); // Too short
    }

    #[tokio::test]
    async fn test_mock_embedding_provider() {
        let provider = MockEmbeddingProvider::new(128);

        let embedding1 = provider.embed("test text").await.unwrap();
        let embedding2 = provider.embed("test text").await.unwrap();
        let embedding3 = provider.embed("different text").await.unwrap();

        assert_eq!(embedding1.len(), 128);
        assert_eq!(embedding1, embedding2); // Same text = same embedding
        assert_ne!(embedding1, embedding3); // Different text = different embedding
    }

    #[tokio::test]
    async fn test_hybrid_search_engine() {
        let config = HybridSearchConfig::default();
        let provider = Arc::new(MockEmbeddingProvider::new(128));
        let mut engine = HybridSearchEngine::new(config, provider);

        engine.index("doc1", "rust programming language").await.unwrap();
        engine.index("doc2", "python programming language").await.unwrap();
        engine.index("doc3", "javascript web development").await.unwrap();

        let results = engine.search("rust programming", 10).await.unwrap();
        assert!(!results.is_empty());

        // First result should be doc1
        assert_eq!(results[0].doc_id, "doc1");
    }
}
