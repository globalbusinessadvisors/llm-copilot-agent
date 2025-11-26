//! Memory management and storage tiers
//!
//! Provides multi-tier memory storage with automatic tier management based on
//! importance, recency, and access patterns.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{ContextError, Result};

/// Memory tier enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryTier {
    /// Short-term memory (recent, frequently accessed)
    /// Retention: Minutes to hours
    /// Capacity: ~10K tokens
    ShortTerm,

    /// Medium-term memory (important, moderately accessed)
    /// Retention: Hours to days
    /// Capacity: ~50K tokens
    MediumTerm,

    /// Long-term memory (archived, rarely accessed)
    /// Retention: Days to weeks
    /// Capacity: ~140K tokens
    LongTerm,
}

impl MemoryTier {
    /// Get the token capacity for this tier
    pub fn token_capacity(&self) -> usize {
        match self {
            MemoryTier::ShortTerm => 10_000,
            MemoryTier::MediumTerm => 50_000,
            MemoryTier::LongTerm => 140_000,
        }
    }

    /// Get the decay rate for importance scoring (lower = slower decay)
    pub fn decay_rate(&self) -> f64 {
        match self {
            MemoryTier::ShortTerm => 0.1,    // Fast decay
            MemoryTier::MediumTerm => 0.05,  // Medium decay
            MemoryTier::LongTerm => 0.01,    // Slow decay
        }
    }

    /// Get the minimum importance threshold for this tier
    pub fn importance_threshold(&self) -> f64 {
        match self {
            MemoryTier::ShortTerm => 0.3,
            MemoryTier::MediumTerm => 0.5,
            MemoryTier::LongTerm => 0.7,
        }
    }
}

/// Metadata associated with memory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Unique identifier
    pub id: Uuid,

    /// Content type (e.g., "conversation", "code", "document")
    pub content_type: String,

    /// Source of the content (e.g., "user_input", "llm_response", "file")
    pub source: String,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Custom metadata fields
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl MemoryMetadata {
    pub fn new(content_type: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            content_type: content_type.into(),
            source: source.into(),
            tags: Vec::new(),
            custom: HashMap::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn add_custom(&mut self, key: String, value: serde_json::Value) {
        self.custom.insert(key, value);
    }
}

/// A memory item stored in the context engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Metadata
    pub metadata: MemoryMetadata,

    /// Actual content
    pub content: String,

    /// Importance score (0.0 - 1.0)
    pub importance: f64,

    /// Current memory tier
    pub tier: MemoryTier,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last access timestamp
    pub last_accessed: DateTime<Utc>,

    /// Access count
    pub access_count: u64,

    /// Token count
    pub token_count: usize,

    /// Compressed version (if available)
    pub compressed_content: Option<String>,
}

impl MemoryItem {
    pub fn new(
        content: String,
        metadata: MemoryMetadata,
        importance: f64,
        token_count: usize,
    ) -> Self {
        let now = Utc::now();
        let tier = Self::select_tier(importance);

        Self {
            metadata,
            content,
            importance,
            tier,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            token_count,
            compressed_content: None,
        }
    }

    /// Select appropriate tier based on importance
    fn select_tier(importance: f64) -> MemoryTier {
        if importance >= MemoryTier::LongTerm.importance_threshold() {
            MemoryTier::LongTerm
        } else if importance >= MemoryTier::MediumTerm.importance_threshold() {
            MemoryTier::MediumTerm
        } else {
            MemoryTier::ShortTerm
        }
    }

    /// Update access statistics
    pub fn record_access(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count += 1;
    }

    /// Calculate current importance with time decay
    pub fn current_importance(&self) -> f64 {
        let age_seconds = (Utc::now() - self.created_at).num_seconds() as f64;
        let recency_seconds = (Utc::now() - self.last_accessed).num_seconds() as f64;

        let decay_rate = self.tier.decay_rate();
        let time_factor = (-decay_rate * recency_seconds / 3600.0).exp(); // Decay per hour

        // Access frequency boost
        let access_boost = (self.access_count as f64).ln_1p() * 0.1;

        (self.importance * time_factor + access_boost).min(1.0)
    }

    /// Check if item should be promoted to a higher tier
    pub fn should_promote(&self) -> Option<MemoryTier> {
        let current_importance = self.current_importance();

        match self.tier {
            MemoryTier::ShortTerm => {
                if current_importance >= MemoryTier::MediumTerm.importance_threshold() {
                    Some(MemoryTier::MediumTerm)
                } else {
                    None
                }
            }
            MemoryTier::MediumTerm => {
                if current_importance >= MemoryTier::LongTerm.importance_threshold() {
                    Some(MemoryTier::LongTerm)
                } else {
                    None
                }
            }
            MemoryTier::LongTerm => None,
        }
    }

    /// Check if item should be demoted to a lower tier
    pub fn should_demote(&self) -> Option<MemoryTier> {
        let current_importance = self.current_importance();

        match self.tier {
            MemoryTier::LongTerm => {
                if current_importance < MemoryTier::MediumTerm.importance_threshold() {
                    Some(MemoryTier::MediumTerm)
                } else {
                    None
                }
            }
            MemoryTier::MediumTerm => {
                if current_importance < MemoryTier::ShortTerm.importance_threshold() {
                    Some(MemoryTier::ShortTerm)
                } else {
                    None
                }
            }
            MemoryTier::ShortTerm => None,
        }
    }

    /// Get content (compressed or original)
    pub fn get_content(&self) -> &str {
        self.compressed_content.as_deref().unwrap_or(&self.content)
    }
}

/// Importance scoring algorithm
pub struct ImportanceScorer;

impl ImportanceScorer {
    /// Calculate importance score based on multiple factors
    pub fn score(
        content: &str,
        content_type: &str,
        source: &str,
        context: &HashMap<String, f64>,
    ) -> f64 {
        let mut score = 0.0;

        // Base score by content type
        score += match content_type {
            "error" | "exception" => 0.9,
            "user_query" => 0.8,
            "llm_response" => 0.7,
            "code" => 0.6,
            "documentation" => 0.5,
            "log" => 0.3,
            _ => 0.4,
        };

        // Source importance
        score += match source {
            "user_input" => 0.3,
            "llm_output" => 0.2,
            "system" => 0.25,
            _ => 0.1,
        };

        // Content-based signals
        if content.contains("error") || content.contains("ERROR") {
            score += 0.2;
        }
        if content.contains("TODO") || content.contains("FIXME") {
            score += 0.15;
        }
        if content.len() > 1000 {
            score += 0.1; // Longer content is often more important
        }

        // Context-based adjustment
        if let Some(custom_score) = context.get("custom_importance") {
            score += custom_score * 0.2;
        }

        score.min(1.0)
    }

    /// Calculate importance for a conversation turn
    pub fn score_conversation(
        role: &str,
        content: &str,
        has_code: bool,
        has_error: bool,
    ) -> f64 {
        let mut score: f64 = match role {
            "user" => 0.7,
            "assistant" => 0.6,
            "system" => 0.5,
            _ => 0.4,
        };

        if has_code {
            score += 0.2;
        }
        if has_error {
            score += 0.2;
        }
        if content.len() > 500 {
            score += 0.1;
        }

        score.min(1.0)
    }
}

/// Trait for tier-specific storage backends
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Store a memory item
    async fn store(&mut self, item: MemoryItem) -> Result<()>;

    /// Retrieve a memory item by ID
    async fn retrieve(&self, id: &Uuid) -> Result<Option<MemoryItem>>;

    /// List all items in the store
    async fn list(&self) -> Result<Vec<MemoryItem>>;

    /// Remove an item by ID
    async fn remove(&mut self, id: &Uuid) -> Result<()>;

    /// Update an existing item
    async fn update(&mut self, item: MemoryItem) -> Result<()>;

    /// Get total token count in store
    async fn total_tokens(&self) -> Result<usize>;

    /// Clear all items from store
    async fn clear(&mut self) -> Result<()>;

    /// Get items by tier
    async fn get_by_tier(&self, tier: MemoryTier) -> Result<Vec<MemoryItem>>;

    /// Evict items to free up space
    async fn evict(&mut self, target_tokens: usize) -> Result<Vec<MemoryItem>>;
}

/// In-memory implementation of MemoryStore
pub struct InMemoryStore {
    items: HashMap<Uuid, MemoryItem>,
    tier: MemoryTier,
}

impl InMemoryStore {
    pub fn new(tier: MemoryTier) -> Self {
        Self {
            items: HashMap::new(),
            tier,
        }
    }
}

#[async_trait]
impl MemoryStore for InMemoryStore {
    async fn store(&mut self, item: MemoryItem) -> Result<()> {
        self.items.insert(item.metadata.id, item);
        Ok(())
    }

    async fn retrieve(&self, id: &Uuid) -> Result<Option<MemoryItem>> {
        Ok(self.items.get(id).cloned())
    }

    async fn list(&self) -> Result<Vec<MemoryItem>> {
        Ok(self.items.values().cloned().collect())
    }

    async fn remove(&mut self, id: &Uuid) -> Result<()> {
        self.items.remove(id);
        Ok(())
    }

    async fn update(&mut self, item: MemoryItem) -> Result<()> {
        self.items.insert(item.metadata.id, item);
        Ok(())
    }

    async fn total_tokens(&self) -> Result<usize> {
        Ok(self.items.values().map(|item| item.token_count).sum())
    }

    async fn clear(&mut self) -> Result<()> {
        self.items.clear();
        Ok(())
    }

    async fn get_by_tier(&self, tier: MemoryTier) -> Result<Vec<MemoryItem>> {
        Ok(self
            .items
            .values()
            .filter(|item| item.tier == tier)
            .cloned()
            .collect())
    }

    async fn evict(&mut self, target_tokens: usize) -> Result<Vec<MemoryItem>> {
        let current_tokens = self.total_tokens().await?;
        if current_tokens <= target_tokens {
            return Ok(Vec::new());
        }

        let mut items: Vec<_> = self.items.values().cloned().collect();
        items.sort_by(|a, b| {
            a.current_importance()
                .partial_cmp(&b.current_importance())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut evicted = Vec::new();
        let mut freed_tokens = 0;
        let tokens_to_free = current_tokens - target_tokens;

        for item in items {
            if freed_tokens >= tokens_to_free {
                break;
            }
            freed_tokens += item.token_count;
            self.items.remove(&item.metadata.id);
            evicted.push(item);
        }

        Ok(evicted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tier_capacity() {
        assert_eq!(MemoryTier::ShortTerm.token_capacity(), 10_000);
        assert_eq!(MemoryTier::MediumTerm.token_capacity(), 50_000);
        assert_eq!(MemoryTier::LongTerm.token_capacity(), 140_000);
    }

    #[test]
    fn test_tier_selection() {
        let item = MemoryItem::new(
            "test".to_string(),
            MemoryMetadata::new("test", "test"),
            0.8,
            100,
        );
        assert_eq!(item.tier, MemoryTier::LongTerm);
    }

    #[test]
    fn test_importance_decay() {
        let mut item = MemoryItem::new(
            "test".to_string(),
            MemoryMetadata::new("test", "test"),
            0.9,
            100,
        );

        let initial = item.current_importance();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let after = item.current_importance();

        assert!(after <= initial);
    }

    #[tokio::test]
    async fn test_in_memory_store() {
        let mut store = InMemoryStore::new(MemoryTier::ShortTerm);
        let item = MemoryItem::new(
            "test content".to_string(),
            MemoryMetadata::new("test", "test"),
            0.5,
            100,
        );
        let id = item.metadata.id;

        store.store(item.clone()).await.unwrap();
        let retrieved = store.retrieve(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "test content");
    }
}
