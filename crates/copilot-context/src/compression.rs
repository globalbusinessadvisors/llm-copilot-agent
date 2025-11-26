//! Context compression strategies
//!
//! Provides various compression strategies to manage token budgets effectively,
//! including summarization, truncation, and intelligent content reduction.

use crate::{ContextError, MemoryItem, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compression strategy enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionStrategy {
    /// No compression
    None,

    /// Truncate content (lossy)
    Truncate,

    /// Summarize content (lossy, preserves meaning)
    Summarize,

    /// Extract key information (lossy)
    Extract,

    /// Remove redundancy (lossless when possible)
    Deduplicate,

    /// Hybrid approach (summarize + extract)
    Hybrid,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Compression strategy to use
    pub strategy: CompressionStrategy,

    /// Target compression ratio (0.0 - 1.0)
    /// 0.5 means compress to 50% of original size
    pub target_ratio: f64,

    /// Minimum compression threshold (don't compress below this size)
    pub min_size: usize,

    /// Maximum tokens per item after compression
    pub max_tokens_per_item: usize,

    /// Preserve important sections (e.g., errors, code blocks)
    pub preserve_important: bool,

    /// Enable aggressive compression when needed
    pub allow_aggressive: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            strategy: CompressionStrategy::Hybrid,
            target_ratio: 0.5,
            min_size: 100,
            max_tokens_per_item: 2000,
            preserve_important: true,
            allow_aggressive: false,
        }
    }
}

impl CompressionConfig {
    pub fn validate(&self) -> Result<()> {
        if self.target_ratio <= 0.0 || self.target_ratio > 1.0 {
            return Err(ContextError::CompressionFailed(
                "Target ratio must be in (0, 1]".to_string(),
            ));
        }
        Ok(())
    }
}

/// Context compressor
pub struct Compressor {
    config: CompressionConfig,
}

impl Compressor {
    pub fn new(config: CompressionConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Compress a single memory item
    pub fn compress_item(&self, item: &MemoryItem) -> Result<String> {
        if item.token_count < self.config.min_size {
            return Ok(item.content.clone());
        }

        match self.config.strategy {
            CompressionStrategy::None => Ok(item.content.clone()),
            CompressionStrategy::Truncate => self.truncate(&item.content, item.token_count),
            CompressionStrategy::Summarize => self.summarize(&item.content, item.token_count),
            CompressionStrategy::Extract => self.extract(&item.content),
            CompressionStrategy::Deduplicate => self.deduplicate(&item.content),
            CompressionStrategy::Hybrid => self.hybrid(&item.content, item.token_count),
        }
    }

    /// Compress multiple items together (batch compression)
    pub fn compress_batch(&self, items: &[MemoryItem]) -> Result<Vec<String>> {
        items.iter().map(|item| self.compress_item(item)).collect()
    }

    /// Calculate compression metrics
    pub fn calculate_metrics(&self, original: &str, compressed: &str) -> CompressionMetrics {
        let original_len = original.len();
        let compressed_len = compressed.len();

        let ratio = if original_len > 0 {
            compressed_len as f64 / original_len as f64
        } else {
            1.0
        };

        CompressionMetrics {
            original_size: original_len,
            compressed_size: compressed_len,
            ratio,
            bytes_saved: original_len.saturating_sub(compressed_len),
        }
    }

    /// Estimate tokens saved by compression
    pub fn estimate_tokens_saved(&self, original_tokens: usize) -> usize {
        let target_tokens = (original_tokens as f64 * self.config.target_ratio) as usize;
        original_tokens.saturating_sub(target_tokens)
    }

    // Compression strategy implementations

    /// Truncate content to target size
    fn truncate(&self, content: &str, current_tokens: usize) -> Result<String> {
        let target_tokens = self.calculate_target_tokens(current_tokens);
        let target_chars = (content.len() as f64 * target_tokens as f64 / current_tokens as f64) as usize;

        if target_chars >= content.len() {
            return Ok(content.to_string());
        }

        // Try to truncate at sentence boundary
        let truncated = &content[..target_chars];
        if let Some(last_period) = truncated.rfind('.') {
            Ok(format!("{}...", &truncated[..=last_period]))
        } else if let Some(last_space) = truncated.rfind(' ') {
            Ok(format!("{}...", &truncated[..last_space]))
        } else {
            Ok(format!("{}...", truncated))
        }
    }

    /// Summarize content (extract key sentences)
    fn summarize(&self, content: &str, current_tokens: usize) -> Result<String> {
        let target_tokens = self.calculate_target_tokens(current_tokens);

        // Split into sentences
        let sentences = self.split_sentences(content);
        if sentences.is_empty() {
            return Ok(content.to_string());
        }

        // Score sentences by importance
        let scored_sentences = self.score_sentences(&sentences, content);

        // Select top sentences within token budget
        let mut selected = Vec::new();
        let mut current_length = 0;
        let target_length = (content.len() as f64 * target_tokens as f64 / current_tokens as f64) as usize;

        for (sentence, _score) in scored_sentences {
            if current_length + sentence.len() > target_length {
                break;
            }
            selected.push(sentence);
            current_length += sentence.len();
        }

        if selected.is_empty() {
            selected.push(sentences[0]);
        }

        Ok(selected.join(" "))
    }

    /// Extract key information (structured data, code, errors)
    fn extract(&self, content: &str) -> Result<String> {
        let mut extracted = Vec::new();

        // Extract code blocks
        if let Some(code) = self.extract_code_blocks(content) {
            extracted.push(format!("Code:\n{}", code));
        }

        // Extract errors
        if let Some(errors) = self.extract_errors(content) {
            extracted.push(format!("Errors:\n{}", errors));
        }

        // Extract key phrases
        let key_phrases = self.extract_key_phrases(content);
        if !key_phrases.is_empty() {
            extracted.push(format!("Key Points:\n- {}", key_phrases.join("\n- ")));
        }

        if extracted.is_empty() {
            // Fallback to truncation
            self.truncate(content, content.len() / 4)
        } else {
            Ok(extracted.join("\n\n"))
        }
    }

    /// Deduplicate content (remove repeated sections)
    fn deduplicate(&self, content: &str) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut seen = HashMap::new();
        let mut deduplicated = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let count = seen.entry(trimmed.to_string()).or_insert(0);
            *count += 1;

            // Keep first occurrence and mark duplicates
            if *count == 1 {
                deduplicated.push(line.to_string());
            } else if *count == 2 {
                // Replace first occurrence with count indicator
                if let Some(pos) = deduplicated.iter().position(|l| l.trim() == trimmed) {
                    deduplicated[pos] = format!("{} [repeated]", line);
                }
            }
        }

        Ok(deduplicated.join("\n"))
    }

    /// Hybrid compression (combine strategies)
    fn hybrid(&self, content: &str, current_tokens: usize) -> Result<String> {
        // First, deduplicate
        let deduplicated = self.deduplicate(content)?;

        // Then extract key information if content is large
        let extracted = if current_tokens > self.config.max_tokens_per_item {
            self.extract(&deduplicated)?
        } else {
            deduplicated
        };

        // Finally, summarize if still too large
        let estimated_tokens = (extracted.len() as f64 / content.len() as f64 * current_tokens as f64) as usize;
        if estimated_tokens > self.calculate_target_tokens(current_tokens) {
            self.summarize(&extracted, estimated_tokens)
        } else {
            Ok(extracted)
        }
    }

    // Helper methods

    fn calculate_target_tokens(&self, current_tokens: usize) -> usize {
        let target = (current_tokens as f64 * self.config.target_ratio) as usize;
        target.min(self.config.max_tokens_per_item)
    }

    fn split_sentences<'a>(&self, content: &'a str) -> Vec<&'a str> {
        content
            .split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .collect()
    }

    fn score_sentences<'a>(&self, sentences: &[&'a str], full_content: &str) -> Vec<(&'a str, f64)> {
        let mut scored: Vec<_> = sentences
            .iter()
            .map(|&sentence| {
                let score = self.calculate_sentence_importance(sentence, full_content);
                (sentence, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    fn calculate_sentence_importance(&self, sentence: &str, _full_content: &str) -> f64 {
        let mut score = 0.0;

        // Length-based score (prefer medium-length sentences)
        let len = sentence.len();
        if len > 20 && len < 200 {
            score += 0.3;
        }

        // Keyword-based scoring
        let sentence_lower = sentence.to_lowercase();
        if sentence_lower.contains("error") || sentence_lower.contains("exception") {
            score += 0.4;
        }
        if sentence_lower.contains("important") || sentence_lower.contains("critical") {
            score += 0.3;
        }
        if sentence_lower.contains("note") || sentence_lower.contains("warning") {
            score += 0.2;
        }

        // Position score (first sentences are often important)
        score += 0.2;

        score
    }

    fn extract_code_blocks(&self, content: &str) -> Option<String> {
        let mut code_blocks = Vec::new();

        // Extract markdown code blocks
        let mut in_code_block = false;
        let mut current_block = Vec::new();

        for line in content.lines() {
            if line.trim().starts_with("```") {
                if in_code_block {
                    if !current_block.is_empty() {
                        code_blocks.push(current_block.join("\n"));
                        current_block.clear();
                    }
                }
                in_code_block = !in_code_block;
            } else if in_code_block {
                current_block.push(line);
            }
        }

        // Extract inline code (simple heuristic)
        for line in content.lines() {
            if line.contains("fn ") || line.contains("def ") || line.contains("function ") {
                code_blocks.push(line.to_string());
            }
        }

        if code_blocks.is_empty() {
            None
        } else {
            Some(code_blocks.join("\n---\n"))
        }
    }

    fn extract_errors(&self, content: &str) -> Option<String> {
        let error_lines: Vec<String> = content
            .lines()
            .filter(|line| {
                let line_lower = line.to_lowercase();
                line_lower.contains("error")
                    || line_lower.contains("exception")
                    || line_lower.contains("failed")
                    || line_lower.contains("panic")
            })
            .map(|s| s.to_string())
            .collect();

        if error_lines.is_empty() {
            None
        } else {
            Some(error_lines.join("\n"))
        }
    }

    fn extract_key_phrases(&self, content: &str) -> Vec<String> {
        let mut phrases = Vec::new();

        for line in content.lines() {
            let line_trimmed = line.trim();

            // Extract bullet points
            if line_trimmed.starts_with("- ") || line_trimmed.starts_with("* ") {
                phrases.push(line_trimmed[2..].to_string());
            }

            // Extract numbered lists
            if line_trimmed.chars().next().map_or(false, |c| c.is_numeric())
                && line_trimmed.contains(". ") {
                if let Some(content) = line_trimmed.split(". ").nth(1) {
                    phrases.push(content.to_string());
                }
            }
        }

        phrases
    }
}

/// Compression metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionMetrics {
    pub original_size: usize,
    pub compressed_size: usize,
    pub ratio: f64,
    pub bytes_saved: usize,
}

impl CompressionMetrics {
    pub fn compression_percentage(&self) -> f64 {
        (1.0 - self.ratio) * 100.0
    }
}

/// Token budget manager
pub struct TokenBudgetManager {
    max_tokens: usize,
    target_utilization: f64,
    current_tokens: usize,
}

impl TokenBudgetManager {
    pub fn new(max_tokens: usize, target_utilization: f64) -> Self {
        Self {
            max_tokens,
            target_utilization,
            current_tokens: 0,
        }
    }

    /// Get target token budget (200K context window by default)
    pub fn target_budget(&self) -> usize {
        (self.max_tokens as f64 * self.target_utilization) as usize
    }

    /// Check if we need compression
    pub fn needs_compression(&self) -> bool {
        self.current_tokens > self.target_budget()
    }

    /// Calculate how many tokens need to be freed
    pub fn tokens_to_free(&self) -> usize {
        if self.current_tokens > self.target_budget() {
            self.current_tokens - self.target_budget()
        } else {
            0
        }
    }

    /// Add tokens to current count
    pub fn add_tokens(&mut self, tokens: usize) -> Result<()> {
        let new_total = self.current_tokens + tokens;
        if new_total > self.max_tokens {
            return Err(ContextError::TokenLimitExceeded {
                current: new_total,
                limit: self.max_tokens,
            });
        }
        self.current_tokens = new_total;
        Ok(())
    }

    /// Remove tokens from current count
    pub fn remove_tokens(&mut self, tokens: usize) {
        self.current_tokens = self.current_tokens.saturating_sub(tokens);
    }

    /// Get current utilization (0.0 - 1.0)
    pub fn utilization(&self) -> f64 {
        self.current_tokens as f64 / self.max_tokens as f64
    }

    /// Check if within budget
    pub fn is_within_budget(&self) -> bool {
        self.current_tokens <= self.target_budget()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryItem, MemoryMetadata};

    fn create_test_item(content: &str, tokens: usize) -> MemoryItem {
        MemoryItem::new(
            content.to_string(),
            MemoryMetadata::new("test", "test"),
            0.5,
            tokens,
        )
    }

    #[test]
    fn test_compression_config_validation() {
        let config = CompressionConfig::default();
        assert!(config.validate().is_ok());

        let mut invalid_config = config.clone();
        invalid_config.target_ratio = 1.5;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_truncation() {
        let config = CompressionConfig {
            strategy: CompressionStrategy::Truncate,
            target_ratio: 0.5,
            ..Default::default()
        };
        let compressor = Compressor::new(config).unwrap();

        let content = "This is a test sentence. This is another sentence. And one more.";
        let item = create_test_item(content, 100);
        let compressed = compressor.compress_item(&item).unwrap();

        assert!(compressed.len() < content.len());
        assert!(compressed.ends_with("..."));
    }

    #[test]
    fn test_deduplication() {
        let config = CompressionConfig {
            strategy: CompressionStrategy::Deduplicate,
            ..Default::default()
        };
        let compressor = Compressor::new(config).unwrap();

        let content = "Line 1\nLine 2\nLine 1\nLine 3\nLine 2";
        let result = compressor.deduplicate(content).unwrap();

        // Check that duplicate lines are marked
        assert!(result.contains("[repeated]"));
        // Result should have fewer lines (removed duplicates)
        let result_lines: Vec<&str> = result.lines().collect();
        let content_lines: Vec<&str> = content.lines().collect();
        assert!(result_lines.len() < content_lines.len());
    }

    #[test]
    fn test_token_budget_manager() {
        let mut manager = TokenBudgetManager::new(200_000, 0.8);

        assert_eq!(manager.target_budget(), 160_000);
        assert!(!manager.needs_compression());

        manager.add_tokens(170_000).unwrap();
        assert!(manager.needs_compression());
        assert_eq!(manager.tokens_to_free(), 10_000);

        manager.remove_tokens(20_000);
        assert!(!manager.needs_compression());
    }

    #[test]
    fn test_compression_metrics() {
        let config = CompressionConfig::default();
        let compressor = Compressor::new(config).unwrap();

        let original = "This is a long string that will be compressed";
        let compressed = "Short string";

        let metrics = compressor.calculate_metrics(original, compressed);
        assert!(metrics.ratio < 1.0);
        assert!(metrics.bytes_saved > 0);
        assert!(metrics.compression_percentage() > 0.0);
    }

    #[test]
    fn test_code_extraction() {
        let config = CompressionConfig {
            strategy: CompressionStrategy::Extract,
            ..Default::default()
        };
        let compressor = Compressor::new(config).unwrap();

        let content = r#"
Here is some text.
```rust
fn main() {
    println!("Hello");
}
```
More text here.
        "#;

        let item = create_test_item(content, 100);
        let extracted = compressor.compress_item(&item).unwrap();

        assert!(extracted.contains("Code:"));
        assert!(extracted.contains("fn main"));
    }
}
