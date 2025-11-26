//! Cache abstraction for key-value storage operations.

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

/// Generic cache trait for key-value storage operations.
///
/// This trait provides a flexible cache abstraction that can be implemented
/// by various cache backends (Redis, in-memory, etc.).
#[async_trait]
pub trait Cache: Send + Sync {
    /// Error type returned by cache operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get a value from the cache
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Self::Error>;

    /// Set a value in the cache
    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<(), Self::Error>
    where
        Self: Sync;

    /// Delete a value from the cache
    async fn delete(&self, key: &str) -> Result<(), Self::Error>;

    /// Clear all values from the cache
    async fn clear(&self) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockCache {
        data: Mutex<HashMap<String, String>>,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("Mock cache error")]
    struct MockError;

    #[async_trait]
    impl Cache for MockCache {
        type Error = MockError;

        async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Self::Error> {
            let data = self.data.lock().unwrap();
            match data.get(key) {
                Some(v) => Ok(serde_json::from_str(v).ok()),
                None => Ok(None),
            }
        }

        async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<(), Self::Error> {
            let mut data = self.data.lock().unwrap();
            data.insert(key.to_string(), serde_json::to_string(value).unwrap());
            Ok(())
        }

        async fn delete(&self, key: &str) -> Result<(), Self::Error> {
            let mut data = self.data.lock().unwrap();
            data.remove(key);
            Ok(())
        }

        async fn clear(&self) -> Result<(), Self::Error> {
            let mut data = self.data.lock().unwrap();
            data.clear();
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_cache() {
        let cache = MockCache {
            data: Mutex::new(HashMap::new()),
        };

        cache.set("key", &"value").await.unwrap();
        let result: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(result, Some("value".to_string()));

        cache.delete("key").await.unwrap();
        let result: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(result, None);
    }
}
