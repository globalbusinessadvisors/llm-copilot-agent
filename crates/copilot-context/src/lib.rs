//! Context Engine for LLM-CoPilot-Agent
//!
//! This crate provides multi-tier context management with intelligent retrieval,
//! compression, and token budget management for LLM interactions.

pub mod compression;
pub mod engine;
pub mod hybrid_search;
pub mod memory;
pub mod reranking;
pub mod retrieval;

// Re-exports
pub use engine::{ContextEngine, ContextEngineImpl, ContextEngineConfig};
pub use memory::{MemoryTier, MemoryItem, MemoryStore, MemoryMetadata};
pub use retrieval::{RelevanceScorer, ContextWindow, RetrievalConfig};
pub use compression::{CompressionStrategy, CompressionConfig, Compressor};
pub use hybrid_search::{
    HybridSearchEngine, HybridSearchConfig, HybridSearchResult,
    EmbeddingProvider, BM25Scorer, SimilarityMetric, Embedding,
    MockEmbeddingProvider, BM25Config,
};
pub use reranking::{
    Reranker, RerankerConfig, CrossEncoderReranker,
    RerankerResult, RerankerProvider,
};

/// Error types for context operations
#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Token limit exceeded: {current} / {limit}")]
    TokenLimitExceeded { current: usize, limit: usize },

    #[error("Invalid tier: {0}")]
    InvalidTier(String),

    #[error("Memory item not found: {0}")]
    ItemNotFound(String),

    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    #[error("Retrieval failed: {0}")]
    RetrievalFailed(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Core error: {0}")]
    CoreError(String),
}

pub type Result<T> = std::result::Result<T, ContextError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ContextError::TokenLimitExceeded {
            current: 250000,
            limit: 200000,
        };
        assert!(err.to_string().contains("250000"));
        assert!(err.to_string().contains("200000"));
    }
}
