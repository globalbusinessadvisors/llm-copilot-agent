//! Response streaming with Server-Sent Events (SSE) support

use crate::{history::HistoryManager, Result, ConversationError};
use copilot_context::ContextEngine;
use copilot_nlp::NlpEngine;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{debug, info};

/// A chunk of streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Chunk type
    #[serde(rename = "type")]
    pub chunk_type: ChunkType,
    /// The text content of this chunk
    pub content: String,
    /// Chunk sequence number
    pub sequence: usize,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Metadata for this chunk
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Type of stream chunk
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkType {
    /// Token chunk (text)
    Token,
    /// Thinking/reasoning step
    Thinking,
    /// Metadata update
    Metadata,
    /// Error occurred
    Error,
    /// Stream completed
    Done,
}

/// Statistics about streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStatistics {
    /// Time to first token (milliseconds)
    pub time_to_first_token_ms: u64,
    /// Total streaming duration (milliseconds)
    pub total_duration_ms: u64,
    /// Number of tokens streamed
    pub token_count: usize,
    /// Average tokens per second
    pub tokens_per_second: f64,
}

/// Streaming response handler
pub struct StreamingResponse {
    session_id: String,
    nlp_engine: Arc<dyn NlpEngine>,
    context_engine: Arc<dyn ContextEngine>,
    history_manager: Arc<RwLock<HistoryManager>>,
    start_time: Option<Instant>,
    first_token_time: Option<Instant>,
    token_count: usize,
}

impl StreamingResponse {
    /// Create a new streaming response
    pub fn new(
        session_id: String,
        nlp_engine: Arc<dyn NlpEngine>,
        context_engine: Arc<dyn ContextEngine>,
        history_manager: Arc<RwLock<HistoryManager>>,
    ) -> Self {
        Self {
            session_id,
            nlp_engine,
            context_engine,
            history_manager,
            start_time: None,
            first_token_time: None,
            token_count: 0,
        }
    }

    /// Start streaming response
    ///
    /// # Arguments
    ///
    /// * `message` - The user message to respond to
    pub async fn stream(
        &mut self,
        message: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        info!("Starting streaming response for session: {}", self.session_id);
        self.start_time = Some(Instant::now());

        // Create the stream
        let session_id = self.session_id.clone();
        let nlp_engine = Arc::clone(&self.nlp_engine);
        let context_engine = Arc::clone(&self.context_engine);
        let _history_manager = Arc::clone(&self.history_manager);

        // In a real implementation, this would stream from an LLM
        // For now, we'll simulate streaming
        let stream = async_stream::stream! {
            // Simulate first token latency optimization (target <500ms)
            let first_token_delay = Duration::from_millis(350);
            sleep(first_token_delay).await;

            // First token
            yield Ok(StreamChunk {
                chunk_type: ChunkType::Token,
                content: "I".to_string(),
                sequence: 0,
                is_final: false,
                metadata: std::collections::HashMap::new(),
            });

            // Simulate streaming tokens
            let response_tokens = vec![
                " understand",
                " your",
                " request",
                " about",
                " \"",
                &message,
                "\".",
                " Let",
                " me",
                " help",
                " you",
                " with",
                " that",
                ".",
            ];

            for (idx, token) in response_tokens.iter().enumerate() {
                // Simulate token generation delay
                sleep(Duration::from_millis(50)).await;

                yield Ok(StreamChunk {
                    chunk_type: ChunkType::Token,
                    content: token.to_string(),
                    sequence: idx + 1,
                    is_final: false,
                    metadata: std::collections::HashMap::new(),
                });
            }

            // Final chunk
            yield Ok(StreamChunk {
                chunk_type: ChunkType::Done,
                content: String::new(),
                sequence: response_tokens.len() + 1,
                is_final: true,
                metadata: std::collections::HashMap::new(),
            });

            debug!("Streaming completed for session: {}", session_id);
        };

        Ok(Box::pin(stream))
    }

    /// Convert stream to Server-Sent Events format
    pub fn to_sse_format(chunk: &StreamChunk) -> String {
        let json = serde_json::to_string(chunk).unwrap_or_default();
        format!("data: {}\n\n", json)
    }

    /// Record first token timing
    pub fn record_first_token(&mut self) {
        if self.first_token_time.is_none() {
            self.first_token_time = Some(Instant::now());
            let ttft = self.start_time
                .map(|start| start.elapsed().as_millis() as u64)
                .unwrap_or(0);
            debug!("Time to first token: {}ms", ttft);
        }
    }

    /// Increment token count
    pub fn increment_token_count(&mut self) {
        self.token_count += 1;
    }

    /// Get streaming statistics
    pub fn statistics(&self) -> StreamStatistics {
        let total_duration = self.start_time
            .map(|start| start.elapsed().as_millis() as u64)
            .unwrap_or(0);

        let time_to_first_token = match (self.start_time, self.first_token_time) {
            (Some(start), Some(first)) => first.duration_since(start).as_millis() as u64,
            _ => 0,
        };

        let tokens_per_second = if total_duration > 0 {
            (self.token_count as f64 / total_duration as f64) * 1000.0
        } else {
            0.0
        };

        StreamStatistics {
            time_to_first_token_ms: time_to_first_token,
            total_duration_ms: total_duration,
            token_count: self.token_count,
            tokens_per_second,
        }
    }
}

/// Stream builder for easier configuration
pub struct StreamBuilder {
    session_id: String,
    nlp_engine: Arc<dyn NlpEngine>,
    context_engine: Arc<dyn ContextEngine>,
    history_manager: Arc<RwLock<HistoryManager>>,
    optimize_first_token: bool,
    target_first_token_ms: u64,
}

impl StreamBuilder {
    /// Create a new stream builder
    pub fn new(
        session_id: String,
        nlp_engine: Arc<dyn NlpEngine>,
        context_engine: Arc<dyn ContextEngine>,
        history_manager: Arc<RwLock<HistoryManager>>,
    ) -> Self {
        Self {
            session_id,
            nlp_engine,
            context_engine,
            history_manager,
            optimize_first_token: true,
            target_first_token_ms: 500,
        }
    }

    /// Set first token latency optimization
    pub fn optimize_first_token(mut self, enabled: bool) -> Self {
        self.optimize_first_token = enabled;
        self
    }

    /// Set target first token latency in milliseconds
    pub fn target_first_token_ms(mut self, ms: u64) -> Self {
        self.target_first_token_ms = ms;
        self
    }

    /// Build the streaming response
    pub fn build(self) -> StreamingResponse {
        StreamingResponse::new(
            self.session_id,
            self.nlp_engine,
            self.context_engine,
            self.history_manager,
        )
    }
}

/// SSE event formatter
pub struct SseFormatter;

impl SseFormatter {
    /// Format a chunk as SSE event
    pub fn format(chunk: &StreamChunk) -> Result<String> {
        let json = serde_json::to_string(chunk)
            .map_err(|e| ConversationError::StreamingError(e.to_string()))?;
        Ok(format!("data: {}\n\n", json))
    }

    /// Format an error as SSE event
    pub fn format_error(error: &str) -> String {
        let chunk = StreamChunk {
            chunk_type: ChunkType::Error,
            content: error.to_string(),
            sequence: 0,
            is_final: true,
            metadata: std::collections::HashMap::new(),
        };
        serde_json::to_string(&chunk)
            .map(|json| format!("data: {}\n\n", json))
            .unwrap_or_else(|_| "data: {\"error\": \"unknown\"}\n\n".to_string())
    }

    /// Format completion event
    pub fn format_done() -> String {
        let chunk = StreamChunk {
            chunk_type: ChunkType::Done,
            content: String::new(),
            sequence: 0,
            is_final: true,
            metadata: std::collections::HashMap::new(),
        };
        serde_json::to_string(&chunk)
            .map(|json| format!("data: {}\n\n", json))
            .unwrap_or_else(|_| "data: {\"type\": \"done\"}\n\n".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copilot_context::{ContextEngineConfig, ContextEngineImpl};
    use copilot_nlp::NlpEngineImpl;

    #[test]
    fn test_sse_formatting() {
        let chunk = StreamChunk {
            chunk_type: ChunkType::Token,
            content: "Hello".to_string(),
            sequence: 0,
            is_final: false,
            metadata: std::collections::HashMap::new(),
        };

        let sse = SseFormatter::format(&chunk).unwrap();
        assert!(sse.starts_with("data: "));
        assert!(sse.ends_with("\n\n"));
        assert!(sse.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_statistics() {
        let context_config = ContextEngineConfig::default();
        let context_engine = ContextEngineImpl::new(context_config).unwrap();

        let mut response = StreamingResponse {
            session_id: "test".to_string(),
            nlp_engine: Arc::new(NlpEngineImpl::default()),
            context_engine: Arc::new(context_engine),
            history_manager: Arc::new(RwLock::new(HistoryManager::new())),
            start_time: Some(Instant::now()),
            first_token_time: None,
            token_count: 0,
        };

        response.record_first_token();
        response.increment_token_count();

        let stats = response.statistics();
        assert!(stats.time_to_first_token_ms >= 0);
        assert_eq!(stats.token_count, 1);
    }
}
