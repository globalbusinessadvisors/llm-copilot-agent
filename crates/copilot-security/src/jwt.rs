//! JWT token management
//!
//! Provides JWT token generation, validation, and refresh functionality.

use crate::error::{Result, SecurityError};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// User's email
    pub email: String,
    /// User's username
    pub username: String,
    /// User's roles
    pub roles: Vec<String>,
    /// Token ID (for revocation)
    pub jti: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Not before (Unix timestamp)
    pub nbf: i64,
    /// Token type (access or refresh)
    pub token_type: TokenType,
}

/// Token type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

/// Token pair containing access and refresh tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// Access token (short-lived)
    pub access_token: String,
    /// Refresh token (long-lived)
    pub refresh_token: String,
    /// Access token type (always "Bearer")
    pub token_type: String,
    /// Access token expiration in seconds
    pub expires_in: i64,
    /// Refresh token expiration in seconds
    pub refresh_expires_in: i64,
}

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens
    pub secret: String,
    /// Token issuer
    pub issuer: String,
    /// Token audience
    pub audience: String,
    /// Access token expiration in seconds
    pub access_token_expiry_secs: i64,
    /// Refresh token expiration in seconds
    pub refresh_token_expiry_secs: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "change-me-in-production".to_string(),
            issuer: "llm-copilot-agent".to_string(),
            audience: "copilot-api".to_string(),
            access_token_expiry_secs: 900,       // 15 minutes
            refresh_token_expiry_secs: 604800,   // 7 days
        }
    }
}

/// JWT token manager
#[derive(Clone)]
pub struct JwtManager {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl std::fmt::Debug for JwtManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtManager")
            .field("config", &self.config)
            .field("encoding_key", &"[REDACTED]")
            .field("decoding_key", &"[REDACTED]")
            .finish()
    }
}

impl JwtManager {
    /// Create a new JWT manager with the given configuration
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// Create a JWT manager from a secret string
    pub fn from_secret(secret: &str) -> Self {
        Self::new(JwtConfig {
            secret: secret.to_string(),
            ..Default::default()
        })
    }

    /// Generate a token pair for a user
    pub fn generate_token_pair(
        &self,
        user_id: &str,
        email: &str,
        username: &str,
        roles: Vec<String>,
    ) -> Result<TokenPair> {
        let access_token = self.generate_token(
            user_id,
            email,
            username,
            roles.clone(),
            TokenType::Access,
            self.config.access_token_expiry_secs,
        )?;

        let refresh_token = self.generate_token(
            user_id,
            email,
            username,
            roles,
            TokenType::Refresh,
            self.config.refresh_token_expiry_secs,
        )?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_expiry_secs,
            refresh_expires_in: self.config.refresh_token_expiry_secs,
        })
    }

    /// Generate a single token
    fn generate_token(
        &self,
        user_id: &str,
        email: &str,
        username: &str,
        roles: Vec<String>,
        token_type: TokenType,
        expiry_secs: i64,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(expiry_secs);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            username: username.to_string(),
            roles,
            jti: Uuid::new_v4().to_string(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            token_type,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;
        Ok(token)
    }

    /// Validate and decode a token
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);

        let token_data: TokenData<Claims> = decode(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Validate an access token specifically
    pub fn validate_access_token(&self, token: &str) -> Result<Claims> {
        let claims = self.validate_token(token)?;

        if claims.token_type != TokenType::Access {
            return Err(SecurityError::InvalidToken(
                "Expected access token".to_string(),
            ));
        }

        Ok(claims)
    }

    /// Validate a refresh token specifically
    pub fn validate_refresh_token(&self, token: &str) -> Result<Claims> {
        let claims = self.validate_token(token)?;

        if claims.token_type != TokenType::Refresh {
            return Err(SecurityError::InvalidRefreshToken);
        }

        Ok(claims)
    }

    /// Refresh tokens using a valid refresh token
    pub fn refresh_tokens(&self, refresh_token: &str) -> Result<TokenPair> {
        let claims = self.validate_refresh_token(refresh_token)?;

        self.generate_token_pair(
            &claims.sub,
            &claims.email,
            &claims.username,
            claims.roles,
        )
    }

    /// Get the token ID (jti) from a token without full validation
    pub fn get_token_id(&self, token: &str) -> Result<String> {
        // Use dangerous_insecure_decode to extract jti without validation
        // This is useful for token revocation checks
        let mut validation = Validation::default();
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.validate_aud = false;

        let token_data: TokenData<Claims> = decode(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims.jti)
    }

    /// Extract claims from token without validation (for debugging)
    pub fn decode_without_validation(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::default();
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.validate_aud = false;

        let token_data: TokenData<Claims> = decode(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    /// Get the configuration
    pub fn config(&self) -> &JwtConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> JwtManager {
        JwtManager::from_secret("test-secret-key-for-testing-only")
    }

    #[test]
    fn test_generate_token_pair() {
        let manager = create_test_manager();
        let result = manager.generate_token_pair(
            "user-123",
            "test@example.com",
            "testuser",
            vec!["user".to_string()],
        );

        assert!(result.is_ok());
        let pair = result.unwrap();
        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert_eq!(pair.token_type, "Bearer");
    }

    #[test]
    fn test_validate_access_token() {
        let manager = create_test_manager();
        let pair = manager
            .generate_token_pair(
                "user-123",
                "test@example.com",
                "testuser",
                vec!["user".to_string(), "admin".to_string()],
            )
            .unwrap();

        let claims = manager.validate_access_token(&pair.access_token).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.roles, vec!["user", "admin"]);
        assert_eq!(claims.token_type, TokenType::Access);
    }

    #[test]
    fn test_validate_refresh_token() {
        let manager = create_test_manager();
        let pair = manager
            .generate_token_pair(
                "user-123",
                "test@example.com",
                "testuser",
                vec!["user".to_string()],
            )
            .unwrap();

        let claims = manager.validate_refresh_token(&pair.refresh_token).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.token_type, TokenType::Refresh);
    }

    #[test]
    fn test_access_token_rejected_as_refresh() {
        let manager = create_test_manager();
        let pair = manager
            .generate_token_pair(
                "user-123",
                "test@example.com",
                "testuser",
                vec!["user".to_string()],
            )
            .unwrap();

        let result = manager.validate_refresh_token(&pair.access_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_tokens() {
        let manager = create_test_manager();
        let pair1 = manager
            .generate_token_pair(
                "user-123",
                "test@example.com",
                "testuser",
                vec!["user".to_string()],
            )
            .unwrap();

        let pair2 = manager.refresh_tokens(&pair1.refresh_token).unwrap();
        assert_ne!(pair1.access_token, pair2.access_token);
        assert_ne!(pair1.refresh_token, pair2.refresh_token);

        // New tokens should be valid
        let claims = manager.validate_access_token(&pair2.access_token).unwrap();
        assert_eq!(claims.sub, "user-123");
    }

    #[test]
    fn test_invalid_token() {
        let manager = create_test_manager();
        let result = manager.validate_access_token("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_from_different_secret() {
        let manager1 = JwtManager::from_secret("secret-1");
        let manager2 = JwtManager::from_secret("secret-2");

        let pair = manager1
            .generate_token_pair(
                "user-123",
                "test@example.com",
                "testuser",
                vec!["user".to_string()],
            )
            .unwrap();

        let result = manager2.validate_access_token(&pair.access_token);
        assert!(result.is_err());
    }
}
