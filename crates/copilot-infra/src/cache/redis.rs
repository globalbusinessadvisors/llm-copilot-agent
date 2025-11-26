use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

use copilot_core::cache::Cache;
use crate::{InfraError, Result};

#[derive(Debug, Clone)]
pub struct RedisCacheConfig {
    pub url: String,
    pub default_ttl: Option<Duration>,
    pub key_prefix: Option<String>,
}

impl Default for RedisCacheConfig {
    fn default() -> Self {
        Self {
            url: String::from("redis://127.0.0.1:6379"),
            default_ttl: Some(Duration::from_secs(3600)),
            key_prefix: Some(String::from("copilot:")),
        }
    }
}

impl RedisCacheConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn with_default_ttl(mut self, ttl: Option<Duration>) -> Self {
        self.default_ttl = ttl;
        self
    }

    pub fn with_key_prefix(mut self, prefix: Option<String>) -> Self {
        self.key_prefix = prefix;
        self
    }
}

#[derive(Clone)]
pub struct RedisCache {
    connection: ConnectionManager,
    config: RedisCacheConfig,
}

impl RedisCache {
    pub async fn new(config: RedisCacheConfig) -> Result<Self> {
        info!("Connecting to Redis at {}", config.url);

        let client = Client::open(config.url.clone())
            .map_err(|e| InfraError::Cache(e))?;

        let connection = ConnectionManager::new(client)
            .await
            .map_err(|e| InfraError::Cache(e))?;

        info!("Redis connection established");

        Ok(Self { connection, config })
    }

    fn make_key(&self, key: &str) -> String {
        match &self.config.key_prefix {
            Some(prefix) => format!("{}{}", prefix, key),
            None => key.to_string(),
        }
    }

    pub async fn set_with_ttl<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<()> {
        let full_key = self.make_key(key);
        debug!("Setting cache key: {} with TTL: {:?}", full_key, ttl);

        let serialized = serde_json::to_string(value)?;
        let ttl_secs = ttl.as_secs();

        let mut conn = self.connection.clone();
        let _: () = conn.set_ex(&full_key, serialized, ttl_secs)
            .await
            .map_err(|e| InfraError::Cache(e))?;

        Ok(())
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let full_key = self.make_key(key);
        debug!("Checking existence of cache key: {}", full_key);

        let mut conn = self.connection.clone();
        let exists: bool = conn.exists(&full_key)
            .await
            .map_err(|e| InfraError::Cache(e))?;

        Ok(exists)
    }

    pub async fn ttl(&self, key: &str) -> Result<Option<Duration>> {
        let full_key = self.make_key(key);
        debug!("Getting TTL for cache key: {}", full_key);

        let mut conn = self.connection.clone();
        let ttl_secs: i64 = conn.ttl(&full_key)
            .await
            .map_err(|e| InfraError::Cache(e))?;

        match ttl_secs {
            -2 => Ok(None), // Key doesn't exist
            -1 => Ok(None), // Key exists but has no expiry
            secs if secs > 0 => Ok(Some(Duration::from_secs(secs as u64))),
            _ => Ok(None),
        }
    }

    pub async fn increment(&self, key: &str, delta: i64) -> Result<i64> {
        let full_key = self.make_key(key);
        debug!("Incrementing cache key: {} by {}", full_key, delta);

        let mut conn = self.connection.clone();
        let new_value: i64 = conn.incr(&full_key, delta)
            .await
            .map_err(|e| InfraError::Cache(e))?;

        Ok(new_value)
    }

    pub async fn delete_pattern(&self, pattern: &str) -> Result<u64> {
        let full_pattern = self.make_key(pattern);
        debug!("Deleting keys matching pattern: {}", full_pattern);

        let mut conn = self.connection.clone();

        // Get all keys matching the pattern
        let keys: Vec<String> = conn.keys(&full_pattern)
            .await
            .map_err(|e| InfraError::Cache(e))?;

        if keys.is_empty() {
            return Ok(0);
        }

        // Delete all matching keys
        let count = keys.len() as u64;
        for key in keys {
            let _: () = conn.del(&key)
                .await
                .map_err(|e| InfraError::Cache(e))?;
        }

        info!("Deleted {} keys matching pattern: {}", count, full_pattern);
        Ok(count)
    }

    pub async fn health_check(&self) -> Result<()> {
        debug!("Performing Redis health check");

        let mut conn = self.connection.clone();
        let _: () = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                warn!("Redis health check failed: {}", e);
                InfraError::HealthCheck(format!("Redis health check failed: {}", e))
            })?;

        Ok(())
    }
}

#[async_trait]
impl Cache for RedisCache {
    type Error = InfraError;

    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let full_key = self.make_key(key);
        debug!("Getting cache key: {}", full_key);

        let mut conn = self.connection.clone();
        let value: Option<String> = conn.get(&full_key)
            .await
            .map_err(|e| InfraError::Cache(e))?;

        match value {
            Some(v) => {
                let deserialized = serde_json::from_str(&v)?;
                Ok(Some(deserialized))
            }
            None => Ok(None),
        }
    }

    async fn set<T: Serialize + Sync>(&self, key: &str, value: &T) -> Result<()> {
        let ttl = self.config.default_ttl.unwrap_or(Duration::from_secs(3600));
        self.set_with_ttl(key, value, ttl).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let full_key = self.make_key(key);
        debug!("Deleting cache key: {}", full_key);

        let mut conn = self.connection.clone();
        let _: () = conn.del(&full_key)
            .await
            .map_err(|e| InfraError::Cache(e))?;

        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        warn!("Clearing all cache keys with prefix");

        match &self.config.key_prefix {
            Some(prefix) => {
                let pattern = format!("{}*", prefix);
                self.delete_pattern(&pattern).await?;
            }
            None => {
                warn!("No key prefix configured, cannot safely clear cache");
                return Err(InfraError::Configuration(
                    "Cannot clear cache without key prefix".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function for testing key generation without needing RedisCache instance
    fn make_key_with_config(config: &RedisCacheConfig, key: &str) -> String {
        match &config.key_prefix {
            Some(prefix) => format!("{}{}", prefix, key),
            None => key.to_string(),
        }
    }

    #[test]
    fn test_config_builder() {
        let config = RedisCacheConfig::new("redis://localhost:6379")
            .with_default_ttl(Some(Duration::from_secs(300)))
            .with_key_prefix(Some("test:".to_string()));

        assert_eq!(config.url, "redis://localhost:6379");
        assert_eq!(config.default_ttl, Some(Duration::from_secs(300)));
        assert_eq!(config.key_prefix, Some("test:".to_string()));
    }

    #[test]
    fn test_make_key_with_prefix() {
        let config = RedisCacheConfig::new("redis://localhost")
            .with_key_prefix(Some("app:".to_string()));

        assert_eq!(make_key_with_config(&config, "session"), "app:session");
    }

    #[test]
    fn test_make_key_without_prefix() {
        let config = RedisCacheConfig::new("redis://localhost")
            .with_key_prefix(None);

        assert_eq!(make_key_with_config(&config, "session"), "session");
    }
}
