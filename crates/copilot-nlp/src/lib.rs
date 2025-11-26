//! # CoPilot NLP
//!
//! Natural Language Processing engine for the LLM CoPilot Agent.
//!
//! This crate provides intent classification, entity extraction, and query translation
//! capabilities to convert natural language queries into structured observability queries.
//!
//! ## Features
//!
//! - **Intent Classification**: Identifies user intent from natural language with confidence scoring
//! - **Entity Extraction**: Extracts entities like time ranges, services, metrics, and severity levels
//! - **Query Translation**: Converts natural language to PromQL, LogQL, and SQL queries
//!
//! ## Example
//!
//! ```rust,no_run
//! use copilot_nlp::{NlpEngine, NlpEngineImpl};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = NlpEngineImpl::new();
//!
//!     let intent = engine.classify_intent("Show me errors in the last 5 minutes").await?;
//!     println!("Intent: {:?}, Confidence: {}", intent.intent_type, intent.confidence);
//!
//!     Ok(())
//! }
//! ```

pub mod engine;
pub mod entity;
pub mod error;
pub mod intent;
pub mod query;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use error::{NlpError, Result};
use std::collections::HashMap;

pub use engine::NlpEngineImpl;
pub use entity::{Entity, EntityExtractor, EntityType};
pub use intent::{Intent, IntentClassifier, IntentType};
pub use query::{QueryLanguage, QueryTranslator};

/// Main NLP engine trait for processing natural language queries.
///
/// This trait defines the core NLP capabilities needed for the CoPilot Agent
/// to understand and process user queries about observability data.
#[async_trait]
pub trait NlpEngine: Send + Sync {
    /// Classifies the user's intent from natural language input.
    ///
    /// # Arguments
    ///
    /// * `query` - The natural language query from the user
    ///
    /// # Returns
    ///
    /// An `Intent` object containing the classified intent type and confidence score
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use copilot_nlp::{NlpEngine, NlpEngineImpl};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let engine = NlpEngineImpl::new();
    /// let intent = engine.classify_intent("Show me CPU usage").await?;
    /// assert!(intent.confidence > 0.5);
    /// # Ok(())
    /// # }
    /// ```
    async fn classify_intent(&self, query: &str) -> Result<Intent>;

    /// Extracts structured entities from the natural language query.
    ///
    /// # Arguments
    ///
    /// * `query` - The natural language query from the user
    ///
    /// # Returns
    ///
    /// A vector of extracted entities with their types and values
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use copilot_nlp::{NlpEngine, NlpEngineImpl};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let engine = NlpEngineImpl::new();
    /// let entities = engine.extract_entities("Show errors from auth-service in the last 5 minutes").await?;
    /// // Entities might include: TimeRange("5m"), Service("auth-service"), Severity("error")
    /// # Ok(())
    /// # }
    /// ```
    async fn extract_entities(&self, query: &str) -> Result<Vec<Entity>>;

    /// Translates a natural language query into a structured query language.
    ///
    /// # Arguments
    ///
    /// * `query` - The natural language query from the user
    /// * `intent` - The classified intent for the query
    /// * `entities` - Extracted entities from the query
    /// * `target_language` - The target query language (PromQL, LogQL, SQL)
    ///
    /// # Returns
    ///
    /// A string containing the translated query in the target language
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use copilot_nlp::{NlpEngine, NlpEngineImpl, QueryLanguage};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let engine = NlpEngineImpl::new();
    /// let intent = engine.classify_intent("Show CPU usage").await?;
    /// let entities = engine.extract_entities("Show CPU usage").await?;
    /// let query = engine.translate_query(
    ///     "Show CPU usage",
    ///     &intent,
    ///     &entities,
    ///     QueryLanguage::PromQL
    /// ).await?;
    /// // query might be: "rate(cpu_usage_seconds_total[5m])"
    /// # Ok(())
    /// # }
    /// ```
    async fn translate_query(
        &self,
        query: &str,
        intent: &Intent,
        entities: &[Entity],
        target_language: QueryLanguage,
    ) -> Result<String>;
}

/// Context information for NLP processing.
///
/// This struct contains additional context that can help improve
/// intent classification and entity extraction accuracy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NlpContext {
    /// Available services in the system
    pub available_services: Vec<String>,
    /// Available metrics
    pub available_metrics: Vec<String>,
    /// User's previous queries (for context)
    pub query_history: Vec<String>,
    /// Custom entity mappings
    pub custom_entities: HashMap<String, String>,
}

impl Default for NlpContext {
    fn default() -> Self {
        Self {
            available_services: Vec::new(),
            available_metrics: Vec::new(),
            query_history: Vec::new(),
            custom_entities: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nlp_engine_basic() {
        let engine = NlpEngineImpl::new();
        let intent = engine.classify_intent("Show me errors").await;
        assert!(intent.is_ok());
    }

    #[tokio::test]
    async fn test_entity_extraction() {
        let engine = NlpEngineImpl::new();
        let entities = engine.extract_entities("Show errors in the last 5 minutes").await;
        assert!(entities.is_ok());
        let entities = entities.unwrap();
        assert!(!entities.is_empty());
    }
}
