//! Streaming support for chat responses

use crate::error::{CopilotError, Result};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Events received during streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum StreamEvent {
    /// Content chunk received
    #[serde(rename = "content")]
    Content { text: String },

    /// Stream started
    #[serde(rename = "start")]
    Start { conversation_id: String },

    /// Stream completed
    #[serde(rename = "done")]
    Done {
        finish_reason: String,
        usage: Option<crate::models::Usage>,
    },

    /// Error occurred
    #[serde(rename = "error")]
    Error { message: String },

    /// Heartbeat/keep-alive
    #[serde(rename = "ping")]
    Ping,
}

/// A stream of chat events
pub struct ChatStream {
    inner: Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>,
}

impl ChatStream {
    /// Create a new chat stream from a boxed stream
    pub fn new(stream: Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>) -> Self {
        Self { inner: stream }
    }

    /// Collect all content from the stream into a single string
    pub async fn collect_content(mut self) -> Result<String> {
        use futures::StreamExt;

        let mut content = String::new();

        while let Some(event) = self.next().await {
            match event? {
                StreamEvent::Content { text } => content.push_str(&text),
                StreamEvent::Error { message } => {
                    return Err(CopilotError::Stream(message));
                }
                _ => {}
            }
        }

        Ok(content)
    }
}

impl Stream for ChatStream {
    type Item = Result<StreamEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Builder for creating mock streams (useful for testing)
#[derive(Default)]
pub struct MockStreamBuilder {
    events: Vec<StreamEvent>,
}

impl MockStreamBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content(mut self, text: impl Into<String>) -> Self {
        self.events.push(StreamEvent::Content { text: text.into() });
        self
    }

    pub fn done(mut self, finish_reason: impl Into<String>) -> Self {
        self.events.push(StreamEvent::Done {
            finish_reason: finish_reason.into(),
            usage: None,
        });
        self
    }

    pub fn build(self) -> ChatStream {
        use futures::stream;

        let events: Vec<Result<StreamEvent>> = self.events.into_iter().map(Ok).collect();
        ChatStream::new(Box::pin(stream::iter(events)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_mock_stream() {
        let stream = MockStreamBuilder::new()
            .content("Hello, ")
            .content("world!")
            .done("stop")
            .build();

        let content = stream.collect_content().await.unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[tokio::test]
    async fn test_stream_iteration() {
        let mut stream = MockStreamBuilder::new()
            .content("test")
            .done("stop")
            .build();

        let mut count = 0;
        while let Some(event) = stream.next().await {
            event.unwrap();
            count += 1;
        }
        assert_eq!(count, 2);
    }
}
