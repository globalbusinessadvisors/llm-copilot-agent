//! Password hashing and validation
//!
//! Uses Argon2id for secure password hashing.

use crate::error::{Result, SecurityError};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Algorithm, Params, Version,
};

/// Password configuration
#[derive(Debug, Clone)]
pub struct PasswordConfig {
    /// Minimum password length
    pub min_length: usize,
    /// Maximum password length
    pub max_length: usize,
    /// Require uppercase letters
    pub require_uppercase: bool,
    /// Require lowercase letters
    pub require_lowercase: bool,
    /// Require digits
    pub require_digit: bool,
    /// Require special characters
    pub require_special: bool,
    /// Argon2 memory cost (in KiB)
    pub argon2_memory_cost: u32,
    /// Argon2 time cost (iterations)
    pub argon2_time_cost: u32,
    /// Argon2 parallelism
    pub argon2_parallelism: u32,
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            min_length: 8,
            max_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: false,
            argon2_memory_cost: 19456,  // 19 MiB (OWASP recommendation)
            argon2_time_cost: 2,
            argon2_parallelism: 1,
        }
    }
}

/// Password manager for hashing and verification
#[derive(Debug, Clone)]
pub struct PasswordManager {
    config: PasswordConfig,
    argon2: Argon2<'static>,
}

impl PasswordManager {
    /// Create a new password manager with the given configuration
    pub fn new(config: PasswordConfig) -> Result<Self> {
        let params = Params::new(
            config.argon2_memory_cost,
            config.argon2_time_cost,
            config.argon2_parallelism,
            None,
        )
        .map_err(|e| SecurityError::Configuration(format!("Invalid Argon2 params: {}", e)))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        Ok(Self { config, argon2 })
    }

    /// Create a password manager with default configuration
    pub fn default_config() -> Result<Self> {
        Self::new(PasswordConfig::default())
    }

    /// Validate password against policy
    pub fn validate_password(&self, password: &str) -> Result<()> {
        let mut errors = Vec::new();

        if password.len() < self.config.min_length {
            errors.push(format!(
                "Password must be at least {} characters",
                self.config.min_length
            ));
        }

        if password.len() > self.config.max_length {
            errors.push(format!(
                "Password must be at most {} characters",
                self.config.max_length
            ));
        }

        if self.config.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            errors.push("Password must contain at least one uppercase letter".to_string());
        }

        if self.config.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
            errors.push("Password must contain at least one lowercase letter".to_string());
        }

        if self.config.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push("Password must contain at least one digit".to_string());
        }

        if self.config.require_special {
            let special_chars = "!@#$%^&*()_+-=[]{}|;':\",./<>?`~";
            if !password.chars().any(|c| special_chars.contains(c)) {
                errors.push("Password must contain at least one special character".to_string());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(SecurityError::PasswordValidation(errors.join("; ")))
        }
    }

    /// Hash a password
    pub fn hash_password(&self, password: &str) -> Result<String> {
        self.validate_password(password)?;

        let salt = SaltString::generate(&mut OsRng);
        let hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| SecurityError::PasswordHashingFailed(e.to_string()))?;

        Ok(hash.to_string())
    }

    /// Verify a password against a hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| SecurityError::PasswordHashingFailed(format!("Invalid hash: {}", e)))?;

        match self.argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(SecurityError::PasswordHashingFailed(e.to_string())),
        }
    }

    /// Check if a password needs rehashing (e.g., if params changed)
    pub fn needs_rehash(&self, hash: &str) -> bool {
        if let Ok(parsed_hash) = PasswordHash::new(hash) {
            // Check if algorithm differs from Argon2id
            if parsed_hash.algorithm != argon2::ARGON2ID_IDENT {
                return true;
            }

            // Extract params from the hash string directly
            // Argon2 hash format: $argon2id$v=19$m=19456,t=2,p=1$salt$hash
            let hash_str = hash;
            if let Some(params_start) = hash_str.find("$m=") {
                let params_section = &hash_str[params_start + 1..];
                if let Some(params_end) = params_section.find('$') {
                    let params_str = &params_section[..params_end];
                    let mut m_val: Option<u32> = None;
                    let mut t_val: Option<u32> = None;
                    let mut p_val: Option<u32> = None;

                    for part in params_str.split(',') {
                        if let Some(val) = part.strip_prefix("m=") {
                            m_val = val.parse().ok();
                        } else if let Some(val) = part.strip_prefix("t=") {
                            t_val = val.parse().ok();
                        } else if let Some(val) = part.strip_prefix("p=") {
                            p_val = val.parse().ok();
                        }
                    }

                    if let (Some(m), Some(t), Some(p)) = (m_val, t_val, p_val) {
                        if m != self.config.argon2_memory_cost
                            || t != self.config.argon2_time_cost
                            || p != self.config.argon2_parallelism
                        {
                            return true;
                        }
                    }
                }
            }

            false
        } else {
            true
        }
    }

    /// Get the password policy as a human-readable string
    pub fn policy_description(&self) -> String {
        let mut parts = vec![format!(
            "Password must be {}-{} characters",
            self.config.min_length, self.config.max_length
        )];

        if self.config.require_uppercase {
            parts.push("contain uppercase letters".to_string());
        }
        if self.config.require_lowercase {
            parts.push("contain lowercase letters".to_string());
        }
        if self.config.require_digit {
            parts.push("contain digits".to_string());
        }
        if self.config.require_special {
            parts.push("contain special characters".to_string());
        }

        parts.join(", ")
    }
}

/// Generate a secure random password
pub fn generate_random_password(length: usize) -> String {
    use rand::Rng;

    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    let mut rng = rand::thread_rng();

    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> PasswordManager {
        PasswordManager::new(PasswordConfig {
            argon2_memory_cost: 4096,  // Lower for tests
            argon2_time_cost: 1,
            argon2_parallelism: 1,
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn test_hash_and_verify() {
        let manager = create_test_manager();
        let password = "SecureP@ss123";
        let hash = manager.hash_password(password).unwrap();

        assert!(manager.verify_password(password, &hash).unwrap());
        assert!(!manager.verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn test_password_validation() {
        let manager = create_test_manager();

        // Valid password
        assert!(manager.validate_password("SecurePass1").is_ok());

        // Too short
        assert!(manager.validate_password("Short1").is_err());

        // No uppercase
        assert!(manager.validate_password("nouppercase1").is_err());

        // No lowercase
        assert!(manager.validate_password("NOLOWERCASE1").is_err());

        // No digit
        assert!(manager.validate_password("NoDigitHere").is_err());
    }

    #[test]
    fn test_different_hashes_for_same_password() {
        let manager = create_test_manager();
        let password = "SecureP@ss123";

        let hash1 = manager.hash_password(password).unwrap();
        let hash2 = manager.hash_password(password).unwrap();

        // Hashes should be different due to random salt
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(manager.verify_password(password, &hash1).unwrap());
        assert!(manager.verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_generate_random_password() {
        let password = generate_random_password(16);
        assert_eq!(password.len(), 16);

        // Should be different each time
        let password2 = generate_random_password(16);
        assert_ne!(password, password2);
    }
}
