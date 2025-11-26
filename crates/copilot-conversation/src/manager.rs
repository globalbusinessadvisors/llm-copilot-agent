//! Conversation manager for handling multi-turn dialogue

use crate::{
    history::{ConversationMessage, HistoryManager, MessageRole},
    session::{SessionManager, SessionState},
    streaming::StreamingResponse,
    Result, ConversationError,
};
use async_trait::async_trait;
use copilot_context::ContextEngine;
use copilot_nlp::NlpEngine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Request for processing a user message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRequest {
    /// Session identifier
    pub session_id: String,
    /// User message content
    pub message: String,
    /// Optional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Response containing the assistant's reply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    /// Session identifier
    pub session_id: String,
    /// Assistant response content
    pub response: String,
    /// Resolved references in the message
    #[serde(default)]
    pub resolved_references: Vec<ResolvedReference>,
    /// Tokens used in this interaction
    pub tokens_used: usize,
    /// Total tokens used in session
    pub total_tokens: usize,
}

/// A resolved reference from the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedReference {
    /// Original reference text (e.g., "it", "that service")
    pub reference: String,
    /// What the reference refers to
    pub refers_to: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

/// Main conversation manager
pub struct ConversationManager {
    nlp_engine: Arc<dyn NlpEngine>,
    context_engine: Arc<dyn ContextEngine>,
    session_manager: Arc<RwLock<SessionManager>>,
    history_manager: Arc<RwLock<HistoryManager>>,
}

impl ConversationManager {
    /// Create a new conversation manager
    ///
    /// # Arguments
    ///
    /// * `nlp_engine` - NLP engine for language processing
    /// * `context_engine` - Context engine for maintaining conversation context
    pub fn new(nlp_engine: Arc<dyn NlpEngine>, context_engine: Arc<dyn ContextEngine>) -> Self {
        Self {
            nlp_engine,
            context_engine,
            session_manager: Arc::new(RwLock::new(SessionManager::new())),
            history_manager: Arc::new(RwLock::new(HistoryManager::new())),
        }
    }

    /// Process a user message
    ///
    /// This is the main entry point for handling user messages. It:
    /// 1. Validates the session
    /// 2. Resolves references
    /// 3. Updates context
    /// 4. Generates response
    /// 5. Updates history
    ///
    /// # Arguments
    ///
    /// * `request` - The message request to process
    pub async fn process_message(&self, request: MessageRequest) -> Result<MessageResponse> {
        info!("Processing message for session: {}", request.session_id);

        // Get or create session
        let mut session_mgr = self.session_manager.write().await;
        let session = session_mgr
            .get_session(&request.session_id)
            .ok_or_else(|| ConversationError::SessionNotFound(request.session_id.clone()))?;

        // Check if session is expired
        if session.state == SessionState::Expired {
            return Err(ConversationError::SessionExpired(request.session_id));
        }

        drop(session_mgr);

        // Resolve references in the message
        let resolved_refs = self.resolve_references(&request.session_id, &request.message).await?;
        debug!("Resolved {} references", resolved_refs.len());

        // Build enhanced message with resolved references
        let enhanced_message = self.enhance_message_with_references(&request.message, &resolved_refs);

        // Add user message to history
        let mut history_mgr = self.history_manager.write().await;
        history_mgr.append_message(
            &request.session_id,
            ConversationMessage {
                role: MessageRole::User,
                content: request.message.clone(),
                timestamp: chrono::Utc::now(),
                token_count: self.estimate_tokens(&request.message),
                metadata: request.metadata.clone(),
            },
        ).await?;
        drop(history_mgr);

        // Generate response
        let response = self.generate_response(&request.session_id, &enhanced_message).await?;
        let response_tokens = self.estimate_tokens(&response);

        // Add assistant message to history
        let mut history_mgr = self.history_manager.write().await;
        history_mgr.append_message(
            &request.session_id,
            ConversationMessage {
                role: MessageRole::Assistant,
                content: response.clone(),
                timestamp: chrono::Utc::now(),
                token_count: response_tokens,
                metadata: std::collections::HashMap::new(),
            },
        ).await?;
        drop(history_mgr);

        // Update session token count
        let message_tokens = self.estimate_tokens(&request.message);
        let total_tokens = message_tokens + response_tokens;

        let mut session_mgr = self.session_manager.write().await;
        session_mgr.update_session(&request.session_id, total_tokens).await?;
        let session = session_mgr.get_session(&request.session_id).unwrap();
        let session_total_tokens = session.total_tokens;

        Ok(MessageResponse {
            session_id: request.session_id,
            response,
            resolved_references: resolved_refs,
            tokens_used: total_tokens,
            total_tokens: session_total_tokens,
        })
    }

    /// Generate an assistant response
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier
    /// * `message` - The enhanced message with resolved references
    pub async fn generate_response(&self, session_id: &str, message: &str) -> Result<String> {
        debug!("Generating response for session: {}", session_id);

        // Get conversation history for context
        let history_mgr = self.history_manager.read().await;
        let history = history_mgr.get_history(session_id, 0, 10).await?;
        drop(history_mgr);

        // Build context from history
        let context = self.build_context_from_history(&history);

        // Use NLP engine to analyze intent
        let intent = self.nlp_engine
            .classify_intent(message)
            .await
            .map_err(|e| ConversationError::NlpError(e.to_string()))?;

        debug!("Detected intent: {:?}", intent);

        // Use context engine to enhance understanding
        let context_data = self.context_engine
            .retrieve(session_id)
            .await
            .map_err(|e| ConversationError::ContextError(e.to_string()))?;

        // Generate response based on intent and context
        // In a real implementation, this would call an LLM
        let response = format!(
            "I understand you're asking about: {:?}. Based on our conversation context, I can help with that.",
            intent
        );

        Ok(response)
    }

    /// Create a streaming response
    ///
    /// # Arguments
    ///
    /// * `request` - The message request to process
    pub async fn create_streaming_response(
        &self,
        request: MessageRequest,
    ) -> Result<StreamingResponse> {
        info!("Creating streaming response for session: {}", request.session_id);

        // Validate session exists
        {
            let mut session_mgr = self.session_manager.write().await;
            let _session = session_mgr
                .get_session(&request.session_id)
                .ok_or_else(|| ConversationError::SessionNotFound(request.session_id.clone()))?;
        }

        // Create streaming response
        let streaming_response = StreamingResponse::new(
            request.session_id.clone(),
            Arc::clone(&self.nlp_engine),
            Arc::clone(&self.context_engine),
            Arc::clone(&self.history_manager),
        );

        Ok(streaming_response)
    }

    /// Resolve references in a message
    ///
    /// Handles pronouns and references like "it", "that service", "the previous one"
    async fn resolve_references(
        &self,
        session_id: &str,
        message: &str,
    ) -> Result<Vec<ResolvedReference>> {
        let mut resolved = Vec::new();

        // Get recent history for reference resolution
        let history_mgr = self.history_manager.read().await;
        let history = history_mgr.get_history(session_id, 0, 5).await?;
        drop(history_mgr);

        // Simple reference patterns (in production, use NLP engine)
        let reference_patterns = vec![
            ("it", "the previously mentioned item"),
            ("that", "the referenced item"),
            ("this", "the current item"),
            ("they", "the mentioned items"),
            ("that service", "the service discussed earlier"),
            ("the previous", "the previous item"),
        ];

        for (pattern, default_ref) in reference_patterns {
            if message.to_lowercase().contains(pattern) {
                // Try to find actual reference in history
                let refers_to = self.find_reference_in_history(&history, pattern)
                    .unwrap_or_else(|| default_ref.to_string());

                resolved.push(ResolvedReference {
                    reference: pattern.to_string(),
                    refers_to,
                    confidence: 0.75, // Would be calculated by NLP engine
                });
            }
        }

        Ok(resolved)
    }

    /// Enhance message with resolved references
    fn enhance_message_with_references(
        &self,
        message: &str,
        references: &[ResolvedReference],
    ) -> String {
        let mut enhanced = message.to_string();

        for resolved in references {
            if resolved.confidence > 0.6 {
                // Only use high-confidence resolutions
                enhanced = enhanced.replace(
                    &resolved.reference,
                    &format!("{} ({})", resolved.reference, resolved.refers_to),
                );
            }
        }

        enhanced
    }

    /// Find what a reference refers to in conversation history
    fn find_reference_in_history(&self, history: &[ConversationMessage], _pattern: &str) -> Option<String> {
        // Look for nouns/entities in recent messages
        for msg in history.iter().rev().take(3) {
            if msg.role == MessageRole::User || msg.role == MessageRole::Assistant {
                // Simple extraction (would use NLP engine in production)
                let words: Vec<&str> = msg.content.split_whitespace().collect();
                for window in words.windows(2) {
                    if window[0].chars().next()?.is_uppercase() {
                        return Some(window.join(" "));
                    }
                }
            }
        }
        None
    }

    /// Build context string from conversation history
    fn build_context_from_history(&self, history: &[ConversationMessage]) -> String {
        history
            .iter()
            .map(|msg| format!("{:?}: {}", msg.role, msg.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Estimate token count for a message
    fn estimate_tokens(&self, text: &str) -> usize {
        // Simple estimation: ~4 characters per token
        // In production, use proper tokenizer
        (text.len() / 4).max(1)
    }

    /// Get session manager
    pub fn session_manager(&self) -> Arc<RwLock<SessionManager>> {
        Arc::clone(&self.session_manager)
    }

    /// Get history manager
    pub fn history_manager(&self) -> Arc<RwLock<HistoryManager>> {
        Arc::clone(&self.history_manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reference_resolution() {
        // Test would go here
    }

    #[tokio::test]
    async fn test_message_processing() {
        // Test would go here
    }
}
