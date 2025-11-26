//! Conversation history management with search and export capabilities

use crate::{Result, ConversationError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Message role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    /// Message from the user
    User,
    /// Message from the assistant
    Assistant,
    /// System message
    System,
}

/// A single message in conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Role of the message sender
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Number of tokens in this message
    pub token_count: usize,
    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Search query for conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Text to search for
    pub query: String,
    /// Filter by role
    pub role: Option<MessageRole>,
    /// Start date filter
    pub start_date: Option<DateTime<Utc>>,
    /// End date filter
    pub end_date: Option<DateTime<Utc>>,
    /// Maximum number of results
    pub limit: Option<usize>,
}

/// Result of a history search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The matching message
    pub message: ConversationMessage,
    /// Relevance score (0.0 - 1.0)
    pub score: f64,
    /// Matching snippets
    pub snippets: Vec<String>,
}

/// Export format for conversation history
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// Markdown format
    Markdown,
    /// Plain text format
    Text,
    /// CSV format
    Csv,
}

/// Manages conversation history for sessions
pub struct HistoryManager {
    /// History storage: session_id -> messages
    history: HashMap<String, Vec<ConversationMessage>>,
    /// Maximum messages per session
    max_messages_per_session: usize,
    /// Whether to enable search indexing
    enable_search_index: bool,
}

impl HistoryManager {
    /// Create a new history manager
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
            max_messages_per_session: 1000,
            enable_search_index: true,
        }
    }

    /// Create a history manager with custom configuration
    pub fn with_config(max_messages: usize, enable_search: bool) -> Self {
        Self {
            history: HashMap::new(),
            max_messages_per_session: max_messages,
            enable_search_index: enable_search,
        }
    }

    /// Append a message to conversation history
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier
    /// * `message` - The message to append
    pub async fn append_message(
        &mut self,
        session_id: &str,
        message: ConversationMessage,
    ) -> Result<()> {
        debug!(
            "Appending message to session {}: {:?} - {} chars",
            session_id,
            message.role,
            message.content.len()
        );

        let messages = self.history.entry(session_id.to_string()).or_insert_with(Vec::new);

        // Enforce max messages limit
        if messages.len() >= self.max_messages_per_session {
            // Remove oldest message
            messages.remove(0);
            debug!("Removed oldest message due to limit");
        }

        messages.push(message);

        Ok(())
    }

    /// Get conversation history with pagination
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier
    /// * `offset` - Number of messages to skip
    /// * `limit` - Maximum number of messages to return
    pub async fn get_history(
        &self,
        session_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ConversationMessage>> {
        let messages: Vec<_> = self.history
            .get(session_id)
            .map(|msgs| {
                msgs.iter()
                    .skip(offset)
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        debug!(
            "Retrieved {} messages for session {} (offset: {}, limit: {})",
            messages.len(),
            session_id,
            offset,
            limit
        );

        Ok(messages)
    }

    /// Get all messages for a session
    pub async fn get_all_messages(&self, session_id: &str) -> Result<Vec<ConversationMessage>> {
        Ok(self.history.get(session_id).cloned().unwrap_or_default())
    }

    /// Get the number of messages in a session
    pub fn message_count(&self, session_id: &str) -> usize {
        self.history.get(session_id).map(|msgs| msgs.len()).unwrap_or(0)
    }

    /// Search conversation history
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier
    /// * `query` - Search query parameters
    pub async fn search_history(
        &self,
        session_id: &str,
        query: SearchQuery,
    ) -> Result<Vec<SearchResult>> {
        info!("Searching history for session {}: {}", session_id, query.query);

        let messages = self.history.get(session_id).cloned().unwrap_or_default();
        let mut results = Vec::new();

        for message in messages {
            // Apply role filter
            if let Some(role) = query.role {
                if message.role != role {
                    continue;
                }
            }

            // Apply date filters
            if let Some(start) = query.start_date {
                if message.timestamp < start {
                    continue;
                }
            }

            if let Some(end) = query.end_date {
                if message.timestamp > end {
                    continue;
                }
            }

            // Simple text search (in production, use proper search engine)
            let query_lower = query.query.to_lowercase();
            let content_lower = message.content.to_lowercase();

            if content_lower.contains(&query_lower) {
                // Calculate simple relevance score
                let score = self.calculate_relevance(&message.content, &query.query);

                // Extract snippets
                let snippets = self.extract_snippets(&message.content, &query.query, 3);

                results.push(SearchResult {
                    message,
                    score,
                    snippets,
                });
            }
        }

        // Sort by relevance score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        debug!("Found {} matching messages", results.len());

        Ok(results)
    }

    /// Export conversation history
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier
    /// * `format` - Export format
    pub async fn export_history(
        &self,
        session_id: &str,
        format: ExportFormat,
    ) -> Result<String> {
        info!("Exporting history for session {} as {:?}", session_id, format);

        let messages = self.history.get(session_id).cloned().unwrap_or_default();

        let output = match format {
            ExportFormat::Json => self.export_as_json(&messages)?,
            ExportFormat::Markdown => self.export_as_markdown(&messages),
            ExportFormat::Text => self.export_as_text(&messages),
            ExportFormat::Csv => self.export_as_csv(&messages),
        };

        Ok(output)
    }

    /// Clear history for a session
    pub fn clear_history(&mut self, session_id: &str) -> usize {
        let count = self.message_count(session_id);
        self.history.remove(session_id);
        info!("Cleared {} messages for session {}", count, session_id);
        count
    }

    /// Delete old messages before a certain date
    pub async fn delete_before(
        &mut self,
        session_id: &str,
        before: DateTime<Utc>,
    ) -> Result<usize> {
        let messages = self.history.get_mut(session_id);

        if let Some(msgs) = messages {
            let before_count = msgs.len();
            msgs.retain(|msg| msg.timestamp >= before);
            let deleted = before_count - msgs.len();
            info!("Deleted {} messages before {} for session {}", deleted, before, session_id);
            Ok(deleted)
        } else {
            Ok(0)
        }
    }

    /// Get statistics about conversation history
    pub fn statistics(&self, session_id: &str) -> HistoryStatistics {
        let messages = self.history.get(session_id).cloned().unwrap_or_default();

        let mut stats = HistoryStatistics {
            total_messages: messages.len(),
            user_messages: 0,
            assistant_messages: 0,
            system_messages: 0,
            total_tokens: 0,
            average_message_length: 0.0,
        };

        let mut total_length = 0;

        for msg in &messages {
            match msg.role {
                MessageRole::User => stats.user_messages += 1,
                MessageRole::Assistant => stats.assistant_messages += 1,
                MessageRole::System => stats.system_messages += 1,
            }
            stats.total_tokens += msg.token_count;
            total_length += msg.content.len();
        }

        if !messages.is_empty() {
            stats.average_message_length = total_length as f64 / messages.len() as f64;
        }

        stats
    }

    // Helper methods

    fn calculate_relevance(&self, content: &str, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();

        // Count occurrences
        let count = content_lower.matches(&query_lower).count();

        // Simple TF-IDF-like scoring
        let term_frequency = count as f64 / content.split_whitespace().count() as f64;
        let boost = if content_lower.starts_with(&query_lower) { 1.5 } else { 1.0 };

        (term_frequency * boost).min(1.0)
    }

    fn extract_snippets(&self, content: &str, query: &str, max_snippets: usize) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        let mut snippets = Vec::new();

        let words: Vec<&str> = content.split_whitespace().collect();
        let context_size = 5; // words before and after

        for (i, window) in words.windows(query.split_whitespace().count()).enumerate() {
            let window_text = window.join(" ");
            if window_text.to_lowercase().contains(&query_lower) {
                let start = i.saturating_sub(context_size);
                let end = (i + window.len() + context_size).min(words.len());

                let snippet = words[start..end].join(" ");
                snippets.push(format!("...{}...", snippet));

                if snippets.len() >= max_snippets {
                    break;
                }
            }
        }

        snippets
    }

    fn export_as_json(&self, messages: &[ConversationMessage]) -> Result<String> {
        serde_json::to_string_pretty(messages)
            .map_err(|e| ConversationError::SerializationError(e))
    }

    fn export_as_markdown(&self, messages: &[ConversationMessage]) -> String {
        let mut output = String::from("# Conversation History\n\n");

        for msg in messages {
            let role = match msg.role {
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::System => "System",
            };

            output.push_str(&format!(
                "## {} - {}\n\n{}\n\n---\n\n",
                role,
                msg.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                msg.content
            ));
        }

        output
    }

    fn export_as_text(&self, messages: &[ConversationMessage]) -> String {
        let mut output = String::from("Conversation History\n");
        output.push_str(&"=".repeat(50));
        output.push_str("\n\n");

        for msg in messages {
            let role = match msg.role {
                MessageRole::User => "USER",
                MessageRole::Assistant => "ASSISTANT",
                MessageRole::System => "SYSTEM",
            };

            output.push_str(&format!(
                "[{}] {} ({})\n{}\n\n",
                msg.timestamp.format("%Y-%m-%d %H:%M:%S"),
                role,
                msg.token_count,
                msg.content
            ));
        }

        output
    }

    fn export_as_csv(&self, messages: &[ConversationMessage]) -> String {
        let mut output = String::from("timestamp,role,content,token_count\n");

        for msg in messages {
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => "system",
            };

            // Escape CSV content
            let content = msg.content.replace('"', "\"\"");

            output.push_str(&format!(
                "{},{},\"{}\",{}\n",
                msg.timestamp.to_rfc3339(),
                role,
                content,
                msg.token_count
            ));
        }

        output
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryStatistics {
    pub total_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub system_messages: usize,
    pub total_tokens: usize,
    pub average_message_length: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_append_and_retrieve() {
        let mut manager = HistoryManager::new();
        let session_id = "test-session";

        let message = ConversationMessage {
            role: MessageRole::User,
            content: "Hello, world!".to_string(),
            timestamp: Utc::now(),
            token_count: 3,
            metadata: HashMap::new(),
        };

        manager.append_message(session_id, message).await.unwrap();

        let history = manager.get_history(session_id, 0, 10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "Hello, world!");
    }

    #[tokio::test]
    async fn test_search() {
        let mut manager = HistoryManager::new();
        let session_id = "test-session";

        manager.append_message(
            session_id,
            ConversationMessage {
                role: MessageRole::User,
                content: "Tell me about Rust programming".to_string(),
                timestamp: Utc::now(),
                token_count: 5,
                metadata: HashMap::new(),
            },
        ).await.unwrap();

        let query = SearchQuery {
            query: "Rust".to_string(),
            role: None,
            start_date: None,
            end_date: None,
            limit: Some(10),
        };

        let results = manager.search_history(session_id, query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].score > 0.0);
    }

    #[tokio::test]
    async fn test_export_formats() {
        let mut manager = HistoryManager::new();
        let session_id = "test-session";

        manager.append_message(
            session_id,
            ConversationMessage {
                role: MessageRole::User,
                content: "Test message".to_string(),
                timestamp: Utc::now(),
                token_count: 2,
                metadata: HashMap::new(),
            },
        ).await.unwrap();

        // Test JSON export
        let json = manager.export_history(session_id, ExportFormat::Json).await.unwrap();
        assert!(json.contains("Test message"));

        // Test Markdown export
        let md = manager.export_history(session_id, ExportFormat::Markdown).await.unwrap();
        assert!(md.contains("# Conversation History"));

        // Test Text export
        let text = manager.export_history(session_id, ExportFormat::Text).await.unwrap();
        assert!(text.contains("Test message"));

        // Test CSV export
        let csv = manager.export_history(session_id, ExportFormat::Csv).await.unwrap();
        assert!(csv.contains("timestamp,role,content,token_count"));
    }
}
