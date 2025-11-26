//! Document Ingestion Pipeline
//!
//! Provides a complete pipeline for ingesting documents including extraction,
//! chunking, processing, and output.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::chunking::{Chunk, ChunkingConfig, TextChunker};
use crate::extractors::{ExtractorRegistry, ExtractionResult};
use crate::processors::{ContentProcessor, ProcessedContent, ProcessorChain};
use crate::{IngestionError, Result};

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Original filename
    pub filename: Option<String>,
    /// Content type (MIME)
    pub content_type: String,
    /// File size in bytes
    pub size: usize,
    /// Source identifier
    pub source: Option<String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

impl DocumentMetadata {
    pub fn new(content_type: impl Into<String>, size: usize) -> Self {
        Self {
            filename: None,
            content_type: content_type.into(),
            size,
            source: None,
            created_at: chrono::Utc::now(),
            custom: HashMap::new(),
        }
    }

    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }
}

/// A document to be ingested
#[derive(Debug, Clone)]
pub struct Document {
    /// Unique document identifier
    pub id: String,
    /// Raw document content
    pub content: Vec<u8>,
    /// Document metadata
    pub metadata: DocumentMetadata,
}

impl Document {
    pub fn new(id: impl Into<String>, content: Vec<u8>, metadata: DocumentMetadata) -> Self {
        Self {
            id: id.into(),
            content,
            metadata,
        }
    }

    /// Create from file path
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = tokio::fs::read(path).await?;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let content_type = mime_guess::from_path(path)
            .first_or_text_plain()
            .to_string();

        let metadata = DocumentMetadata::new(content_type, content.len())
            .with_filename(filename);

        let id = format!(
            "{}_{:x}",
            filename,
            uuid::Uuid::new_v4().as_simple()
        );

        Ok(Self::new(id, content, metadata))
    }

    /// Create from raw text
    pub fn from_text(id: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let content = text.into_bytes();
        let metadata = DocumentMetadata::new("text/plain", content.len());
        Self::new(id, content, metadata)
    }
}

/// Result of document ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionResult {
    /// Document ID
    pub document_id: String,
    /// Number of chunks created
    pub chunk_count: usize,
    /// Processed chunks
    pub chunks: Vec<ProcessedChunk>,
    /// Extraction metadata
    pub extraction_metadata: HashMap<String, serde_json::Value>,
    /// Processing duration in milliseconds
    pub processing_time_ms: u64,
    /// Warnings during processing
    pub warnings: Vec<String>,
    /// Whether ingestion was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// A processed chunk ready for storage/embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedChunk {
    /// Chunk ID
    pub id: String,
    /// Document ID
    pub document_id: String,
    /// Chunk content
    pub content: String,
    /// Content hash
    pub content_hash: String,
    /// Chunk metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl From<ProcessedContent> for ProcessedChunk {
    fn from(content: ProcessedContent) -> Self {
        let id = content
            .metadata
            .get("chunk_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let document_id = content
            .metadata
            .get("document_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            id,
            document_id,
            content: content.text,
            content_hash: content.content_hash,
            metadata: content.metadata,
        }
    }
}

/// Pipeline stage trait
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// Stage name
    fn name(&self) -> &'static str;

    /// Process document through this stage
    async fn process(&self, context: &mut PipelineContext) -> Result<()>;
}

/// Context passed through pipeline stages
pub struct PipelineContext {
    /// Original document
    pub document: Document,
    /// Extracted text
    pub extracted_text: Option<String>,
    /// Extraction result
    pub extraction_result: Option<ExtractionResult>,
    /// Generated chunks
    pub chunks: Vec<Chunk>,
    /// Processed chunks
    pub processed_chunks: Vec<ProcessedContent>,
    /// Warnings collected during processing
    pub warnings: Vec<String>,
    /// Stage-specific data
    pub stage_data: HashMap<String, serde_json::Value>,
}

impl PipelineContext {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            extracted_text: None,
            extraction_result: None,
            chunks: Vec::new(),
            processed_chunks: Vec::new(),
            warnings: Vec::new(),
            stage_data: HashMap::new(),
        }
    }

    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

/// Extraction stage
pub struct ExtractionStage {
    registry: ExtractorRegistry,
}

impl ExtractionStage {
    pub fn new(registry: ExtractorRegistry) -> Self {
        Self { registry }
    }
}

impl Default for ExtractionStage {
    fn default() -> Self {
        Self::new(ExtractorRegistry::with_defaults())
    }
}

#[async_trait]
impl PipelineStage for ExtractionStage {
    fn name(&self) -> &'static str {
        "extraction"
    }

    async fn process(&self, context: &mut PipelineContext) -> Result<()> {
        let extractor = if let Some(ref filename) = context.document.metadata.filename {
            self.registry.get_by_filename(filename)
        } else {
            self.registry.get_extractor(&context.document.metadata.content_type)
        };

        let result = extractor
            .extract(
                &context.document.content,
                context.document.metadata.filename.as_deref(),
            )
            .await?;

        // Add any extraction warnings
        for warning in &result.warnings {
            context.add_warning(warning.clone());
        }

        context.extracted_text = Some(result.text.clone());
        context.extraction_result = Some(result);

        debug!(
            document_id = %context.document.id,
            extractor = %extractor.name(),
            "Text extracted"
        );

        Ok(())
    }
}

/// Chunking stage
pub struct ChunkingStage {
    chunker: TextChunker,
}

impl ChunkingStage {
    pub fn new(config: ChunkingConfig) -> Result<Self> {
        let chunker = TextChunker::new(config)?;
        Ok(Self { chunker })
    }
}

impl Default for ChunkingStage {
    fn default() -> Self {
        Self::new(ChunkingConfig::default()).expect("Default config should be valid")
    }
}

#[async_trait]
impl PipelineStage for ChunkingStage {
    fn name(&self) -> &'static str {
        "chunking"
    }

    async fn process(&self, context: &mut PipelineContext) -> Result<()> {
        let text = context
            .extracted_text
            .as_ref()
            .ok_or_else(|| IngestionError::PipelineError("No extracted text available".to_string()))?;

        let chunks = self.chunker.chunk(&context.document.id, text)?;

        debug!(
            document_id = %context.document.id,
            chunk_count = chunks.len(),
            "Document chunked"
        );

        context.chunks = chunks;
        Ok(())
    }
}

/// Processing stage
pub struct ProcessingStage {
    chain: ProcessorChain,
}

impl ProcessingStage {
    pub fn new(chain: ProcessorChain) -> Self {
        Self { chain }
    }
}

impl Default for ProcessingStage {
    fn default() -> Self {
        Self::new(ProcessorChain::new())
    }
}

#[async_trait]
impl PipelineStage for ProcessingStage {
    fn name(&self) -> &'static str {
        "processing"
    }

    async fn process(&self, context: &mut PipelineContext) -> Result<()> {
        if context.chunks.is_empty() {
            context.add_warning("No chunks to process".to_string());
            return Ok(());
        }

        let processed = self.chain.process_chunks(&context.chunks).await?;

        debug!(
            document_id = %context.document.id,
            processed_count = processed.len(),
            "Chunks processed"
        );

        context.processed_chunks = processed;
        Ok(())
    }
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Chunking configuration
    pub chunking: ChunkingConfig,
    /// Maximum document size in bytes
    pub max_document_size: usize,
    /// Whether to fail on warnings
    pub strict_mode: bool,
    /// Parallel processing (number of documents)
    pub parallelism: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            chunking: ChunkingConfig::default(),
            max_document_size: 50 * 1024 * 1024, // 50MB
            strict_mode: false,
            parallelism: 4,
        }
    }
}

/// Document ingestion pipeline
pub struct IngestionPipeline {
    config: PipelineConfig,
    stages: Vec<Arc<dyn PipelineStage>>,
    stats: Arc<RwLock<PipelineStats>>,
}

/// Pipeline statistics
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    pub documents_processed: u64,
    pub documents_failed: u64,
    pub chunks_created: u64,
    pub total_processing_time_ms: u64,
    pub bytes_processed: u64,
}

impl IngestionPipeline {
    pub fn new(config: PipelineConfig) -> Result<Self> {
        Ok(Self {
            config,
            stages: Vec::new(),
            stats: Arc::new(RwLock::new(PipelineStats::default())),
        })
    }

    /// Create with default stages
    pub fn with_defaults(config: PipelineConfig) -> Result<Self> {
        let mut pipeline = Self::new(config.clone())?;
        pipeline.add_stage(Arc::new(ExtractionStage::default()));
        pipeline.add_stage(Arc::new(ChunkingStage::new(config.chunking)?));
        pipeline.add_stage(Arc::new(ProcessingStage::default()));
        Ok(pipeline)
    }

    /// Add a pipeline stage
    pub fn add_stage(&mut self, stage: Arc<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    /// Ingest a single document
    pub async fn ingest(&self, document: Document) -> IngestionResult {
        let start = std::time::Instant::now();
        let document_id = document.id.clone();
        let doc_size = document.content.len();

        // Check document size
        if doc_size > self.config.max_document_size {
            return IngestionResult {
                document_id,
                chunk_count: 0,
                chunks: Vec::new(),
                extraction_metadata: HashMap::new(),
                processing_time_ms: start.elapsed().as_millis() as u64,
                warnings: Vec::new(),
                success: false,
                error: Some(format!(
                    "Document too large: {} bytes (max {})",
                    doc_size, self.config.max_document_size
                )),
            };
        }

        let mut context = PipelineContext::new(document);

        // Run through stages
        for stage in &self.stages {
            if let Err(e) = stage.process(&mut context).await {
                // Update stats
                let mut stats = self.stats.write().await;
                stats.documents_failed += 1;

                warn!(
                    document_id = %document_id,
                    stage = %stage.name(),
                    error = %e,
                    "Pipeline stage failed"
                );

                return IngestionResult {
                    document_id,
                    chunk_count: 0,
                    chunks: Vec::new(),
                    extraction_metadata: HashMap::new(),
                    processing_time_ms: start.elapsed().as_millis() as u64,
                    warnings: context.warnings,
                    success: false,
                    error: Some(e.to_string()),
                };
            }
        }

        let processing_time_ms = start.elapsed().as_millis() as u64;

        // Convert processed content to chunks
        let chunks: Vec<ProcessedChunk> = context
            .processed_chunks
            .into_iter()
            .map(ProcessedChunk::from)
            .collect();

        // Extract metadata
        let extraction_metadata = context
            .extraction_result
            .map(|r| r.metadata)
            .unwrap_or_default();

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.documents_processed += 1;
            stats.chunks_created += chunks.len() as u64;
            stats.total_processing_time_ms += processing_time_ms;
            stats.bytes_processed += doc_size as u64;
        }

        info!(
            document_id = %document_id,
            chunk_count = chunks.len(),
            processing_time_ms = processing_time_ms,
            "Document ingested"
        );

        IngestionResult {
            document_id,
            chunk_count: chunks.len(),
            chunks,
            extraction_metadata,
            processing_time_ms,
            warnings: context.warnings,
            success: true,
            error: None,
        }
    }

    /// Ingest multiple documents
    pub async fn ingest_batch(&self, documents: Vec<Document>) -> Vec<IngestionResult> {
        let mut results = Vec::with_capacity(documents.len());

        // Process in chunks based on parallelism setting
        for chunk in documents.chunks(self.config.parallelism) {
            let mut handles = Vec::new();

            for doc in chunk {
                let doc = doc.clone();
                let stages = self.stages.clone();
                let stats = self.stats.clone();
                let config = self.config.clone();

                handles.push(tokio::spawn(async move {
                    let pipeline = IngestionPipeline {
                        config,
                        stages,
                        stats,
                    };
                    pipeline.ingest(doc).await
                }));
            }

            for handle in handles {
                match handle.await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        results.push(IngestionResult {
                            document_id: "unknown".to_string(),
                            chunk_count: 0,
                            chunks: Vec::new(),
                            extraction_metadata: HashMap::new(),
                            processing_time_ms: 0,
                            warnings: Vec::new(),
                            success: false,
                            error: Some(format!("Task failed: {}", e)),
                        });
                    }
                }
            }
        }

        results
    }

    /// Get pipeline statistics
    pub async fn stats(&self) -> PipelineStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        *self.stats.write().await = PipelineStats::default();
    }

    /// List pipeline stages
    pub fn list_stages(&self) -> Vec<&'static str> {
        self.stages.iter().map(|s| s.name()).collect()
    }
}

/// Builder for custom pipeline stages
pub struct CustomStage<F>
where
    F: Fn(&mut PipelineContext) -> Result<()> + Send + Sync + 'static,
{
    name: &'static str,
    processor: F,
}

impl<F> CustomStage<F>
where
    F: Fn(&mut PipelineContext) -> Result<()> + Send + Sync + 'static,
{
    pub fn new(name: &'static str, processor: F) -> Self {
        Self { name, processor }
    }
}

#[async_trait]
impl<F> PipelineStage for CustomStage<F>
where
    F: Fn(&mut PipelineContext) -> Result<()> + Send + Sync + 'static,
{
    fn name(&self) -> &'static str {
        self.name
    }

    async fn process(&self, context: &mut PipelineContext) -> Result<()> {
        (self.processor)(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_document_from_text() {
        let doc = Document::from_text("test-doc", "Hello, world!");

        assert_eq!(doc.id, "test-doc");
        assert_eq!(doc.metadata.content_type, "text/plain");
        assert_eq!(doc.content, b"Hello, world!");
    }

    #[tokio::test]
    async fn test_document_metadata() {
        let meta = DocumentMetadata::new("text/plain", 100)
            .with_filename("test.txt")
            .with_source("upload")
            .with_custom("author", serde_json::json!("Test User"));

        assert_eq!(meta.filename, Some("test.txt".to_string()));
        assert_eq!(meta.source, Some("upload".to_string()));
        assert!(meta.custom.contains_key("author"));
    }

    #[tokio::test]
    async fn test_pipeline_basic() {
        let config = PipelineConfig::default();
        let pipeline = IngestionPipeline::with_defaults(config).unwrap();

        let doc = Document::from_text("doc1", "This is a test document with some content.");
        let result = pipeline.ingest(doc).await;

        assert!(result.success);
        assert!(!result.chunks.is_empty());
        assert_eq!(result.document_id, "doc1");
    }

    #[tokio::test]
    async fn test_pipeline_with_markdown() {
        let config = PipelineConfig::default();
        let pipeline = IngestionPipeline::with_defaults(config).unwrap();

        let content = b"# Title\n\nSome text here.\n\n## Section\n\nMore content.".to_vec();
        let metadata = DocumentMetadata::new("text/markdown", content.len())
            .with_filename("test.md");
        let doc = Document::new("md-doc", content, metadata);

        let result = pipeline.ingest(doc).await;

        assert!(result.success);
        assert!(result.extraction_metadata.contains_key("headings"));
    }

    #[tokio::test]
    async fn test_pipeline_batch() {
        let config = PipelineConfig {
            parallelism: 2,
            ..Default::default()
        };
        let pipeline = IngestionPipeline::with_defaults(config).unwrap();

        let docs = vec![
            Document::from_text("doc1", "First document content."),
            Document::from_text("doc2", "Second document content."),
            Document::from_text("doc3", "Third document content."),
        ];

        let results = pipeline.ingest_batch(docs).await;

        assert_eq!(results.len(), 3);
        for result in &results {
            assert!(result.success);
        }
    }

    #[tokio::test]
    async fn test_pipeline_stats() {
        let config = PipelineConfig::default();
        let pipeline = IngestionPipeline::with_defaults(config).unwrap();

        let doc = Document::from_text("doc1", "Test content with more text to ensure we have enough for chunk creation and processing.");
        let _ = pipeline.ingest(doc).await;

        let stats = pipeline.stats().await;
        assert_eq!(stats.documents_processed, 1);
        assert!(stats.chunks_created > 0);

        pipeline.reset_stats().await;
        let stats = pipeline.stats().await;
        assert_eq!(stats.documents_processed, 0);
    }

    #[tokio::test]
    async fn test_pipeline_large_document_rejection() {
        let config = PipelineConfig {
            max_document_size: 10, // Very small limit
            ..Default::default()
        };
        let pipeline = IngestionPipeline::with_defaults(config).unwrap();

        let doc = Document::from_text("doc1", "This content is too large for the limit.");
        let result = pipeline.ingest(doc).await;

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_pipeline_stages_list() {
        let config = PipelineConfig::default();
        let pipeline = IngestionPipeline::with_defaults(config).unwrap();

        let stages = pipeline.list_stages();
        assert!(stages.contains(&"extraction"));
        assert!(stages.contains(&"chunking"));
        assert!(stages.contains(&"processing"));
    }

    #[tokio::test]
    async fn test_processed_chunk_conversion() {
        let content = ProcessedContent::new("test content")
            .with_metadata("chunk_id", serde_json::json!("chunk-1"))
            .with_metadata("document_id", serde_json::json!("doc-1"));

        let chunk = ProcessedChunk::from(content);

        assert_eq!(chunk.id, "chunk-1");
        assert_eq!(chunk.document_id, "doc-1");
        assert_eq!(chunk.content, "test content");
    }

    #[tokio::test]
    async fn test_custom_stage() {
        let custom = CustomStage::new("custom", |ctx| {
            ctx.stage_data.insert(
                "custom_processed".to_string(),
                serde_json::json!(true),
            );
            Ok(())
        });

        let mut ctx = PipelineContext::new(Document::from_text("doc", "test"));
        ctx.extracted_text = Some("test".to_string());

        custom.process(&mut ctx).await.unwrap();

        assert!(ctx.stage_data.contains_key("custom_processed"));
    }
}
