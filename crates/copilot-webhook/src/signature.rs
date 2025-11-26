//! Webhook signature handling
//!
//! Provides HMAC-SHA256 signature generation and verification.

use crate::{Result, WebhookError};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Signature algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// HMAC-SHA256
    HmacSha256,
}

impl Default for SignatureAlgorithm {
    fn default() -> Self {
        Self::HmacSha256
    }
}

impl SignatureAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HmacSha256 => "sha256",
        }
    }
}

/// Webhook signature configuration
#[derive(Debug, Clone)]
pub struct SignatureConfig {
    /// Algorithm to use
    pub algorithm: SignatureAlgorithm,
    /// Signature header name
    pub header_name: String,
    /// Timestamp header name
    pub timestamp_header: String,
    /// Tolerance for timestamp validation (in seconds)
    pub timestamp_tolerance_seconds: i64,
    /// Signature version
    pub version: String,
}

impl Default for SignatureConfig {
    fn default() -> Self {
        Self {
            algorithm: SignatureAlgorithm::HmacSha256,
            header_name: "X-Webhook-Signature".to_string(),
            timestamp_header: "X-Webhook-Timestamp".to_string(),
            timestamp_tolerance_seconds: 300, // 5 minutes
            version: "v1".to_string(),
        }
    }
}

/// Webhook signer for generating signatures
pub struct WebhookSigner {
    secret: Vec<u8>,
    config: SignatureConfig,
}

impl WebhookSigner {
    /// Create a new signer with a secret
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            config: SignatureConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(secret: &str, config: SignatureConfig) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            config,
        }
    }

    /// Generate a signature for a payload
    pub fn sign(&self, payload: &[u8], timestamp: DateTime<Utc>) -> String {
        let timestamp_str = timestamp.timestamp().to_string();
        let message = format!("{}.{}", timestamp_str, String::from_utf8_lossy(payload));

        let signature = self.compute_signature(message.as_bytes());

        format!(
            "{}={},{}={}",
            "t", timestamp_str, self.config.algorithm.as_str(), signature
        )
    }

    /// Generate signature with current timestamp
    pub fn sign_now(&self, payload: &[u8]) -> (String, DateTime<Utc>) {
        let timestamp = Utc::now();
        let signature = self.sign(payload, timestamp);
        (signature, timestamp)
    }

    /// Compute raw HMAC signature
    fn compute_signature(&self, data: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC can accept any key length");
        mac.update(data);
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    /// Get headers for a signed request
    pub fn get_headers(&self, payload: &[u8]) -> Vec<(String, String)> {
        let timestamp = Utc::now();
        let signature = self.sign(payload, timestamp);

        vec![
            (self.config.header_name.clone(), signature),
            (
                self.config.timestamp_header.clone(),
                timestamp.timestamp().to_string(),
            ),
        ]
    }
}

/// Webhook verifier for validating signatures
pub struct WebhookVerifier {
    secret: Vec<u8>,
    config: SignatureConfig,
}

impl WebhookVerifier {
    /// Create a new verifier with a secret
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            config: SignatureConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(secret: &str, config: SignatureConfig) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            config,
        }
    }

    /// Verify a webhook signature
    pub fn verify(&self, payload: &[u8], signature_header: &str) -> Result<()> {
        let (timestamp, signatures) = self.parse_signature_header(signature_header)?;

        // Verify timestamp
        self.verify_timestamp(timestamp)?;

        // Construct expected signature
        let message = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));
        let expected = self.compute_signature(message.as_bytes());

        // Check if any signature matches
        let algorithm = self.config.algorithm.as_str();
        for (alg, sig) in signatures {
            if alg == algorithm && constant_time_compare(&sig, &expected) {
                return Ok(());
            }
        }

        Err(WebhookError::SignatureVerificationFailed(
            "No matching signature found".to_string(),
        ))
    }

    /// Parse signature header
    fn parse_signature_header(&self, header: &str) -> Result<(i64, Vec<(String, String)>)> {
        let mut timestamp: Option<i64> = None;
        let mut signatures = Vec::new();

        for part in header.split(',') {
            let mut kv = part.trim().splitn(2, '=');
            let key = kv.next().ok_or_else(|| {
                WebhookError::SignatureVerificationFailed("Invalid signature format".to_string())
            })?;
            let value = kv.next().ok_or_else(|| {
                WebhookError::SignatureVerificationFailed("Invalid signature format".to_string())
            })?;

            if key == "t" {
                timestamp = Some(value.parse().map_err(|_| {
                    WebhookError::SignatureVerificationFailed("Invalid timestamp".to_string())
                })?);
            } else {
                signatures.push((key.to_string(), value.to_string()));
            }
        }

        let ts = timestamp.ok_or_else(|| {
            WebhookError::SignatureVerificationFailed("Missing timestamp".to_string())
        })?;

        Ok((ts, signatures))
    }

    /// Verify timestamp is within tolerance
    fn verify_timestamp(&self, timestamp: i64) -> Result<()> {
        let now = Utc::now().timestamp();
        let diff = (now - timestamp).abs();

        if diff > self.config.timestamp_tolerance_seconds {
            return Err(WebhookError::SignatureVerificationFailed(format!(
                "Timestamp too old: {} seconds difference",
                diff
            )));
        }

        Ok(())
    }

    /// Compute raw HMAC signature
    fn compute_signature(&self, data: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC can accept any key length");
        mac.update(data);
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }

    result == 0
}

/// Generate a secure random webhook secret
pub fn generate_webhook_secret() -> String {
    use rand::Rng;
    let secret: [u8; 32] = rand::thread_rng().gen();
    format!("whsec_{}", hex::encode(secret))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_sign_and_verify() {
        let secret = "test-secret-key";
        let signer = WebhookSigner::new(secret);
        let verifier = WebhookVerifier::new(secret);

        let payload = b"test payload";
        let timestamp = Utc::now();

        let signature = signer.sign(payload, timestamp);
        assert!(verifier.verify(payload, &signature).is_ok());
    }

    #[test]
    fn test_wrong_secret() {
        let signer = WebhookSigner::new("secret-1");
        let verifier = WebhookVerifier::new("secret-2");

        let payload = b"test payload";
        let (signature, _) = signer.sign_now(payload);

        assert!(verifier.verify(payload, &signature).is_err());
    }

    #[test]
    fn test_modified_payload() {
        let secret = "test-secret";
        let signer = WebhookSigner::new(secret);
        let verifier = WebhookVerifier::new(secret);

        let payload = b"original payload";
        let (signature, _) = signer.sign_now(payload);

        // Try to verify with different payload
        let result = verifier.verify(b"modified payload", &signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_timestamp() {
        let secret = "test-secret";
        let signer = WebhookSigner::new(secret);

        let mut config = SignatureConfig::default();
        config.timestamp_tolerance_seconds = 60; // 1 minute tolerance
        let verifier = WebhookVerifier::with_config(secret, config);

        let payload = b"test payload";
        // Sign with old timestamp
        let old_time = Utc::now() - Duration::seconds(120); // 2 minutes ago
        let signature = signer.sign(payload, old_time);

        let result = verifier.verify(payload, &signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_headers() {
        let signer = WebhookSigner::new("test-secret");
        let payload = b"test payload";

        let headers = signer.get_headers(payload);
        assert_eq!(headers.len(), 2);
        assert!(headers.iter().any(|(k, _)| k == "X-Webhook-Signature"));
        assert!(headers.iter().any(|(k, _)| k == "X-Webhook-Timestamp"));
    }

    #[test]
    fn test_generate_secret() {
        let secret = generate_webhook_secret();
        assert!(secret.starts_with("whsec_"));
        assert!(secret.len() > 10);

        // Generate multiple and ensure they're different
        let secret2 = generate_webhook_secret();
        assert_ne!(secret, secret2);
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("abc", "abc"));
        assert!(!constant_time_compare("abc", "abd"));
        assert!(!constant_time_compare("abc", "ab"));
        assert!(!constant_time_compare("abc", "abcd"));
    }
}
