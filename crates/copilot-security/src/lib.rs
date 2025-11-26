//! # Copilot Security
//!
//! Security, authentication, and authorization module for LLM CoPilot Agent.
//!
//! This crate provides:
//! - JWT-based authentication with access and refresh tokens
//! - Password hashing and verification (Argon2)
//! - Role-based access control (RBAC)
//! - API key management with scopes
//! - Rate limiting
//! - Audit logging

pub mod auth;
pub mod jwt;
pub mod password;
pub mod rbac;
pub mod api_key;
pub mod rate_limit;
pub mod audit;
pub mod error;

pub use auth::*;
pub use jwt::*;
pub use password::*;
pub use rbac::*;
pub use api_key::*;
pub use rate_limit::*;
pub use audit::*;
pub use error::{SecurityError, Result};
