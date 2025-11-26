//! Response caching for HTTP endpoints
//!
//! Provides response caching with configurable TTL and cache key strategies.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info};

use crate::Result;

/// Cached response entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    /// Response status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
    /// Content type
    pub content_type: Option<String>,
    /// ETag for conditional requests
    pub etag: Option<String>,
    /// When this response was cached
    pub cached_at: i64,
}

impl CachedResponse {
    /// Create a new cached response
    pub fn new(status: u16, headers: HashMap<String, String>, body: Vec<u8>) -> Self {
        let content_type = headers.get("content-type").cloned();
        let etag = Self::compute_etag(&body);

        Self {
            status,
            headers,
            body,
            content_type,
            etag: Some(etag),
            cached_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Compute ETag for the body
    fn compute_etag(body: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(body);
        let result = hasher.finalize();
        format!("\"{}\"", hex::encode(&result[..8]))
    }
}

/// Configuration for response caching
#[derive(Debug, Clone)]
pub struct ResponseCacheConfig {
    /// Default TTL for cached responses
    pub default_ttl: Duration,
    /// Maximum body size to cache (in bytes)
    pub max_body_size: usize,
    /// Whether to cache 4xx responses
    pub cache_client_errors: bool,
    /// Whether to include query params in cache key
    pub include_query_params: bool,
    /// Methods to cache (defaults to GET only)
    pub cacheable_methods: Vec<String>,
    /// Status codes to cache
    pub cacheable_statuses: Vec<u16>,
}

impl Default for ResponseCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(300), // 5 minutes
            max_body_size: 1024 * 1024,            // 1 MB
            cache_client_errors: false,
            include_query_params: true,
            cacheable_methods: vec!["GET".to_string(), "HEAD".to_string()],
            cacheable_statuses: vec![200, 203, 204, 206, 300, 301, 404, 405, 410, 414, 501],
        }
    }
}

impl ResponseCacheConfig {
    /// Create a new config with default TTL
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            default_ttl,
            ..Default::default()
        }
    }

    /// Create an aggressive caching config
    pub fn aggressive() -> Self {
        Self {
            default_ttl: Duration::from_secs(3600), // 1 hour
            cache_client_errors: true,
            ..Default::default()
        }
    }

    /// Create a conservative caching config
    pub fn conservative() -> Self {
        Self {
            default_ttl: Duration::from_secs(60), // 1 minute
            max_body_size: 256 * 1024,            // 256 KB
            cache_client_errors: false,
            ..Default::default()
        }
    }

    /// Set max body size
    pub fn with_max_body_size(mut self, size: usize) -> Self {
        self.max_body_size = size;
        self
    }

    /// Set whether to cache client errors
    pub fn with_cache_client_errors(mut self, cache: bool) -> Self {
        self.cache_client_errors = cache;
        self
    }

    /// Set whether to include query params in cache key
    pub fn with_include_query_params(mut self, include: bool) -> Self {
        self.include_query_params = include;
        self
    }

    /// Check if a method is cacheable
    pub fn is_method_cacheable(&self, method: &str) -> bool {
        self.cacheable_methods.iter().any(|m| m.eq_ignore_ascii_case(method))
    }

    /// Check if a status code is cacheable
    pub fn is_status_cacheable(&self, status: u16) -> bool {
        self.cacheable_statuses.contains(&status)
    }
}

/// Cache key builder for HTTP requests
#[derive(Debug, Clone)]
pub struct CacheKeyBuilder {
    /// Request method
    method: String,
    /// Request path
    path: String,
    /// Query parameters
    query: Option<String>,
    /// Headers to include in the cache key
    vary_headers: HashMap<String, String>,
    /// User ID (for per-user caching)
    user_id: Option<String>,
    /// Tenant ID (for multi-tenant caching)
    tenant_id: Option<String>,
}

impl CacheKeyBuilder {
    /// Create a new cache key builder
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_uppercase(),
            path: path.to_string(),
            query: None,
            vary_headers: HashMap::new(),
            user_id: None,
            tenant_id: None,
        }
    }

    /// Add query parameters
    pub fn with_query(mut self, query: Option<&str>) -> Self {
        self.query = query.map(String::from);
        self
    }

    /// Add a vary header
    pub fn with_vary_header(mut self, name: &str, value: &str) -> Self {
        self.vary_headers.insert(name.to_lowercase(), value.to_string());
        self
    }

    /// Add user ID for per-user caching
    pub fn with_user_id(mut self, user_id: Option<&str>) -> Self {
        self.user_id = user_id.map(String::from);
        self
    }

    /// Add tenant ID for multi-tenant caching
    pub fn with_tenant_id(mut self, tenant_id: Option<&str>) -> Self {
        self.tenant_id = tenant_id.map(String::from);
        self
    }

    /// Build the cache key
    pub fn build(&self) -> String {
        let mut parts = vec![
            format!("response:{}:{}", self.method, self.path),
        ];

        if let Some(ref query) = self.query {
            parts.push(format!("q:{}", query));
        }

        if let Some(ref user_id) = self.user_id {
            parts.push(format!("u:{}", user_id));
        }

        if let Some(ref tenant_id) = self.tenant_id {
            parts.push(format!("t:{}", tenant_id));
        }

        // Add sorted vary headers
        let mut vary_parts: Vec<_> = self
            .vary_headers
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        vary_parts.sort();

        if !vary_parts.is_empty() {
            parts.push(format!("v:{}", vary_parts.join("&")));
        }

        parts.join(":")
    }

    /// Build a hashed cache key (for long keys)
    pub fn build_hashed(&self) -> String {
        let full_key = self.build();
        if full_key.len() > 200 {
            let mut hasher = Sha256::new();
            hasher.update(full_key.as_bytes());
            let result = hasher.finalize();
            format!("response:h:{}", hex::encode(result))
        } else {
            full_key
        }
    }
}

/// Response cache trait
#[async_trait]
pub trait ResponseCache: Send + Sync {
    /// Get a cached response
    async fn get_response(&self, key: &str) -> Result<Option<CachedResponse>>;

    /// Cache a response
    async fn set_response(&self, key: &str, response: &CachedResponse, ttl: Duration) -> Result<()>;

    /// Invalidate a cached response
    async fn invalidate(&self, key: &str) -> Result<()>;

    /// Invalidate all responses matching a pattern
    async fn invalidate_pattern(&self, pattern: &str) -> Result<u64>;
}

/// Cache control parser for HTTP Cache-Control headers
#[derive(Debug, Clone, Default)]
pub struct CacheControl {
    /// Whether the response is public
    pub public: bool,
    /// Whether the response is private
    pub private: bool,
    /// No-cache directive
    pub no_cache: bool,
    /// No-store directive
    pub no_store: bool,
    /// Max-age in seconds
    pub max_age: Option<u64>,
    /// S-maxage in seconds (shared cache)
    pub s_maxage: Option<u64>,
    /// Must-revalidate directive
    pub must_revalidate: bool,
    /// Immutable directive
    pub immutable: bool,
}

impl CacheControl {
    /// Parse a Cache-Control header value
    pub fn parse(header: &str) -> Self {
        let mut cc = CacheControl::default();

        for directive in header.split(',').map(str::trim) {
            let parts: Vec<&str> = directive.splitn(2, '=').collect();
            let name = parts[0].to_lowercase();
            let value = parts.get(1).map(|v| v.trim_matches('"'));

            match name.as_str() {
                "public" => cc.public = true,
                "private" => cc.private = true,
                "no-cache" => cc.no_cache = true,
                "no-store" => cc.no_store = true,
                "must-revalidate" => cc.must_revalidate = true,
                "immutable" => cc.immutable = true,
                "max-age" => cc.max_age = value.and_then(|v| v.parse().ok()),
                "s-maxage" => cc.s_maxage = value.and_then(|v| v.parse().ok()),
                _ => {}
            }
        }

        cc
    }

    /// Check if caching is allowed
    pub fn is_cacheable(&self) -> bool {
        !self.no_store && !self.no_cache
    }

    /// Get the effective TTL
    pub fn effective_ttl(&self) -> Option<Duration> {
        self.s_maxage
            .or(self.max_age)
            .map(Duration::from_secs)
    }

    /// Generate a Cache-Control header value
    pub fn to_header(&self) -> String {
        let mut parts = Vec::new();

        if self.public {
            parts.push("public".to_string());
        }
        if self.private {
            parts.push("private".to_string());
        }
        if self.no_cache {
            parts.push("no-cache".to_string());
        }
        if self.no_store {
            parts.push("no-store".to_string());
        }
        if let Some(max_age) = self.max_age {
            parts.push(format!("max-age={}", max_age));
        }
        if let Some(s_maxage) = self.s_maxage {
            parts.push(format!("s-maxage={}", s_maxage));
        }
        if self.must_revalidate {
            parts.push("must-revalidate".to_string());
        }
        if self.immutable {
            parts.push("immutable".to_string());
        }

        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_builder() {
        let key = CacheKeyBuilder::new("GET", "/api/users")
            .with_query(Some("page=1"))
            .with_user_id(Some("user-123"))
            .build();

        assert!(key.contains("GET"));
        assert!(key.contains("/api/users"));
        assert!(key.contains("page=1"));
        assert!(key.contains("user-123"));
    }

    #[test]
    fn test_cache_key_hashing() {
        let builder = CacheKeyBuilder::new("GET", "/api/users")
            .with_query(Some(&"x".repeat(500)));

        let hashed = builder.build_hashed();
        assert!(hashed.len() < 200);
        assert!(hashed.starts_with("response:h:"));
    }

    #[test]
    fn test_cache_control_parse() {
        let cc = CacheControl::parse("public, max-age=300, s-maxage=600");

        assert!(cc.public);
        assert!(!cc.private);
        assert_eq!(cc.max_age, Some(300));
        assert_eq!(cc.s_maxage, Some(600));
        assert!(cc.is_cacheable());
    }

    #[test]
    fn test_cache_control_no_store() {
        let cc = CacheControl::parse("no-store, no-cache");

        assert!(cc.no_store);
        assert!(cc.no_cache);
        assert!(!cc.is_cacheable());
    }

    #[test]
    fn test_cache_control_effective_ttl() {
        let cc = CacheControl::parse("max-age=100, s-maxage=200");
        assert_eq!(cc.effective_ttl(), Some(Duration::from_secs(200)));

        let cc2 = CacheControl::parse("max-age=100");
        assert_eq!(cc2.effective_ttl(), Some(Duration::from_secs(100)));
    }

    #[test]
    fn test_cache_control_to_header() {
        let cc = CacheControl {
            public: true,
            max_age: Some(300),
            must_revalidate: true,
            ..Default::default()
        };

        let header = cc.to_header();
        assert!(header.contains("public"));
        assert!(header.contains("max-age=300"));
        assert!(header.contains("must-revalidate"));
    }

    #[test]
    fn test_cached_response() {
        let response = CachedResponse::new(
            200,
            HashMap::from([("content-type".to_string(), "application/json".to_string())]),
            b"{}".to_vec(),
        );

        assert_eq!(response.status, 200);
        assert!(response.etag.is_some());
        assert_eq!(response.content_type, Some("application/json".to_string()));
    }

    #[test]
    fn test_config_methods() {
        let config = ResponseCacheConfig::default();

        assert!(config.is_method_cacheable("GET"));
        assert!(config.is_method_cacheable("get"));
        assert!(config.is_method_cacheable("HEAD"));
        assert!(!config.is_method_cacheable("POST"));
        assert!(!config.is_method_cacheable("PUT"));
    }

    #[test]
    fn test_config_statuses() {
        let config = ResponseCacheConfig::default();

        assert!(config.is_status_cacheable(200));
        assert!(config.is_status_cacheable(301));
        assert!(!config.is_status_cacheable(500));
        assert!(!config.is_status_cacheable(401));
    }
}
