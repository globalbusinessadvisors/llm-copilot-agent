//! API Key management
//!
//! Provides API key generation, validation, and scope management.

use crate::error::{Result, SecurityError};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use uuid::Uuid;

/// API key prefix for identification
pub const API_KEY_PREFIX: &str = "cplt";

/// API key version
pub const API_KEY_VERSION: u8 = 1;

/// API key scopes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyScope {
    /// Full read access
    Read,
    /// Full write access
    Write,
    /// Chat/conversation access
    Chat,
    /// Workflow execution access
    Workflows,
    /// Context management access
    Context,
    /// Sandbox execution access
    Sandbox,
    /// Admin access
    Admin,
}

impl ApiKeyScope {
    /// Get scope from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "read" => Ok(ApiKeyScope::Read),
            "write" => Ok(ApiKeyScope::Write),
            "chat" => Ok(ApiKeyScope::Chat),
            "workflows" => Ok(ApiKeyScope::Workflows),
            "context" => Ok(ApiKeyScope::Context),
            "sandbox" => Ok(ApiKeyScope::Sandbox),
            "admin" => Ok(ApiKeyScope::Admin),
            _ => Err(SecurityError::InsufficientScope(format!(
                "Unknown scope: {}",
                s
            ))),
        }
    }

    /// Get scope as string
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiKeyScope::Read => "read",
            ApiKeyScope::Write => "write",
            ApiKeyScope::Chat => "chat",
            ApiKeyScope::Workflows => "workflows",
            ApiKeyScope::Context => "context",
            ApiKeyScope::Sandbox => "sandbox",
            ApiKeyScope::Admin => "admin",
        }
    }

    /// Parse scopes from a space-separated string
    pub fn parse_scopes(s: &str) -> HashSet<ApiKeyScope> {
        s.split_whitespace()
            .filter_map(|scope| ApiKeyScope::from_str(scope).ok())
            .collect()
    }

    /// Get default scopes for a new API key
    pub fn default_scopes() -> HashSet<ApiKeyScope> {
        let mut scopes = HashSet::new();
        scopes.insert(ApiKeyScope::Read);
        scopes.insert(ApiKeyScope::Chat);
        scopes
    }

    /// Get all scopes (for admin keys)
    pub fn all_scopes() -> HashSet<ApiKeyScope> {
        let mut scopes = HashSet::new();
        scopes.insert(ApiKeyScope::Read);
        scopes.insert(ApiKeyScope::Write);
        scopes.insert(ApiKeyScope::Chat);
        scopes.insert(ApiKeyScope::Workflows);
        scopes.insert(ApiKeyScope::Context);
        scopes.insert(ApiKeyScope::Sandbox);
        scopes.insert(ApiKeyScope::Admin);
        scopes
    }
}

impl std::fmt::Display for ApiKeyScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// API key metadata stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyMetadata {
    /// Unique key ID
    pub id: String,
    /// Key name/description
    pub name: String,
    /// User ID who owns this key
    pub user_id: String,
    /// Tenant ID (for multi-tenant)
    pub tenant_id: Option<String>,
    /// Key prefix (for display, e.g., "cplt_v1_abc...")
    pub prefix: String,
    /// SHA-256 hash of the full key
    pub key_hash: String,
    /// Granted scopes
    pub scopes: HashSet<ApiKeyScope>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Expiration timestamp (None for non-expiring)
    pub expires_at: Option<DateTime<Utc>>,
    /// Last used timestamp
    pub last_used_at: Option<DateTime<Utc>>,
    /// Whether the key is active
    pub is_active: bool,
    /// Request count
    pub request_count: u64,
    /// Rate limit (requests per minute, None for default)
    pub rate_limit: Option<u32>,
}

impl ApiKeyMetadata {
    /// Check if the key is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if the key is valid (active and not expired)
    pub fn is_valid(&self) -> bool {
        self.is_active && !self.is_expired()
    }

    /// Check if the key has a specific scope
    pub fn has_scope(&self, scope: &ApiKeyScope) -> bool {
        self.scopes.contains(scope) || self.scopes.contains(&ApiKeyScope::Admin)
    }

    /// Check if the key has all required scopes
    pub fn has_all_scopes(&self, required: &[ApiKeyScope]) -> bool {
        required.iter().all(|s| self.has_scope(s))
    }
}

/// Generated API key (only returned once during creation)
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedApiKey {
    /// The full API key (only shown once)
    pub key: String,
    /// Key metadata
    pub metadata: ApiKeyMetadata,
}

/// API key manager
#[derive(Debug, Clone)]
pub struct ApiKeyManager {
    /// Default expiration in days (None for non-expiring)
    pub default_expiry_days: Option<i64>,
    /// Default rate limit
    pub default_rate_limit: u32,
}

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self {
            default_expiry_days: Some(365),  // 1 year
            default_rate_limit: 1000,        // 1000 requests per minute
        }
    }
}

impl ApiKeyManager {
    /// Create a new API key manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a new API key
    pub fn generate_key(
        &self,
        name: &str,
        user_id: &str,
        tenant_id: Option<String>,
        scopes: Option<HashSet<ApiKeyScope>>,
        expires_in_days: Option<i64>,
        rate_limit: Option<u32>,
    ) -> GeneratedApiKey {
        // Generate random bytes
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 32] = rng.gen();

        // Create the key ID
        let key_id = Uuid::new_v4().to_string();

        // Encode the random part
        let random_part = URL_SAFE_NO_PAD.encode(random_bytes);

        // Create the full key: cplt_v1_{random_part}
        let full_key = format!("{}_v{}_{}", API_KEY_PREFIX, API_KEY_VERSION, random_part);

        // Create the prefix (for display)
        let prefix = format!(
            "{}_v{}_{}...",
            API_KEY_PREFIX,
            API_KEY_VERSION,
            &random_part[..8]
        );

        // Hash the full key for storage
        let key_hash = self.hash_key(&full_key);

        // Determine expiration
        let expires_at = expires_in_days
            .or(self.default_expiry_days)
            .map(|days| Utc::now() + Duration::days(days));

        // Create metadata
        let metadata = ApiKeyMetadata {
            id: key_id,
            name: name.to_string(),
            user_id: user_id.to_string(),
            tenant_id,
            prefix,
            key_hash,
            scopes: scopes.unwrap_or_else(ApiKeyScope::default_scopes),
            created_at: Utc::now(),
            expires_at,
            last_used_at: None,
            is_active: true,
            request_count: 0,
            rate_limit: rate_limit.or(Some(self.default_rate_limit)),
        };

        GeneratedApiKey {
            key: full_key,
            metadata,
        }
    }

    /// Hash an API key for storage comparison
    pub fn hash_key(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Validate an API key format
    pub fn validate_key_format(&self, key: &str) -> Result<()> {
        // Check prefix
        if !key.starts_with(&format!("{}_v", API_KEY_PREFIX)) {
            return Err(SecurityError::InvalidApiKey);
        }

        // Check version
        let parts: Vec<&str> = key.splitn(3, '_').collect();
        if parts.len() != 3 {
            return Err(SecurityError::InvalidApiKey);
        }

        let version_str = parts[1].trim_start_matches('v');
        if version_str.parse::<u8>().is_err() {
            return Err(SecurityError::InvalidApiKey);
        }

        // Check random part length
        if parts[2].len() < 20 {
            return Err(SecurityError::InvalidApiKey);
        }

        Ok(())
    }

    /// Verify an API key against stored metadata
    pub fn verify_key(&self, key: &str, metadata: &ApiKeyMetadata) -> Result<()> {
        // Validate format
        self.validate_key_format(key)?;

        // Check if key matches
        let key_hash = self.hash_key(key);
        if key_hash != metadata.key_hash {
            return Err(SecurityError::InvalidApiKey);
        }

        // Check if active
        if !metadata.is_active {
            return Err(SecurityError::InvalidApiKey);
        }

        // Check expiration
        if metadata.is_expired() {
            return Err(SecurityError::ApiKeyExpired);
        }

        Ok(())
    }

    /// Check if a key has required scopes
    pub fn check_scopes(&self, metadata: &ApiKeyMetadata, required: &[ApiKeyScope]) -> Result<()> {
        for scope in required {
            if !metadata.has_scope(scope) {
                return Err(SecurityError::InsufficientScope(scope.to_string()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let manager = ApiKeyManager::new();
        let generated = manager.generate_key(
            "Test Key",
            "user-123",
            None,
            None,
            None,
            None,
        );

        assert!(generated.key.starts_with("cplt_v1_"));
        assert!(generated.metadata.prefix.starts_with("cplt_v1_"));
        assert!(generated.metadata.prefix.ends_with("..."));
        assert_eq!(generated.metadata.name, "Test Key");
        assert_eq!(generated.metadata.user_id, "user-123");
        assert!(generated.metadata.is_active);
    }

    #[test]
    fn test_key_hashing() {
        let manager = ApiKeyManager::new();
        let generated = manager.generate_key(
            "Test Key",
            "user-123",
            None,
            None,
            None,
            None,
        );

        let hash = manager.hash_key(&generated.key);
        assert_eq!(hash, generated.metadata.key_hash);

        // Different key should have different hash
        let generated2 = manager.generate_key(
            "Test Key 2",
            "user-123",
            None,
            None,
            None,
            None,
        );
        assert_ne!(generated.metadata.key_hash, generated2.metadata.key_hash);
    }

    #[test]
    fn test_validate_key_format() {
        let manager = ApiKeyManager::new();
        let generated = manager.generate_key(
            "Test Key",
            "user-123",
            None,
            None,
            None,
            None,
        );

        assert!(manager.validate_key_format(&generated.key).is_ok());
        assert!(manager.validate_key_format("invalid").is_err());
        assert!(manager.validate_key_format("cplt_invalid").is_err());
    }

    #[test]
    fn test_verify_key() {
        let manager = ApiKeyManager::new();
        let generated = manager.generate_key(
            "Test Key",
            "user-123",
            None,
            None,
            None,
            None,
        );

        assert!(manager.verify_key(&generated.key, &generated.metadata).is_ok());

        // Wrong key should fail
        let wrong_key = format!("{}_wrong", generated.key);
        assert!(manager.verify_key(&wrong_key, &generated.metadata).is_err());
    }

    #[test]
    fn test_scopes() {
        let manager = ApiKeyManager::new();

        let mut scopes = HashSet::new();
        scopes.insert(ApiKeyScope::Read);
        scopes.insert(ApiKeyScope::Chat);

        let generated = manager.generate_key(
            "Test Key",
            "user-123",
            None,
            Some(scopes),
            None,
            None,
        );

        assert!(generated.metadata.has_scope(&ApiKeyScope::Read));
        assert!(generated.metadata.has_scope(&ApiKeyScope::Chat));
        assert!(!generated.metadata.has_scope(&ApiKeyScope::Admin));

        assert!(manager.check_scopes(&generated.metadata, &[ApiKeyScope::Read]).is_ok());
        assert!(manager.check_scopes(&generated.metadata, &[ApiKeyScope::Admin]).is_err());
    }

    #[test]
    fn test_admin_scope_grants_all() {
        let manager = ApiKeyManager::new();

        let generated = manager.generate_key(
            "Admin Key",
            "admin-123",
            None,
            Some(ApiKeyScope::all_scopes()),
            None,
            None,
        );

        // Admin scope should grant access to everything
        assert!(generated.metadata.has_scope(&ApiKeyScope::Read));
        assert!(generated.metadata.has_scope(&ApiKeyScope::Write));
        assert!(generated.metadata.has_scope(&ApiKeyScope::Chat));
        assert!(generated.metadata.has_scope(&ApiKeyScope::Workflows));
    }

    #[test]
    fn test_expiration() {
        let manager = ApiKeyManager::new();

        // Non-expiring key
        let generated = manager.generate_key(
            "Test Key",
            "user-123",
            None,
            None,
            Some(-1),  // Special value for non-expiring
            None,
        );

        // Key with short expiry (already expired)
        let mut metadata = generated.metadata.clone();
        metadata.expires_at = Some(Utc::now() - Duration::hours(1));
        assert!(metadata.is_expired());
        assert!(!metadata.is_valid());
    }
}
