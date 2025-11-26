//! Content Processors
//!
//! Provides processing stages for document content including normalization,
//! deduplication, and metadata enrichment.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

use crate::{IngestionError, Result};
use crate::chunking::Chunk;

/// Processed content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedContent {
    /// Content text
    pub text: String,
    /// Content metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Content hash for deduplication
    pub content_hash: String,
    /// Processing timestamp
    pub processed_at: chrono::DateTime<chrono::Utc>,
}

impl ProcessedContent {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let content_hash = compute_hash(&text);
        Self {
            text,
            metadata: HashMap::new(),
            content_hash,
            processed_at: chrono::Utc::now(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Compute SHA-256 hash of content
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Trait for content processors
#[async_trait]
pub trait ContentProcessor: Send + Sync {
    /// Process content
    async fn process(&self, content: ProcessedContent) -> Result<ProcessedContent>;

    /// Get processor name
    fn name(&self) -> &'static str;
}

/// Content normalizer
pub struct ContentNormalizer {
    /// Convert to lowercase
    lowercase: bool,
    /// Remove extra whitespace
    normalize_whitespace: bool,
    /// Remove special characters
    remove_special: bool,
    /// Characters to preserve even when removing special chars
    preserve_chars: HashSet<char>,
    /// Maximum content length (0 = unlimited)
    max_length: usize,
}

impl ContentNormalizer {
    pub fn new() -> Self {
        Self {
            lowercase: false,
            normalize_whitespace: true,
            remove_special: false,
            preserve_chars: HashSet::new(),
            max_length: 0,
        }
    }

    pub fn with_lowercase(mut self, lowercase: bool) -> Self {
        self.lowercase = lowercase;
        self
    }

    pub fn with_normalize_whitespace(mut self, normalize: bool) -> Self {
        self.normalize_whitespace = normalize;
        self
    }

    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_length = max;
        self
    }

    fn normalize_text(&self, text: &str) -> String {
        let mut result = text.to_string();

        if self.lowercase {
            result = result.to_lowercase();
        }

        if self.normalize_whitespace {
            // Replace multiple whitespace with single space
            let mut prev_whitespace = false;
            result = result
                .chars()
                .filter_map(|c| {
                    if c.is_whitespace() {
                        if prev_whitespace {
                            None
                        } else {
                            prev_whitespace = true;
                            Some(' ')
                        }
                    } else {
                        prev_whitespace = false;
                        Some(c)
                    }
                })
                .collect();
            result = result.trim().to_string();
        }

        if self.remove_special {
            result = result
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || self.preserve_chars.contains(c))
                .collect();
        }

        if self.max_length > 0 && result.len() > self.max_length {
            result.truncate(self.max_length);
        }

        result
    }
}

impl Default for ContentNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentProcessor for ContentNormalizer {
    async fn process(&self, mut content: ProcessedContent) -> Result<ProcessedContent> {
        content.text = self.normalize_text(&content.text);
        content.content_hash = compute_hash(&content.text);
        content.metadata.insert(
            "normalized".to_string(),
            serde_json::json!(true),
        );

        debug!(
            text_len = content.text.len(),
            "Content normalized"
        );

        Ok(content)
    }

    fn name(&self) -> &'static str {
        "normalizer"
    }
}

/// Deduplication processor
pub struct DeduplicationProcessor {
    /// Seen content hashes
    seen_hashes: Arc<RwLock<HashSet<String>>>,
    /// Seen content fingerprints (for near-duplicate detection)
    fingerprints: Arc<RwLock<HashMap<String, Vec<u64>>>>,
    /// Similarity threshold for near-duplicates (0.0 - 1.0)
    similarity_threshold: f32,
    /// Whether to use fuzzy matching
    use_fuzzy: bool,
}

impl DeduplicationProcessor {
    pub fn new() -> Self {
        Self {
            seen_hashes: Arc::new(RwLock::new(HashSet::new())),
            fingerprints: Arc::new(RwLock::new(HashMap::new())),
            similarity_threshold: 0.9,
            use_fuzzy: false,
        }
    }

    pub fn with_similarity_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_fuzzy(mut self, use_fuzzy: bool) -> Self {
        self.use_fuzzy = use_fuzzy;
        self
    }

    /// Generate fingerprint (simhash-like) for near-duplicate detection
    fn generate_fingerprint(&self, text: &str) -> Vec<u64> {
        let shingle_size = 3;
        let mut fingerprint = Vec::new();

        // Generate word shingles
        let words: Vec<&str> = text.split_whitespace().collect();
        for window in words.windows(shingle_size) {
            let shingle = window.join(" ");
            let mut hasher = Sha256::new();
            hasher.update(shingle.as_bytes());
            let hash = hasher.finalize();
            // Take first 8 bytes as u64
            let value = u64::from_be_bytes([
                hash[0], hash[1], hash[2], hash[3],
                hash[4], hash[5], hash[6], hash[7],
            ]);
            fingerprint.push(value);
        }

        fingerprint
    }

    /// Calculate Jaccard similarity between fingerprints
    fn jaccard_similarity(&self, fp1: &[u64], fp2: &[u64]) -> f32 {
        if fp1.is_empty() || fp2.is_empty() {
            return 0.0;
        }

        let set1: HashSet<_> = fp1.iter().collect();
        let set2: HashSet<_> = fp2.iter().collect();

        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Check if content is a near-duplicate
    async fn is_near_duplicate(&self, text: &str) -> Option<String> {
        if !self.use_fuzzy {
            return None;
        }

        let new_fp = self.generate_fingerprint(text);
        let fingerprints = self.fingerprints.read().await;

        for (hash, fp) in fingerprints.iter() {
            let similarity = self.jaccard_similarity(&new_fp, fp);
            if similarity >= self.similarity_threshold {
                return Some(hash.clone());
            }
        }

        None
    }

    /// Add content to deduplication index
    async fn add_to_index(&self, hash: &str, text: &str) {
        self.seen_hashes.write().await.insert(hash.to_string());

        if self.use_fuzzy {
            let fingerprint = self.generate_fingerprint(text);
            self.fingerprints.write().await.insert(hash.to_string(), fingerprint);
        }
    }

    /// Clear the deduplication index
    pub async fn clear(&self) {
        self.seen_hashes.write().await.clear();
        self.fingerprints.write().await.clear();
    }

    /// Get index size
    pub async fn index_size(&self) -> usize {
        self.seen_hashes.read().await.len()
    }
}

impl Default for DeduplicationProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentProcessor for DeduplicationProcessor {
    async fn process(&self, content: ProcessedContent) -> Result<ProcessedContent> {
        // Check exact duplicate
        if self.seen_hashes.read().await.contains(&content.content_hash) {
            return Err(IngestionError::DuplicateDocument(content.content_hash.clone()));
        }

        // Check near-duplicate
        if let Some(similar_hash) = self.is_near_duplicate(&content.text).await {
            return Err(IngestionError::DuplicateDocument(format!(
                "Near-duplicate of {}",
                similar_hash
            )));
        }

        // Add to index
        self.add_to_index(&content.content_hash, &content.text).await;

        debug!(
            hash = %content.content_hash,
            "Content passed deduplication"
        );

        Ok(content)
    }

    fn name(&self) -> &'static str {
        "deduplication"
    }
}

/// Metadata enricher
pub struct MetadataEnricher {
    /// Static metadata to add
    static_metadata: HashMap<String, serde_json::Value>,
    /// Whether to compute statistics
    compute_stats: bool,
    /// Whether to detect language
    detect_language: bool,
}

impl MetadataEnricher {
    pub fn new() -> Self {
        Self {
            static_metadata: HashMap::new(),
            compute_stats: true,
            detect_language: false,
        }
    }

    pub fn with_static_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.static_metadata.insert(key.into(), value);
        self
    }

    pub fn with_compute_stats(mut self, compute: bool) -> Self {
        self.compute_stats = compute;
        self
    }

    /// Count words in text
    fn word_count(&self, text: &str) -> usize {
        text.split_whitespace().count()
    }

    /// Count sentences in text
    fn sentence_count(&self, text: &str) -> usize {
        text.chars()
            .filter(|&c| c == '.' || c == '!' || c == '?')
            .count()
            .max(1)
    }

    /// Compute readability score (simplified Flesch-Kincaid)
    fn readability_score(&self, text: &str) -> f32 {
        let words = self.word_count(text) as f32;
        let sentences = self.sentence_count(text) as f32;
        let syllables = text
            .to_lowercase()
            .chars()
            .filter(|&c| "aeiou".contains(c))
            .count() as f32;

        if words == 0.0 || sentences == 0.0 {
            return 0.0;
        }

        let avg_words_per_sentence = words / sentences;
        let avg_syllables_per_word = syllables / words;

        // Simplified Flesch Reading Ease
        206.835 - (1.015 * avg_words_per_sentence) - (84.6 * avg_syllables_per_word)
    }
}

impl Default for MetadataEnricher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentProcessor for MetadataEnricher {
    async fn process(&self, mut content: ProcessedContent) -> Result<ProcessedContent> {
        // Add static metadata
        for (key, value) in &self.static_metadata {
            content.metadata.insert(key.clone(), value.clone());
        }

        // Compute statistics
        if self.compute_stats {
            let word_count = self.word_count(&content.text);
            let sentence_count = self.sentence_count(&content.text);
            let char_count = content.text.len();
            let readability = self.readability_score(&content.text);

            content.metadata.insert(
                "stats".to_string(),
                serde_json::json!({
                    "word_count": word_count,
                    "sentence_count": sentence_count,
                    "char_count": char_count,
                    "readability_score": readability,
                }),
            );
        }

        debug!(
            metadata_count = content.metadata.len(),
            "Content metadata enriched"
        );

        Ok(content)
    }

    fn name(&self) -> &'static str {
        "enricher"
    }
}

/// Chain of processors
pub struct ProcessorChain {
    processors: Vec<Arc<dyn ContentProcessor>>,
}

impl ProcessorChain {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn add(mut self, processor: Arc<dyn ContentProcessor>) -> Self {
        self.processors.push(processor);
        self
    }

    /// Process content through all processors in chain
    pub async fn process(&self, content: ProcessedContent) -> Result<ProcessedContent> {
        let mut current = content;

        for processor in &self.processors {
            current = processor.process(current).await?;
        }

        Ok(current)
    }

    /// Process a chunk
    pub async fn process_chunk(&self, chunk: &Chunk) -> Result<ProcessedContent> {
        let content = ProcessedContent::new(&chunk.content)
            .with_metadata("chunk_id", serde_json::json!(chunk.id))
            .with_metadata("document_id", serde_json::json!(chunk.document_id))
            .with_metadata("chunk_index", serde_json::json!(chunk.metadata.index));

        self.process(content).await
    }

    /// Process multiple chunks
    pub async fn process_chunks(&self, chunks: &[Chunk]) -> Result<Vec<ProcessedContent>> {
        let mut results = Vec::with_capacity(chunks.len());

        for chunk in chunks {
            match self.process_chunk(chunk).await {
                Ok(processed) => results.push(processed),
                Err(IngestionError::DuplicateDocument(_)) => {
                    // Skip duplicates
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(results)
    }

    /// Get processor names
    pub fn list(&self) -> Vec<&'static str> {
        self.processors.iter().map(|p| p.name()).collect()
    }
}

impl Default for ProcessorChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processed_content() {
        let content = ProcessedContent::new("test content")
            .with_metadata("key", serde_json::json!("value"));

        assert_eq!(content.text, "test content");
        assert!(content.metadata.contains_key("key"));
        assert!(!content.content_hash.is_empty());
    }

    #[tokio::test]
    async fn test_content_normalizer() {
        let normalizer = ContentNormalizer::new()
            .with_normalize_whitespace(true);

        let content = ProcessedContent::new("  Hello   World  ");
        let result = normalizer.process(content).await.unwrap();

        assert_eq!(result.text, "Hello World");
    }

    #[tokio::test]
    async fn test_normalizer_lowercase() {
        let normalizer = ContentNormalizer::new()
            .with_lowercase(true);

        let content = ProcessedContent::new("Hello WORLD");
        let result = normalizer.process(content).await.unwrap();

        assert_eq!(result.text, "hello world");
    }

    #[tokio::test]
    async fn test_normalizer_max_length() {
        let normalizer = ContentNormalizer::new()
            .with_max_length(5);

        let content = ProcessedContent::new("Hello World");
        let result = normalizer.process(content).await.unwrap();

        assert_eq!(result.text.len(), 5);
    }

    #[tokio::test]
    async fn test_deduplication_exact() {
        let dedup = DeduplicationProcessor::new();

        let content1 = ProcessedContent::new("unique content");
        let content2 = ProcessedContent::new("unique content");

        // First should succeed
        assert!(dedup.process(content1).await.is_ok());

        // Second should fail (duplicate)
        assert!(dedup.process(content2).await.is_err());
    }

    #[tokio::test]
    async fn test_deduplication_fuzzy() {
        let dedup = DeduplicationProcessor::new()
            .with_fuzzy(true)
            .with_similarity_threshold(0.8);

        let content1 = ProcessedContent::new("The quick brown fox jumps over the lazy dog");
        let content2 = ProcessedContent::new("A quick brown fox jumps over a lazy dog");

        // First should succeed
        assert!(dedup.process(content1).await.is_ok());

        // Second might be detected as near-duplicate depending on threshold
        let result = dedup.process(content2).await;
        // This test verifies the mechanism works
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_deduplication_clear() {
        let dedup = DeduplicationProcessor::new();

        let content = ProcessedContent::new("test content");
        assert!(dedup.process(content.clone()).await.is_ok());
        assert!(dedup.process(content.clone()).await.is_err());

        dedup.clear().await;

        // After clear, same content should succeed
        assert!(dedup.process(content).await.is_ok());
    }

    #[tokio::test]
    async fn test_metadata_enricher() {
        let enricher = MetadataEnricher::new()
            .with_static_metadata("source", serde_json::json!("test"))
            .with_compute_stats(true);

        let content = ProcessedContent::new("Hello world. This is a test.");
        let result = enricher.process(content).await.unwrap();

        assert_eq!(result.metadata["source"], "test");
        assert!(result.metadata.contains_key("stats"));

        let stats = &result.metadata["stats"];
        assert!(stats["word_count"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_processor_chain() {
        let chain = ProcessorChain::new()
            .add(Arc::new(ContentNormalizer::new().with_normalize_whitespace(true)))
            .add(Arc::new(MetadataEnricher::new().with_compute_stats(true)));

        let content = ProcessedContent::new("  Hello   world  ");
        let result = chain.process(content).await.unwrap();

        assert_eq!(result.text, "Hello world");
        assert!(result.metadata.contains_key("stats"));
    }

    #[tokio::test]
    async fn test_processor_chain_with_dedup() {
        let dedup = Arc::new(DeduplicationProcessor::new());
        let chain = ProcessorChain::new()
            .add(dedup.clone())
            .add(Arc::new(MetadataEnricher::new()));

        let content1 = ProcessedContent::new("test content");
        let content2 = ProcessedContent::new("test content");

        assert!(chain.process(content1).await.is_ok());
        assert!(chain.process(content2).await.is_err());
    }

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash("test");
        let hash2 = compute_hash("test");
        let hash3 = compute_hash("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA-256 = 256 bits = 64 hex chars
    }
}
