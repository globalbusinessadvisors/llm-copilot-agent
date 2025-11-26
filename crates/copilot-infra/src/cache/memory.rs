//! In-memory cache implementation
//!
//! Provides a fast, local cache for development and testing,
//! with optional TTL support.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

use copilot_core::cache::Cache;
use crate::{InfraError, Result};

/// Entry in the memory cache
#[derive(Clone)]
struct CacheEntry {
    /// Serialized value
    value: String,
    /// When this entry expires (None for no expiration)
    expires_at: Option<Instant>,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.expires_at.map(|exp| Instant::now() > exp).unwrap_or(false)
    }
}

/// Configuration for the memory cache
#[derive(Debug, Clone)]
pub struct MemoryCacheConfig {
    /// Default TTL for entries
    pub default_ttl: Option<Duration>,
    /// Maximum number of entries
    pub max_entries: usize,
    /// Whether to clean up expired entries periodically
    pub cleanup_interval: Option<Duration>,
}

impl Default for MemoryCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Some(Duration::from_secs(300)), // 5 minutes
            max_entries: 10_000,
            cleanup_interval: Some(Duration::from_secs(60)),
        }
    }
}

impl MemoryCacheConfig {
    /// Create a new config with default TTL
    pub fn new(default_ttl: Option<Duration>) -> Self {
        Self {
            default_ttl,
            ..Default::default()
        }
    }

    /// Set max entries
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Set cleanup interval
    pub fn with_cleanup_interval(mut self, interval: Option<Duration>) -> Self {
        self.cleanup_interval = interval;
        self
    }
}

/// In-memory cache implementation
#[derive(Clone)]
pub struct MemoryCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    config: MemoryCacheConfig,
}

impl MemoryCache {
    /// Create a new memory cache
    pub fn new(config: MemoryCacheConfig) -> Self {
        let cache = Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            config,
        };

        // Start cleanup task if configured
        if let Some(interval) = cache.config.cleanup_interval {
            let entries = Arc::clone(&cache.entries);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(interval).await;
                    let mut entries = entries.write().await;
                    let before = entries.len();
                    entries.retain(|_, v| !v.is_expired());
                    let removed = before - entries.len();
                    if removed > 0 {
                        debug!("Memory cache cleanup: removed {} expired entries", removed);
                    }
                }
            });
        }

        cache
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(MemoryCacheConfig::default())
    }

    /// Set a value with a specific TTL
    pub async fn set_with_ttl<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        let entry = CacheEntry {
            value: serialized,
            expires_at: Some(Instant::now() + ttl),
        };

        let mut entries = self.entries.write().await;

        // Evict if at capacity
        if entries.len() >= self.config.max_entries && !entries.contains_key(key) {
            self.evict_one(&mut entries);
        }

        entries.insert(key.to_string(), entry);
        debug!("Set cache key: {} with TTL: {:?}", key, ttl);
        Ok(())
    }

    /// Check if a key exists and is not expired
    pub async fn exists(&self, key: &str) -> bool {
        let entries = self.entries.read().await;
        entries.get(key).map(|e| !e.is_expired()).unwrap_or(false)
    }

    /// Get remaining TTL for a key
    pub async fn ttl(&self, key: &str) -> Option<Duration> {
        let entries = self.entries.read().await;
        entries.get(key).and_then(|e| {
            e.expires_at.and_then(|exp| {
                let now = Instant::now();
                if exp > now {
                    Some(exp - now)
                } else {
                    None
                }
            })
        })
    }

    /// Get the number of entries in the cache
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }

    /// Evict one entry (using simple random eviction)
    fn evict_one(&self, entries: &mut HashMap<String, CacheEntry>) {
        // First try to evict an expired entry
        let expired_key = entries
            .iter()
            .find(|(_, v)| v.is_expired())
            .map(|(k, _)| k.clone());

        if let Some(key) = expired_key {
            entries.remove(&key);
            return;
        }

        // Otherwise evict any entry
        if let Some(key) = entries.keys().next().cloned() {
            entries.remove(&key);
        }
    }
}

#[async_trait]
impl Cache for MemoryCache {
    type Error = InfraError;

    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let entries = self.entries.read().await;

        match entries.get(key) {
            Some(entry) if !entry.is_expired() => {
                let value: T = serde_json::from_str(&entry.value)?;
                debug!("Cache hit: {}", key);
                Ok(Some(value))
            }
            _ => {
                debug!("Cache miss: {}", key);
                Ok(None)
            }
        }
    }

    async fn set<T: Serialize + Sync>(&self, key: &str, value: &T) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        let entry = CacheEntry {
            value: serialized,
            expires_at: self.config.default_ttl.map(|ttl| Instant::now() + ttl),
        };

        let mut entries = self.entries.write().await;

        // Evict if at capacity
        if entries.len() >= self.config.max_entries && !entries.contains_key(key) {
            self.evict_one(&mut entries);
        }

        entries.insert(key.to_string(), entry);
        debug!("Set cache key: {}", key);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut entries = self.entries.write().await;
        entries.remove(key);
        debug!("Deleted cache key: {}", key);
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        let mut entries = self.entries.write().await;
        entries.clear();
        debug!("Cleared all cache entries");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_and_get() {
        let cache = MemoryCache::new(MemoryCacheConfig {
            cleanup_interval: None,
            ..Default::default()
        });

        cache.set("key1", &"value1".to_string()).await.unwrap();
        let result: Option<String> = cache.get("key1").await.unwrap();

        assert_eq!(result, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_get_missing_key() {
        let cache = MemoryCache::new(MemoryCacheConfig {
            cleanup_interval: None,
            ..Default::default()
        });

        let result: Option<String> = cache.get("missing").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_delete() {
        let cache = MemoryCache::new(MemoryCacheConfig {
            cleanup_interval: None,
            ..Default::default()
        });

        cache.set("key1", &42i32).await.unwrap();
        cache.delete("key1").await.unwrap();

        let result: Option<i32> = cache.get("key1").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = MemoryCache::new(MemoryCacheConfig {
            cleanup_interval: None,
            ..Default::default()
        });

        cache.set("key1", &1i32).await.unwrap();
        cache.set("key2", &2i32).await.unwrap();

        cache.clear().await.unwrap();

        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache = MemoryCache::new(MemoryCacheConfig {
            default_ttl: None,
            cleanup_interval: None,
            ..Default::default()
        });

        cache
            .set_with_ttl("key1", &"value1".to_string(), Duration::from_millis(10))
            .await
            .unwrap();

        // Should exist initially
        assert!(cache.exists("key1").await);

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should be expired
        let result: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_max_entries() {
        let cache = MemoryCache::new(MemoryCacheConfig {
            max_entries: 2,
            cleanup_interval: None,
            ..Default::default()
        });

        cache.set("key1", &1i32).await.unwrap();
        cache.set("key2", &2i32).await.unwrap();
        cache.set("key3", &3i32).await.unwrap(); // Should evict one

        assert!(cache.len().await <= 2);
    }

    #[tokio::test]
    async fn test_exists() {
        let cache = MemoryCache::new(MemoryCacheConfig {
            cleanup_interval: None,
            ..Default::default()
        });

        cache.set("key1", &42i32).await.unwrap();

        assert!(cache.exists("key1").await);
        assert!(!cache.exists("key2").await);
    }

    #[tokio::test]
    async fn test_ttl() {
        let cache = MemoryCache::new(MemoryCacheConfig {
            default_ttl: None,
            cleanup_interval: None,
            ..Default::default()
        });

        cache
            .set_with_ttl("key1", &42i32, Duration::from_secs(100))
            .await
            .unwrap();

        let ttl = cache.ttl("key1").await;
        assert!(ttl.is_some());
        assert!(ttl.unwrap() > Duration::from_secs(90));
    }
}
