//! Context retrieval and relevance scoring
//!
//! Provides intelligent retrieval of context items based on relevance,
//! importance, and recency with token budget management.

use crate::{ContextError, MemoryItem, Result};
use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;
use std::cmp::Ordering;

/// Configuration for context retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Maximum tokens in context window
    pub max_tokens: usize,

    /// Target token utilization (0.0 - 1.0)
    pub target_utilization: f64,

    /// Weight for relevance score (0.0 - 1.0)
    pub relevance_weight: f64,

    /// Weight for importance score (0.0 - 1.0)
    pub importance_weight: f64,

    /// Weight for recency score (0.0 - 1.0)
    pub recency_weight: f64,

    /// Minimum relevance threshold
    pub min_relevance: f64,

    /// Include compressed content
    pub allow_compressed: bool,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            max_tokens: 200_000,
            target_utilization: 0.8,
            relevance_weight: 0.5,
            importance_weight: 0.3,
            recency_weight: 0.2,
            min_relevance: 0.3,
            allow_compressed: true,
        }
    }
}

impl RetrievalConfig {
    pub fn validate(&self) -> Result<()> {
        let total_weight = self.relevance_weight + self.importance_weight + self.recency_weight;
        if (total_weight - 1.0).abs() > 0.001 {
            return Err(ContextError::RetrievalFailed(format!(
                "Weights must sum to 1.0, got {}",
                total_weight
            )));
        }

        if self.target_utilization <= 0.0 || self.target_utilization > 1.0 {
            return Err(ContextError::RetrievalFailed(
                "Target utilization must be in (0, 1]".to_string(),
            ));
        }

        Ok(())
    }

    pub fn target_tokens(&self) -> usize {
        (self.max_tokens as f64 * self.target_utilization) as usize
    }
}

/// Relevance scorer for context items
pub struct RelevanceScorer {
    config: RetrievalConfig,
}

impl RelevanceScorer {
    pub fn new(config: RetrievalConfig) -> Self {
        Self { config }
    }

    /// Calculate relevance score between query and content
    pub fn calculate_relevance(&self, query: &str, content: &str) -> f64 {
        // Simple keyword-based relevance (in production, use embeddings)
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();

        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        if query_words.is_empty() {
            return 0.5; // Neutral score for empty query
        }

        let mut matches = 0;
        let mut weighted_matches = 0.0;

        for (idx, word) in query_words.iter().enumerate() {
            if word.len() < 3 {
                continue; // Skip short words
            }

            let count = content_lower.matches(word).count();
            if count > 0 {
                matches += 1;
                // Earlier words in query are more important
                let position_weight = 1.0 - (idx as f64 / query_words.len() as f64) * 0.3;
                weighted_matches += count as f64 * position_weight;
            }
        }

        let coverage = matches as f64 / query_words.len() as f64;
        let density = (weighted_matches / content.len() as f64).min(1.0) * 10.0;

        (coverage * 0.7 + density * 0.3).min(1.0)
    }

    /// Calculate recency score (0.0 - 1.0)
    pub fn calculate_recency(&self, item: &MemoryItem) -> f64 {
        let age_seconds = (chrono::Utc::now() - item.last_accessed).num_seconds() as f64;
        let half_life_hours = 24.0; // Score halves every 24 hours

        let decay = (-age_seconds / (half_life_hours * 3600.0) * 0.693).exp();
        decay
    }

    /// Calculate composite score for retrieval prioritization
    pub fn calculate_score(&self, query: &str, item: &MemoryItem) -> f64 {
        let relevance = self.calculate_relevance(query, item.get_content());
        let importance = item.current_importance();
        let recency = self.calculate_recency(item);

        self.config.relevance_weight * relevance
            + self.config.importance_weight * importance
            + self.config.recency_weight * recency
    }

    /// Filter items by minimum relevance
    pub fn filter_relevant(&self, query: &str, items: Vec<MemoryItem>) -> Vec<ScoredItem> {
        items
            .into_iter()
            .filter_map(|item| {
                let relevance = self.calculate_relevance(query, item.get_content());
                if relevance >= self.config.min_relevance {
                    let score = self.calculate_score(query, &item);
                    Some(ScoredItem { item, score })
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Memory item with retrieval score
#[derive(Debug, Clone)]
pub struct ScoredItem {
    pub item: MemoryItem,
    pub score: f64,
}

impl PartialEq for ScoredItem {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for ScoredItem {}

impl PartialOrd for ScoredItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Standard ordering for max-heap (BinaryHeap pops largest first)
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(Ordering::Equal)
    }
}

/// Context window manager with token budget
pub struct ContextWindow {
    config: RetrievalConfig,
    scorer: RelevanceScorer,
}

impl ContextWindow {
    pub fn new(config: RetrievalConfig) -> Result<Self> {
        config.validate()?;
        let scorer = RelevanceScorer::new(config.clone());
        Ok(Self { config, scorer })
    }

    /// Retrieve and prioritize items within token budget
    pub fn retrieve(&self, query: &str, items: Vec<MemoryItem>) -> Result<RetrievalResult> {
        let target_tokens = self.config.target_tokens();

        // Score and filter items
        let mut scored_items = self.scorer.filter_relevant(query, items);

        // Sort by score (descending)
        scored_items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

        // Select items within token budget using greedy algorithm
        let mut selected = Vec::new();
        let mut current_tokens = 0;
        let mut rejected = Vec::new();

        for scored_item in scored_items {
            let item_tokens = scored_item.item.token_count;

            if current_tokens + item_tokens <= target_tokens {
                current_tokens += item_tokens;
                selected.push(scored_item);
            } else {
                rejected.push(scored_item);
            }
        }

        Ok(RetrievalResult {
            selected,
            rejected,
            total_tokens: current_tokens,
            target_tokens,
            max_tokens: self.config.max_tokens,
        })
    }

    /// Retrieve with advanced prioritization (knapsack-like optimization)
    pub fn retrieve_optimized(&self, query: &str, items: Vec<MemoryItem>) -> Result<RetrievalResult> {
        let target_tokens = self.config.target_tokens();

        // Score and filter items
        let scored_items = self.scorer.filter_relevant(query, items);

        // Use priority queue for better selection
        let mut heap: BinaryHeap<ScoredItem> = scored_items.into_iter().collect();

        let mut selected = Vec::new();
        let mut current_tokens = 0;
        let mut rejected = Vec::new();

        // First pass: greedy selection by score
        while let Some(scored_item) = heap.pop() {
            let item_tokens = scored_item.item.token_count;

            if current_tokens + item_tokens <= target_tokens {
                current_tokens += item_tokens;
                selected.push(scored_item);
            } else {
                rejected.push(scored_item);
            }
        }

        // Optimization: try to swap items for better utilization
        self.optimize_selection(&mut selected, &mut rejected, target_tokens);

        let total_tokens = selected.iter().map(|s| s.item.token_count).sum();

        Ok(RetrievalResult {
            selected,
            rejected,
            total_tokens,
            target_tokens,
            max_tokens: self.config.max_tokens,
        })
    }

    /// Optimize selection for better token utilization
    fn optimize_selection(
        &self,
        selected: &mut Vec<ScoredItem>,
        rejected: &mut Vec<ScoredItem>,
        target_tokens: usize,
    ) {
        let current_tokens: usize = selected.iter().map(|s| s.item.token_count).sum();
        let available_tokens = target_tokens.saturating_sub(current_tokens);

        if available_tokens == 0 {
            return;
        }

        // Try to swap or add items for better utilization
        rejected.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));

        let mut i = 0;
        while i < rejected.len() {
            let rejected_item = &rejected[i];
            let rejected_tokens = rejected_item.item.token_count;

            if rejected_tokens <= available_tokens {
                // Can fit directly
                let item = rejected.remove(i);
                selected.push(item);
                return; // One swap is enough per optimization pass
            } else {
                // Try to swap with a lower-scored selected item
                if let Some(swap_idx) = self.find_swap_candidate(
                    selected,
                    rejected_item,
                    rejected_tokens,
                    available_tokens,
                ) {
                    let removed = selected.remove(swap_idx);
                    let added = rejected.remove(i);
                    selected.push(added);
                    rejected.push(removed);
                    return;
                }
            }
            i += 1;
        }
    }

    /// Find a candidate for swapping
    fn find_swap_candidate(
        &self,
        selected: &[ScoredItem],
        new_item: &ScoredItem,
        new_tokens: usize,
        available_tokens: usize,
    ) -> Option<usize> {
        for (idx, item) in selected.iter().enumerate() {
            if item.score < new_item.score {
                let freed_tokens = item.item.token_count;
                if freed_tokens + available_tokens >= new_tokens {
                    return Some(idx);
                }
            }
        }
        None
    }

    /// Build context string from selected items
    pub fn build_context(&self, result: &RetrievalResult) -> String {
        let mut context = String::new();

        for scored in &result.selected {
            context.push_str("--- Context Item ---\n");
            context.push_str(&format!("ID: {}\n", scored.item.metadata.id));
            context.push_str(&format!("Type: {}\n", scored.item.metadata.content_type));
            context.push_str(&format!("Score: {:.3}\n", scored.score));
            context.push_str(&format!("Importance: {:.3}\n", scored.item.current_importance()));
            context.push_str(&format!("Tokens: {}\n", scored.item.token_count));
            context.push_str("---\n");
            context.push_str(scored.item.get_content());
            context.push_str("\n\n");
        }

        context
    }
}

/// Result of context retrieval operation
#[derive(Debug)]
pub struct RetrievalResult {
    /// Items selected for context
    pub selected: Vec<ScoredItem>,

    /// Items rejected (below threshold or out of budget)
    pub rejected: Vec<ScoredItem>,

    /// Total tokens in selected items
    pub total_tokens: usize,

    /// Target token budget
    pub target_tokens: usize,

    /// Maximum allowed tokens
    pub max_tokens: usize,
}

impl RetrievalResult {
    /// Get utilization percentage
    pub fn utilization(&self) -> f64 {
        self.total_tokens as f64 / self.max_tokens as f64
    }

    /// Get efficiency (utilization vs target)
    pub fn efficiency(&self) -> f64 {
        self.total_tokens as f64 / self.target_tokens as f64
    }

    /// Check if within budget
    pub fn is_within_budget(&self) -> bool {
        self.total_tokens <= self.max_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryMetadata, MemoryTier};

    fn create_test_item(content: &str, importance: f64, tokens: usize) -> MemoryItem {
        MemoryItem::new(
            content.to_string(),
            MemoryMetadata::new("test", "test"),
            importance,
            tokens,
        )
    }

    #[test]
    fn test_retrieval_config_validation() {
        let mut config = RetrievalConfig::default();
        assert!(config.validate().is_ok());

        config.relevance_weight = 0.5;
        config.importance_weight = 0.3;
        config.recency_weight = 0.1; // Sum != 1.0
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_relevance_calculation() {
        let config = RetrievalConfig::default();
        let scorer = RelevanceScorer::new(config);

        let query = "rust programming language";
        let content1 = "Rust is a systems programming language";
        let content2 = "Python is a dynamic programming language";

        let score1 = scorer.calculate_relevance(query, content1);
        let score2 = scorer.calculate_relevance(query, content2);

        assert!(score1 > score2);
    }

    #[test]
    fn test_context_window_retrieval() {
        let mut config = RetrievalConfig::default();
        config.max_tokens = 1000;
        config.target_utilization = 0.8;

        let window = ContextWindow::new(config).unwrap();

        let items = vec![
            create_test_item("rust programming", 0.9, 200),
            create_test_item("python code", 0.7, 200),
            create_test_item("javascript tutorial", 0.6, 200),
            create_test_item("java examples", 0.5, 200),
            create_test_item("c++ basics", 0.4, 200),
        ];

        let result = window.retrieve("rust programming", items).unwrap();

        assert!(result.is_within_budget());
        assert!(result.total_tokens <= 800); // Target is 80% of 1000
        assert!(!result.selected.is_empty());
    }

    #[test]
    fn test_scored_item_ordering() {
        let item1 = create_test_item("test1", 0.9, 100);
        let item2 = create_test_item("test2", 0.7, 100);

        let scored1 = ScoredItem {
            item: item1,
            score: 0.9,
        };
        let scored2 = ScoredItem {
            item: item2,
            score: 0.7,
        };

        let mut heap = BinaryHeap::new();
        heap.push(scored2);
        heap.push(scored1);

        let first = heap.pop().unwrap();
        assert_eq!(first.score, 0.9);
    }

    #[test]
    fn test_recency_calculation() {
        let config = RetrievalConfig::default();
        let scorer = RelevanceScorer::new(config);

        let item = create_test_item("test", 0.5, 100);
        let recency = scorer.calculate_recency(&item);

        assert!(recency > 0.99); // Very recent
        assert!(recency <= 1.0);
    }
}
