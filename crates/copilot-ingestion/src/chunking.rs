//! Text Chunking Module
//!
//! Provides intelligent text chunking strategies for splitting documents
//! into optimal sizes for embedding and retrieval.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

use crate::{IngestionError, Result};

/// Metadata associated with a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Chunk index within the document
    pub index: usize,
    /// Start character offset in original document
    pub start_offset: usize,
    /// End character offset in original document
    pub end_offset: usize,
    /// Token count (approximate)
    pub token_count: usize,
    /// Character count
    pub char_count: usize,
    /// Section or heading this chunk belongs to (if detected)
    pub section: Option<String>,
    /// Additional metadata
    pub extra: HashMap<String, serde_json::Value>,
}

/// A chunk of text from a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique chunk identifier
    pub id: String,
    /// Source document identifier
    pub document_id: String,
    /// Chunk content
    pub content: String,
    /// Chunk metadata
    pub metadata: ChunkMetadata,
}

impl Chunk {
    pub fn new(
        document_id: impl Into<String>,
        content: impl Into<String>,
        metadata: ChunkMetadata,
    ) -> Self {
        let content = content.into();
        let document_id = document_id.into();
        let id = format!("{}_{}", document_id, metadata.index);
        Self {
            id,
            document_id,
            content,
            metadata,
        }
    }
}

/// Chunking strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChunkingStrategy {
    /// Fixed size chunks with overlap
    #[default]
    FixedSize,
    /// Split by sentences
    Sentence,
    /// Split by paragraphs
    Paragraph,
    /// Split by sections (headers)
    Section,
    /// Semantic chunking (attempts to keep related content together)
    Semantic,
    /// Recursive character-based splitting
    Recursive,
}

/// Configuration for text chunking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Chunking strategy to use
    pub strategy: ChunkingStrategy,
    /// Target chunk size in tokens
    pub chunk_size: usize,
    /// Overlap between chunks in tokens
    pub chunk_overlap: usize,
    /// Minimum chunk size (won't create smaller chunks)
    pub min_chunk_size: usize,
    /// Maximum chunk size (will split larger chunks)
    pub max_chunk_size: usize,
    /// Separators for recursive splitting (in priority order)
    pub separators: Vec<String>,
    /// Whether to trim whitespace from chunks
    pub trim_chunks: bool,
    /// Whether to preserve paragraph boundaries when possible
    pub preserve_paragraphs: bool,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            strategy: ChunkingStrategy::FixedSize,
            chunk_size: 512,
            chunk_overlap: 50,
            min_chunk_size: 10,  // Allow small chunks (in tokens)
            max_chunk_size: 1000,
            separators: vec![
                "\n\n".to_string(),
                "\n".to_string(),
                ". ".to_string(),
                "! ".to_string(),
                "? ".to_string(),
                "; ".to_string(),
                ", ".to_string(),
                " ".to_string(),
            ],
            trim_chunks: true,
            preserve_paragraphs: true,
        }
    }
}

impl ChunkingConfig {
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    pub fn with_overlap(mut self, overlap: usize) -> Self {
        self.chunk_overlap = overlap;
        self
    }

    pub fn with_strategy(mut self, strategy: ChunkingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.chunk_size == 0 {
            return Err(IngestionError::ValidationError(
                "Chunk size must be greater than 0".to_string(),
            ));
        }
        if self.chunk_overlap >= self.chunk_size {
            return Err(IngestionError::ValidationError(
                "Chunk overlap must be less than chunk size".to_string(),
            ));
        }
        if self.min_chunk_size > self.max_chunk_size {
            return Err(IngestionError::ValidationError(
                "Min chunk size cannot exceed max chunk size".to_string(),
            ));
        }
        Ok(())
    }
}

/// Text chunker for splitting documents
pub struct TextChunker {
    config: ChunkingConfig,
}

impl TextChunker {
    pub fn new(config: ChunkingConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Chunk a document
    pub fn chunk(&self, document_id: &str, text: &str) -> Result<Vec<Chunk>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        let chunks = match self.config.strategy {
            ChunkingStrategy::FixedSize => self.chunk_fixed_size(text),
            ChunkingStrategy::Sentence => self.chunk_by_sentences(text),
            ChunkingStrategy::Paragraph => self.chunk_by_paragraphs(text),
            ChunkingStrategy::Section => self.chunk_by_sections(text),
            ChunkingStrategy::Semantic => self.chunk_semantic(text),
            ChunkingStrategy::Recursive => self.chunk_recursive(text, 0),
        }?;

        // Convert to Chunk objects with metadata
        let result: Vec<Chunk> = chunks
            .into_iter()
            .enumerate()
            .filter(|(_, (content, _, _))| {
                let trimmed = content.trim();
                !trimmed.is_empty() && self.estimate_tokens(trimmed) >= self.config.min_chunk_size
            })
            .map(|(index, (content, start, end))| {
                let content = if self.config.trim_chunks {
                    content.trim().to_string()
                } else {
                    content
                };
                let token_count = self.estimate_tokens(&content);

                Chunk::new(
                    document_id,
                    content.clone(),
                    ChunkMetadata {
                        index,
                        start_offset: start,
                        end_offset: end,
                        token_count,
                        char_count: content.len(),
                        section: None,
                        extra: HashMap::new(),
                    },
                )
            })
            .collect();

        debug!(
            document_id = %document_id,
            chunk_count = result.len(),
            "Document chunked"
        );

        Ok(result)
    }

    /// Estimate token count (simple approximation: ~4 chars per token)
    fn estimate_tokens(&self, text: &str) -> usize {
        (text.len() + 3) / 4
    }

    /// Fixed size chunking with overlap
    fn chunk_fixed_size(&self, text: &str) -> Result<Vec<(String, usize, usize)>> {
        let chars: Vec<char> = text.chars().collect();
        let total_chars = chars.len();

        // Approximate chars per token
        let chars_per_token = 4;
        let chunk_chars = self.config.chunk_size * chars_per_token;
        let overlap_chars = self.config.chunk_overlap * chars_per_token;
        let step = chunk_chars.saturating_sub(overlap_chars).max(1);

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < total_chars {
            let end = (start + chunk_chars).min(total_chars);
            let content: String = chars[start..end].iter().collect();
            chunks.push((content, start, end));

            if end >= total_chars {
                break;
            }
            start += step;
        }

        Ok(chunks)
    }

    /// Chunk by sentences
    fn chunk_by_sentences(&self, text: &str) -> Result<Vec<(String, usize, usize)>> {
        let sentences = self.split_into_sentences(text);
        self.merge_chunks(sentences, text)
    }

    /// Split text into sentences
    fn split_into_sentences(&self, text: &str) -> Vec<(String, usize, usize)> {
        let mut sentences = Vec::new();
        let mut current_start = 0;
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();

        for (i, &c) in chars.iter().enumerate() {
            current.push(c);

            // Check for sentence ending
            let is_sentence_end = (c == '.' || c == '!' || c == '?')
                && chars.get(i + 1).map(|&nc| nc.is_whitespace() || nc == '"' || nc == '\'').unwrap_or(true);

            if is_sentence_end || i == chars.len() - 1 {
                if !current.trim().is_empty() {
                    sentences.push((current.clone(), current_start, i + 1));
                }
                current.clear();
                current_start = i + 1;
            }
        }

        sentences
    }

    /// Chunk by paragraphs
    fn chunk_by_paragraphs(&self, text: &str) -> Result<Vec<(String, usize, usize)>> {
        let paragraphs: Vec<(String, usize, usize)> = text
            .split("\n\n")
            .scan(0, |offset, para| {
                let start = *offset;
                let end = start + para.len();
                *offset = end + 2; // Account for \n\n
                Some((para.to_string(), start, end))
            })
            .filter(|(p, _, _)| !p.trim().is_empty())
            .collect();

        self.merge_chunks(paragraphs, text)
    }

    /// Chunk by sections (markdown headers)
    fn chunk_by_sections(&self, text: &str) -> Result<Vec<(String, usize, usize)>> {
        let mut sections = Vec::new();
        let mut current_section = String::new();
        let mut section_start = 0;
        let mut offset = 0;

        for line in text.lines() {
            let line_len = line.len() + 1; // +1 for newline

            if line.starts_with('#') {
                // New section header
                if !current_section.trim().is_empty() {
                    sections.push((current_section.clone(), section_start, offset));
                }
                current_section = String::new();
                section_start = offset;
            }

            current_section.push_str(line);
            current_section.push('\n');
            offset += line_len;
        }

        // Don't forget the last section
        if !current_section.trim().is_empty() {
            sections.push((current_section, section_start, offset));
        }

        // Merge small sections or split large ones
        self.merge_chunks(sections, text)
    }

    /// Semantic chunking (keeps related content together)
    fn chunk_semantic(&self, text: &str) -> Result<Vec<(String, usize, usize)>> {
        // For now, use paragraph-based chunking as a proxy for semantic
        // In a production system, this would use embeddings to detect topic shifts
        self.chunk_by_paragraphs(text)
    }

    /// Recursive character-based chunking
    fn chunk_recursive(&self, text: &str, separator_idx: usize) -> Result<Vec<(String, usize, usize)>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        let token_estimate = self.estimate_tokens(text);

        // If text is small enough, return as single chunk
        if token_estimate <= self.config.chunk_size {
            return Ok(vec![(text.to_string(), 0, text.len())]);
        }

        // Try splitting with current separator
        let separator = self.config.separators.get(separator_idx);

        if let Some(sep) = separator {
            let parts: Vec<&str> = text.split(sep).collect();

            if parts.len() > 1 {
                let mut chunks = Vec::new();
                let mut current = String::new();
                let mut current_start = 0;
                let mut offset = 0;

                for (i, part) in parts.iter().enumerate() {
                    let potential = if current.is_empty() {
                        part.to_string()
                    } else {
                        format!("{}{}{}", current, sep, part)
                    };

                    if self.estimate_tokens(&potential) <= self.config.chunk_size {
                        current = potential;
                    } else {
                        if !current.is_empty() {
                            chunks.push((current.clone(), current_start, offset));
                        }
                        current = part.to_string();
                        current_start = offset;
                    }

                    offset += part.len();
                    if i < parts.len() - 1 {
                        offset += sep.len();
                    }
                }

                if !current.is_empty() {
                    chunks.push((current, current_start, offset));
                }

                // Recursively split any chunks that are still too large
                let mut result = Vec::new();
                for (chunk, start, end) in chunks {
                    if self.estimate_tokens(&chunk) > self.config.chunk_size {
                        let sub_chunks = self.chunk_recursive(&chunk, separator_idx + 1)?;
                        for (sub, sub_start, sub_end) in sub_chunks {
                            result.push((sub, start + sub_start, start + sub_end));
                        }
                    } else {
                        result.push((chunk, start, end));
                    }
                }

                return Ok(result);
            }
        }

        // If we've exhausted separators, fall back to fixed size
        self.chunk_fixed_size(text)
    }

    /// Merge small chunks to reach target size
    fn merge_chunks(
        &self,
        chunks: Vec<(String, usize, usize)>,
        _original: &str,
    ) -> Result<Vec<(String, usize, usize)>> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        let mut merged = Vec::new();
        let mut current = String::new();
        let mut current_start = 0;
        let mut current_end = 0;

        for (chunk, start, end) in chunks {
            let potential = if current.is_empty() {
                chunk.clone()
            } else {
                format!("{}\n\n{}", current, chunk)
            };

            let potential_tokens = self.estimate_tokens(&potential);

            if potential_tokens <= self.config.chunk_size {
                // Merge
                if current.is_empty() {
                    current_start = start;
                }
                current = potential;
                current_end = end;
            } else {
                // Save current and start new
                if !current.is_empty() {
                    merged.push((current, current_start, current_end));
                }
                current = chunk;
                current_start = start;
                current_end = end;
            }
        }

        // Don't forget the last chunk
        if !current.is_empty() {
            merged.push((current, current_start, current_end));
        }

        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking_config_validation() {
        let config = ChunkingConfig::default();
        assert!(config.validate().is_ok());

        let invalid = ChunkingConfig {
            chunk_size: 0,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());

        let invalid2 = ChunkingConfig {
            chunk_overlap: 600,
            chunk_size: 500,
            ..Default::default()
        };
        assert!(invalid2.validate().is_err());
    }

    #[test]
    fn test_fixed_size_chunking() {
        let config = ChunkingConfig {
            chunk_size: 50,
            chunk_overlap: 10,
            min_chunk_size: 5, // Lower minimum for test
            ..Default::default()
        };
        let chunker = TextChunker::new(config).unwrap();

        let text = "This is a long text that should be split into multiple chunks based on the configured size and overlap parameters. Adding more content to ensure we have enough text for multiple chunks to be created properly.";
        let chunks = chunker.chunk("doc1", text).unwrap();

        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_sentence_chunking() {
        let config = ChunkingConfig {
            strategy: ChunkingStrategy::Sentence,
            chunk_size: 100,
            min_chunk_size: 5,
            ..Default::default()
        };
        let chunker = TextChunker::new(config).unwrap();

        let text = "First sentence here with more words. Second sentence with more content! Third sentence is also long? Fourth sentence completes the test.";
        let chunks = chunker.chunk("doc1", text).unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_paragraph_chunking() {
        let config = ChunkingConfig {
            strategy: ChunkingStrategy::Paragraph,
            chunk_size: 200,
            min_chunk_size: 5,
            ..Default::default()
        };
        let chunker = TextChunker::new(config).unwrap();

        let text = "First paragraph content here with enough words to pass minimum size checks.\n\nSecond paragraph content here also with sufficient length.\n\nThird paragraph has plenty of content too.";
        let chunks = chunker.chunk("doc1", text).unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_section_chunking() {
        let config = ChunkingConfig {
            strategy: ChunkingStrategy::Section,
            chunk_size: 200,
            min_chunk_size: 5,
            ..Default::default()
        };
        let chunker = TextChunker::new(config).unwrap();

        let text = "# Header 1\n\nContent for section 1 with more detailed information here.\n\n# Header 2\n\nContent for section 2 with additional details and context.";
        let chunks = chunker.chunk("doc1", text).unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_recursive_chunking() {
        let config = ChunkingConfig {
            strategy: ChunkingStrategy::Recursive,
            chunk_size: 100,  // Larger to accommodate overlap
            chunk_overlap: 10,
            min_chunk_size: 5,
            ..Default::default()
        };
        let chunker = TextChunker::new(config).unwrap();

        let text = "Short paragraph one with some more content.\n\nShort paragraph two also has content.\n\nA much longer paragraph that contains many more words and should potentially be split further if needed due to the size limit.";
        let chunks = chunker.chunk("doc1", text).unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_empty_text() {
        let config = ChunkingConfig::default();
        let chunker = TextChunker::new(config).unwrap();

        let chunks = chunker.chunk("doc1", "").unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_metadata() {
        let config = ChunkingConfig::default().with_chunk_size(100);
        let chunker = TextChunker::new(config).unwrap();

        let text = "Some sample text for testing chunk metadata generation.";
        let chunks = chunker.chunk("doc1", text).unwrap();

        for chunk in &chunks {
            assert!(chunk.metadata.token_count > 0);
            assert!(chunk.metadata.char_count > 0);
            assert!(chunk.metadata.end_offset >= chunk.metadata.start_offset);
        }
    }

    #[test]
    fn test_chunk_id_generation() {
        let config = ChunkingConfig::default();
        let chunker = TextChunker::new(config).unwrap();

        let text = "Test text for ID generation. More content here to create multiple chunks if the text is long enough.";
        let chunks = chunker.chunk("mydoc", text).unwrap();

        for chunk in &chunks {
            assert!(chunk.id.starts_with("mydoc_"));
        }
    }
}
