//! Document Ingestion Pipeline for LLM-CoPilot-Agent
//!
//! This crate provides a comprehensive document ingestion system for processing
//! various document types, extracting text, chunking, and preparing content for
//! vector embedding and retrieval.
//!
//! # Features
//!
//! - Multi-format document processing (text, markdown, JSON, code)
//! - Intelligent text chunking with overlap
//! - Content deduplication using fingerprinting
//! - Metadata extraction and enrichment
//! - Pipeline-based processing with configurable stages
//! - Async streaming support for large documents

pub mod chunking;
pub mod extractors;
pub mod pipeline;
pub mod processors;

// Re-exports
pub use chunking::{
    ChunkingStrategy, ChunkingConfig, TextChunker,
    Chunk, ChunkMetadata,
};
pub use extractors::{
    TextExtractor, ExtractorRegistry, ExtractionResult,
    PlainTextExtractor, MarkdownExtractor, JsonExtractor,
};
pub use pipeline::{
    IngestionPipeline, PipelineConfig, PipelineStage,
    IngestionResult, Document, DocumentMetadata,
};
pub use processors::{
    ContentProcessor, ProcessorChain, DeduplicationProcessor,
    MetadataEnricher, ContentNormalizer,
};

/// Error types for ingestion operations
#[derive(Debug, thiserror::Error)]
pub enum IngestionError {
    #[error("Unsupported document type: {0}")]
    UnsupportedType(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Chunking failed: {0}")]
    ChunkingFailed(String),

    #[error("Processing failed: {0}")]
    ProcessingFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Pipeline error: {0}")]
    PipelineError(String),

    #[error("Duplicate document: {0}")]
    DuplicateDocument(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

pub type Result<T> = std::result::Result<T, IngestionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = IngestionError::UnsupportedType("application/octet-stream".to_string());
        assert!(err.to_string().contains("Unsupported document type"));
    }
}
