//! Copilot API client implementation

use crate::error::{CopilotError, Result};
use crate::models::*;
use crate::streaming::{ChatStream, StreamEvent};
use futures::StreamExt;
use reqwest::{header, Client, Response, StatusCode};
use secrecy::{ExposeSecret, Secret};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, instrument};
use url::Url;

/// Client for interacting with the Copilot API
#[derive(Clone)]
pub struct CopilotClient {
    http: Client,
    base_url: Url,
    api_key: Option<Secret<String>>,
}

impl std::fmt::Debug for CopilotClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CopilotClient")
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

/// Builder for creating a CopilotClient
#[derive(Default)]
pub struct CopilotClientBuilder {
    base_url: Option<String>,
    api_key: Option<String>,
    timeout: Option<Duration>,
    user_agent: Option<String>,
}

impl CopilotClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base URL for the API
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the API key for authentication
    pub fn api_key(mut self, key: Option<String>) -> Self {
        self.api_key = key;
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set a custom user agent
    pub fn user_agent(mut self, agent: impl Into<String>) -> Self {
        self.user_agent = Some(agent.into());
        self
    }

    /// Build the client
    pub fn build(self) -> Result<CopilotClient> {
        let base_url = self
            .base_url
            .unwrap_or_else(|| "http://localhost:8080".to_string());

        let base_url = Url::parse(&base_url)?;

        let timeout = self.timeout.unwrap_or(Duration::from_secs(60));
        let user_agent = self
            .user_agent
            .unwrap_or_else(|| format!("copilot-sdk/{}", env!("CARGO_PKG_VERSION")));

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );

        let http = Client::builder()
            .timeout(timeout)
            .user_agent(user_agent)
            .default_headers(headers)
            .build()
            .map_err(CopilotError::Http)?;

        Ok(CopilotClient {
            http,
            base_url,
            api_key: self.api_key.map(Secret::new),
        })
    }
}

impl CopilotClient {
    /// Create a new client builder
    pub fn builder() -> CopilotClientBuilder {
        CopilotClientBuilder::new()
    }

    /// Create a client with default settings
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        Self::builder().base_url(base_url).build()
    }

    /// Get the base URL
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// Build a URL for an endpoint
    fn url(&self, path: &str) -> Result<Url> {
        self.base_url.join(path).map_err(CopilotError::Url)
    }

    /// Add authentication header if API key is set
    fn auth_header(&self) -> Option<String> {
        self.api_key
            .as_ref()
            .map(|key| format!("Bearer {}", key.expose_secret()))
    }

    /// Handle API response
    async fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            response.json().await.map_err(CopilotError::Http)
        } else {
            let error_body = response.text().await.unwrap_or_default();

            match status {
                StatusCode::UNAUTHORIZED => Err(CopilotError::Auth(error_body)),
                StatusCode::NOT_FOUND => Err(CopilotError::NotFound(error_body)),
                StatusCode::TOO_MANY_REQUESTS => {
                    // Try to parse retry-after header
                    Err(CopilotError::RateLimit { retry_after: None })
                }
                _ if status.is_server_error() => Err(CopilotError::Server(error_body)),
                _ => Err(CopilotError::Api {
                    status: status.as_u16(),
                    message: error_body,
                    code: None,
                }),
            }
        }
    }

    // ===== Chat API =====

    /// Send a chat message
    #[instrument(skip(self, message))]
    pub async fn chat(
        &self,
        message: impl Into<String>,
        conversation_id: Option<String>,
    ) -> Result<ChatResponse> {
        self.chat_with_options(message, conversation_id, ChatOptions::default())
            .await
    }

    /// Send a chat message with options
    #[instrument(skip(self, message, options))]
    pub async fn chat_with_options(
        &self,
        message: impl Into<String>,
        conversation_id: Option<String>,
        options: ChatOptions,
    ) -> Result<ChatResponse> {
        let request = ChatRequest {
            message: message.into(),
            conversation_id,
            model: options.model,
            system_prompt: options.system_prompt,
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            stream: false,
        };

        let mut req = self.http.post(self.url("/api/v1/chat")?).json(&request);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Send a chat message with streaming response
    #[instrument(skip(self, message))]
    pub async fn chat_stream(
        &self,
        message: impl Into<String>,
        conversation_id: Option<String>,
    ) -> Result<ChatStream> {
        self.chat_stream_with_options(message, conversation_id, ChatOptions::default())
            .await
    }

    /// Send a chat message with streaming and options
    #[instrument(skip(self, message, options))]
    pub async fn chat_stream_with_options(
        &self,
        message: impl Into<String>,
        conversation_id: Option<String>,
        options: ChatOptions,
    ) -> Result<ChatStream> {
        let request = ChatRequest {
            message: message.into(),
            conversation_id,
            model: options.model,
            system_prompt: options.system_prompt,
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            stream: true,
        };

        let mut req = self
            .http
            .post(self.url("/api/v1/chat/stream")?)
            .json(&request);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(CopilotError::Api {
                status: status.as_u16(),
                message: error_body,
                code: None,
            });
        }

        let stream = response
            .bytes_stream()
            .map(|result| -> Result<StreamEvent> {
                let bytes = result.map_err(CopilotError::Http)?;
                let text = String::from_utf8_lossy(&bytes);

                // Parse SSE format
                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            return Ok(StreamEvent::Done {
                                finish_reason: "stop".to_string(),
                                usage: None,
                            });
                        }
                        if let Ok(event) = serde_json::from_str(data) {
                            return Ok(event);
                        }
                    }
                }

                // Default to content if we can't parse
                Ok(StreamEvent::Content {
                    text: text.to_string(),
                })
            });

        Ok(ChatStream::new(Box::pin(stream)))
    }

    // ===== Conversation API =====

    /// List conversations
    #[instrument(skip(self))]
    pub async fn list_conversations(&self, limit: usize) -> Result<Vec<Conversation>> {
        let mut req = self
            .http
            .get(self.url(&format!("/api/v1/conversations?limit={}", limit))?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Get a specific conversation
    #[instrument(skip(self))]
    pub async fn get_conversation(&self, id: &str) -> Result<Conversation> {
        let mut req = self
            .http
            .get(self.url(&format!("/api/v1/conversations/{}", id))?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Delete a conversation
    #[instrument(skip(self))]
    pub async fn delete_conversation(&self, id: &str) -> Result<()> {
        let mut req = self
            .http
            .delete(self.url(&format!("/api/v1/conversations/{}", id))?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(CopilotError::Api {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
                code: None,
            })
        }
    }

    // ===== Context API =====

    /// Add context
    #[instrument(skip(self, content))]
    pub async fn add_context(
        &self,
        source: &str,
        content: &str,
        tags: Vec<String>,
    ) -> Result<ContextItem> {
        let body = serde_json::json!({
            "source": source,
            "content": content,
            "tags": tags,
        });

        let mut req = self.http.post(self.url("/api/v1/context")?).json(&body);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// List context items
    #[instrument(skip(self))]
    pub async fn list_context(&self, tag: Option<String>) -> Result<Vec<ContextItem>> {
        let mut url = self.url("/api/v1/context")?;

        if let Some(t) = tag {
            url.query_pairs_mut().append_pair("tag", &t);
        }

        let mut req = self.http.get(url);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Search context
    #[instrument(skip(self))]
    pub async fn search_context(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ContextSearchResult>> {
        let mut url = self.url("/api/v1/context/search")?;
        url.query_pairs_mut()
            .append_pair("q", query)
            .append_pair("limit", &limit.to_string());

        let mut req = self.http.get(url);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Clear context
    #[instrument(skip(self))]
    pub async fn clear_context(&self, tag: Option<String>) -> Result<()> {
        let mut url = self.url("/api/v1/context")?;

        if let Some(t) = tag {
            url.query_pairs_mut().append_pair("tag", &t);
        }

        let mut req = self.http.delete(url);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(CopilotError::Api {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
                code: None,
            })
        }
    }

    // ===== Workflow API =====

    /// List workflows
    #[instrument(skip(self))]
    pub async fn list_workflows(&self) -> Result<Vec<WorkflowSummary>> {
        let mut req = self.http.get(self.url("/api/v1/workflows")?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Get a workflow
    #[instrument(skip(self))]
    pub async fn get_workflow(&self, id: &str) -> Result<Workflow> {
        let mut req = self
            .http
            .get(self.url(&format!("/api/v1/workflows/{}", id))?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Start a workflow execution
    #[instrument(skip(self, input))]
    pub async fn start_workflow(
        &self,
        workflow_id: &str,
        input: HashMap<String, serde_json::Value>,
    ) -> Result<WorkflowExecution> {
        let body = serde_json::json!({
            "input": input,
        });

        let mut req = self
            .http
            .post(self.url(&format!("/api/v1/workflows/{}/run", workflow_id))?)
            .json(&body);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Get workflow execution status
    #[instrument(skip(self))]
    pub async fn get_workflow_status(&self, execution_id: &str) -> Result<WorkflowStatus> {
        let mut req = self
            .http
            .get(self.url(&format!("/api/v1/executions/{}", execution_id))?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Cancel a workflow execution
    #[instrument(skip(self))]
    pub async fn cancel_workflow(&self, execution_id: &str) -> Result<()> {
        let mut req = self
            .http
            .post(self.url(&format!("/api/v1/executions/{}/cancel", execution_id))?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(CopilotError::Api {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
                code: None,
            })
        }
    }

    // ===== Sandbox API =====

    /// List sandboxes
    #[instrument(skip(self))]
    pub async fn list_sandboxes(&self) -> Result<Vec<Sandbox>> {
        let mut req = self.http.get(self.url("/api/v1/sandboxes")?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Get sandbox status
    #[instrument(skip(self))]
    pub async fn get_sandbox(&self, id: &str) -> Result<Sandbox> {
        let mut req = self
            .http
            .get(self.url(&format!("/api/v1/sandboxes/{}", id))?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Execute code in a sandbox
    #[instrument(skip(self, code))]
    pub async fn execute_code(
        &self,
        code: &str,
        runtime: &str,
        timeout: u64,
    ) -> Result<ExecutionResult> {
        let body = serde_json::json!({
            "code": code,
            "runtime": runtime,
            "timeout": timeout,
        });

        let mut req = self
            .http
            .post(self.url("/api/v1/sandbox/execute")?)
            .json(&body);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Destroy a sandbox
    #[instrument(skip(self))]
    pub async fn destroy_sandbox(&self, id: &str) -> Result<()> {
        let mut req = self
            .http
            .delete(self.url(&format!("/api/v1/sandboxes/{}", id))?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(CopilotError::Api {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
                code: None,
            })
        }
    }

    // ===== Session API =====

    /// Create a new chat session
    #[instrument(skip(self))]
    pub async fn create_session(&self, model: Option<String>) -> Result<Session> {
        let body = serde_json::json!({
            "model": model,
        });

        let mut req = self
            .http
            .post(self.url("/api/v1/sessions")?)
            .json(&body);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Resume an existing session
    #[instrument(skip(self))]
    pub async fn resume_session(&self, session_id: &str) -> Result<Session> {
        let mut req = self
            .http
            .get(self.url(&format!("/api/v1/sessions/{}", session_id))?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Send a message in a session
    #[instrument(skip(self, message))]
    pub async fn send_message(
        &self,
        session_id: &str,
        message: impl Into<String>,
    ) -> Result<ChatResponse> {
        let body = serde_json::json!({
            "message": message.into(),
        });

        let mut req = self
            .http
            .post(self.url(&format!("/api/v1/sessions/{}/messages", session_id))?)
            .json(&body);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Get session history
    #[instrument(skip(self))]
    pub async fn get_history(&self, session_id: &str) -> Result<Vec<Message>> {
        let mut req = self
            .http
            .get(self.url(&format!("/api/v1/sessions/{}/history", session_id))?);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    // ===== Ask API =====

    /// Send a single question (stateless)
    #[instrument(skip(self, message, context))]
    pub async fn ask(
        &self,
        message: impl Into<String>,
        context: Option<&str>,
        model: Option<String>,
    ) -> Result<ChatResponse> {
        let body = serde_json::json!({
            "message": message.into(),
            "context": context,
            "model": model,
        });

        let mut req = self
            .http
            .post(self.url("/api/v1/ask")?)
            .json(&body);

        if let Some(auth) = self.auth_header() {
            req = req.header(header::AUTHORIZATION, auth);
        }

        let response = req.send().await.map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    // ===== Health API =====

    /// Check server health
    #[instrument(skip(self))]
    pub async fn health(&self) -> Result<HealthResponse> {
        debug!("Checking server health");
        let response = self
            .http
            .get(self.url("/health")?)
            .send()
            .await
            .map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Check server health with optional detail level
    #[instrument(skip(self))]
    pub async fn health_check(&self, detailed: bool) -> Result<HealthResponse> {
        let url = if detailed {
            self.url("/health?detailed=true")?
        } else {
            self.url("/health")?
        };

        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }

    /// Get server version
    #[instrument(skip(self))]
    pub async fn version(&self) -> Result<VersionInfo> {
        let response = self
            .http
            .get(self.url("/version")?)
            .send()
            .await
            .map_err(CopilotError::Http)?;
        self.handle_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let client = CopilotClient::builder()
            .base_url("http://localhost:8080")
            .api_key(Some("test-key".to_string()))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        assert_eq!(client.base_url().as_str(), "http://localhost:8080/");
    }

    #[test]
    fn test_url_building() {
        let client = CopilotClient::new("http://localhost:8080").unwrap();
        let url = client.url("/api/v1/chat").unwrap();
        assert_eq!(url.as_str(), "http://localhost:8080/api/v1/chat");
    }
}
