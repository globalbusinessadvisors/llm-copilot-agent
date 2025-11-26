"""
LLM CoPilot Agent Python SDK

A comprehensive Python client for interacting with the LLM CoPilot Agent API.

Example usage:
    from llm_copilot import CoPilotClient, AsyncCoPilotClient

    # Synchronous client
    with CoPilotClient(base_url="http://localhost:8080", api_key="your-key") as client:
        conversation = client.create_conversation()
        response = client.send_message(conversation.id, "Hello!")
        print(response.content)

    # Async client
    async with AsyncCoPilotClient(base_url="http://localhost:8080", api_key="your-key") as client:
        conversation = await client.create_conversation()
        response = await client.send_message(conversation.id, "Hello!", stream=True)
        async for event in response:
            print(event.content, end="")
"""

from llm_copilot.client import CoPilotClient, AsyncCoPilotClient
from llm_copilot.models import (
    Message,
    MessageRole,
    MessageCreate,
    Conversation,
    ConversationCreate,
    WorkflowDefinition,
    WorkflowDefinitionCreate,
    WorkflowRun,
    WorkflowRunCreate,
    WorkflowStatus,
    WorkflowStep,
    WorkflowStepType,
    ContextItem,
    ContextItemCreate,
    ContextType,
    User,
    LoginRequest,
    LoginResponse,
    RegisterRequest,
    TokenPair,
    ApiKeyCreate,
    ApiKey,
    ApiKeyScope,
    ApiKeyWithSecret,
    HealthStatus,
)
from llm_copilot.exceptions import (
    CoPilotError,
    AuthenticationError,
    AuthorizationError,
    NotFoundError,
    ValidationError,
    RateLimitError,
    ServerError,
    ConnectionError,
    TimeoutError,
)
from llm_copilot.streaming import StreamingResponse, StreamEvent, StreamEventType
from llm_copilot.retry import RetryConfig, with_retry
from llm_copilot.documents import (
    AsyncDocumentsClient,
    DocumentsClient,
    Document,
    DocumentCreate,
    DocumentUpdate,
    DocumentChunk,
    DocumentStatus,
    ChunkingStrategy,
    SearchResult,
    SearchResultItem,
    SearchOptions,
)

__version__ = "0.1.0"
__all__ = [
    # Client classes
    "CoPilotClient",
    "AsyncCoPilotClient",
    "DocumentsClient",
    "AsyncDocumentsClient",
    # Message models
    "Message",
    "MessageRole",
    "MessageCreate",
    # Conversation models
    "Conversation",
    "ConversationCreate",
    # Workflow models
    "WorkflowDefinition",
    "WorkflowDefinitionCreate",
    "WorkflowRun",
    "WorkflowRunCreate",
    "WorkflowStatus",
    "WorkflowStep",
    "WorkflowStepType",
    # Context models
    "ContextItem",
    "ContextItemCreate",
    "ContextType",
    # Document models
    "Document",
    "DocumentCreate",
    "DocumentUpdate",
    "DocumentChunk",
    "DocumentStatus",
    "ChunkingStrategy",
    "SearchResult",
    "SearchResultItem",
    "SearchOptions",
    # Auth models
    "User",
    "LoginRequest",
    "LoginResponse",
    "RegisterRequest",
    "TokenPair",
    "ApiKeyCreate",
    "ApiKey",
    "ApiKeyScope",
    "ApiKeyWithSecret",
    "HealthStatus",
    # Exceptions
    "CoPilotError",
    "AuthenticationError",
    "AuthorizationError",
    "NotFoundError",
    "ValidationError",
    "RateLimitError",
    "ServerError",
    "ConnectionError",
    "TimeoutError",
    # Streaming
    "StreamingResponse",
    "StreamEvent",
    "StreamEventType",
    # Retry
    "RetryConfig",
    "with_retry",
]
