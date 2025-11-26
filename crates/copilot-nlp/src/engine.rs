//! NLP Engine implementation.
//!
//! This module provides the main NLP engine that orchestrates intent classification,
//! entity extraction, and query translation.

use async_trait::async_trait;
use crate::error::{NlpError, Result};
use tracing::{debug, info, instrument};

use crate::entity::{Entity, EntityExtractor};
use crate::intent::{Intent, IntentClassifier};
use crate::query::{QueryLanguage, QueryTranslator};
use crate::{NlpContext, NlpEngine};

/// Implementation of the NLP engine.
///
/// This struct provides the main NLP capabilities for processing natural language
/// queries about observability data.
pub struct NlpEngineImpl {
    /// Intent classifier
    intent_classifier: IntentClassifier,
    /// Entity extractor
    entity_extractor: EntityExtractor,
    /// Query translator
    query_translator: QueryTranslator,
    /// Optional context for improved accuracy
    context: Option<NlpContext>,
}

impl NlpEngineImpl {
    /// Creates a new NLP engine with default configuration.
    pub fn new() -> Self {
        info!("Initializing NLP engine");
        Self {
            intent_classifier: IntentClassifier::new(),
            entity_extractor: EntityExtractor::new(),
            query_translator: QueryTranslator::new(),
            context: None,
        }
    }

    /// Creates a new NLP engine with context.
    ///
    /// # Arguments
    ///
    /// * `context` - Additional context to improve NLP accuracy
    pub fn with_context(context: NlpContext) -> Self {
        info!("Initializing NLP engine with context");

        let entity_extractor = EntityExtractor::with_context(
            context.available_services.clone(),
            context.available_metrics.clone(),
        );

        Self {
            intent_classifier: IntentClassifier::new(),
            entity_extractor,
            query_translator: QueryTranslator::new(),
            context: Some(context),
        }
    }

    /// Updates the context for the NLP engine.
    ///
    /// # Arguments
    ///
    /// * `context` - New context to use
    pub fn update_context(&mut self, context: NlpContext) {
        debug!("Updating NLP engine context");

        self.entity_extractor = EntityExtractor::with_context(
            context.available_services.clone(),
            context.available_metrics.clone(),
        );

        self.context = Some(context);
    }

    /// Gets the current context.
    pub fn context(&self) -> Option<&NlpContext> {
        self.context.as_ref()
    }

    /// Validates the query before processing.
    fn validate_query(&self, query: &str) -> Result<()> {
        if query.trim().is_empty() {
            return Err(NlpError::validation("Query cannot be empty"));
        }

        if query.len() > 1000 {
            return Err(NlpError::validation("Query is too long (max 1000 characters)"));
        }

        Ok(())
    }

    /// Pre-processes the query (normalization, cleaning).
    fn preprocess_query(&self, query: &str) -> String {
        // Basic preprocessing: trim whitespace, normalize spaces
        query
            .trim()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Post-processes the intent (applies context, adjusts confidence).
    fn postprocess_intent(&self, mut intent: Intent, query: &str) -> Intent {
        // If we have context and the intent is uncertain, we could adjust confidence
        if let Some(context) = &self.context {
            // Check if the query matches recent query patterns
            let similar_queries = context.query_history.iter()
                .filter(|q| self.query_similarity(query, q) > 0.7)
                .count();

            if similar_queries > 0 && intent.confidence < 0.7 {
                // Boost confidence slightly if similar queries exist
                intent.confidence = (intent.confidence + 0.1).min(1.0);
                debug!("Boosted intent confidence based on query history");
            }
        }

        intent
    }

    /// Calculates similarity between two queries (simple word overlap).
    fn query_similarity(&self, query1: &str, query2: &str) -> f64 {
        let q1_lower = query1.to_lowercase();
        let q2_lower = query2.to_lowercase();

        let words1: std::collections::HashSet<&str> = q1_lower.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = q2_lower.split_whitespace().collect();

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }
}

impl Default for NlpEngineImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NlpEngine for NlpEngineImpl {
    #[instrument(skip(self), fields(query_len = query.len()))]
    async fn classify_intent(&self, query: &str) -> Result<Intent> {
        debug!("Classifying intent for query");

        // Validate input
        self.validate_query(query)?;

        // Preprocess query
        let processed_query = self.preprocess_query(query);

        // Classify intent
        let intent = self.intent_classifier.classify(&processed_query);

        // Post-process with context
        let intent = self.postprocess_intent(intent, &processed_query);

        info!(
            "Intent classified: {:?} with confidence: {:.2}",
            intent.intent_type, intent.confidence
        );

        Ok(intent)
    }

    #[instrument(skip(self), fields(query_len = query.len()))]
    async fn extract_entities(&self, query: &str) -> Result<Vec<Entity>> {
        debug!("Extracting entities from query");

        // Validate input
        self.validate_query(query)?;

        // Preprocess query
        let processed_query = self.preprocess_query(query);

        // Extract entities
        let entities = self.entity_extractor.extract(&processed_query);

        info!("Extracted {} entities", entities.len());

        // Log entity details at debug level
        for entity in &entities {
            debug!(
                "Entity: {:?} = {} (confidence: {:.2})",
                entity.entity_type, entity.normalized_value, entity.confidence
            );
        }

        Ok(entities)
    }

    #[instrument(
        skip(self, intent, entities),
        fields(
            intent_type = ?intent.intent_type,
            entity_count = entities.len(),
            target_lang = ?target_language
        )
    )]
    async fn translate_query(
        &self,
        query: &str,
        intent: &Intent,
        entities: &[Entity],
        target_language: QueryLanguage,
    ) -> Result<String> {
        debug!("Translating query to {:?}", target_language);

        // Validate input
        self.validate_query(query)?;

        // Translate based on target language
        let translated_query = match target_language {
            QueryLanguage::PromQL => self.query_translator.to_promql(intent, entities),
            QueryLanguage::LogQL => self.query_translator.to_logql(intent, entities),
            QueryLanguage::SQL => self.query_translator.to_sql(intent, entities),
            QueryLanguage::TraceQL => {
                // TraceQL not yet implemented, return a placeholder
                debug!("TraceQL translation not yet implemented");
                return Err(NlpError::unsupported("TraceQL translation not yet implemented"));
            }
        };

        info!(
            "Query translated to {:?}: {}",
            target_language, translated_query
        );

        Ok(translated_query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_classify_intent_basic() {
        let engine = NlpEngineImpl::new();
        let intent = engine.classify_intent("Show me CPU usage").await.unwrap();
        assert_eq!(intent.intent_type, crate::intent::IntentType::QueryMetrics);
        assert!(intent.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_classify_intent_empty_query() {
        let engine = NlpEngineImpl::new();
        let result = engine.classify_intent("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_entities_basic() {
        let engine = NlpEngineImpl::new();
        let entities = engine
            .extract_entities("Show errors from auth-service in the last 5 minutes")
            .await
            .unwrap();

        assert!(!entities.is_empty());

        // Should have time range
        assert!(entities.iter().any(|e| e.entity_type == crate::entity::EntityType::TimeRange));

        // Should have service
        assert!(entities.iter().any(|e| e.entity_type == crate::entity::EntityType::Service));
    }

    #[tokio::test]
    async fn test_translate_query_promql() {
        let engine = NlpEngineImpl::new();
        let intent = engine.classify_intent("Show CPU usage").await.unwrap();
        let entities = engine.extract_entities("Show CPU usage").await.unwrap();

        let query = engine
            .translate_query("Show CPU usage", &intent, &entities, QueryLanguage::PromQL)
            .await
            .unwrap();

        assert!(!query.is_empty());
        assert!(query.contains("node_cpu_seconds_total") || query.contains("rate"));
    }

    #[tokio::test]
    async fn test_translate_query_logql() {
        let engine = NlpEngineImpl::new();
        let intent = engine.classify_intent("Find errors in auth-service").await.unwrap();
        let entities = engine.extract_entities("Find errors in auth-service").await.unwrap();

        let query = engine
            .translate_query(
                "Find errors in auth-service",
                &intent,
                &entities,
                QueryLanguage::LogQL,
            )
            .await
            .unwrap();

        assert!(!query.is_empty());
    }

    #[tokio::test]
    async fn test_engine_with_context() {
        let context = NlpContext {
            available_services: vec!["payment-service".to_string()],
            available_metrics: vec!["checkout_duration".to_string()],
            query_history: Vec::new(),
            custom_entities: std::collections::HashMap::new(),
        };

        let engine = NlpEngineImpl::with_context(context);
        let entities = engine
            .extract_entities("Show checkout_duration for payment-service")
            .await
            .unwrap();

        assert!(entities.iter().any(|e| e.entity_type == crate::entity::EntityType::Service));
        assert!(entities.iter().any(|e| e.entity_type == crate::entity::EntityType::Metric));
    }

    #[tokio::test]
    async fn test_query_preprocessing() {
        let engine = NlpEngineImpl::new();
        let processed = engine.preprocess_query("  Show   me   CPU   usage  ");
        assert_eq!(processed, "Show me CPU usage");
    }

    #[tokio::test]
    async fn test_query_validation() {
        let engine = NlpEngineImpl::new();

        // Empty query should fail
        assert!(engine.validate_query("").is_err());

        // Very long query should fail
        let long_query = "a".repeat(1001);
        assert!(engine.validate_query(&long_query).is_err());

        // Normal query should pass
        assert!(engine.validate_query("Show CPU usage").is_ok());
    }

    #[tokio::test]
    async fn test_query_similarity() {
        let engine = NlpEngineImpl::new();

        let sim1 = engine.query_similarity("Show CPU usage", "Show CPU usage");
        assert_eq!(sim1, 1.0);

        let sim2 = engine.query_similarity("Show CPU usage", "Display CPU utilization");
        assert!(sim2 > 0.0 && sim2 < 1.0);

        let sim3 = engine.query_similarity("Show CPU usage", "Hello world");
        assert!(sim3 < 0.5);
    }

    #[tokio::test]
    async fn test_update_context() {
        let mut engine = NlpEngineImpl::new();

        let context = NlpContext {
            available_services: vec!["test-service".to_string()],
            available_metrics: vec!["test-metric".to_string()],
            query_history: Vec::new(),
            custom_entities: std::collections::HashMap::new(),
        };

        engine.update_context(context);
        assert!(engine.context().is_some());
    }
}
