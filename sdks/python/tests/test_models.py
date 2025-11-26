"""Tests for SDK models."""

from datetime import datetime
from llm_copilot.models import (
    Message,
    MessageRole,
    Conversation,
    WorkflowStatus,
    ContextType,
    ApiKeyScope,
    LoginResponse,
    User,
)


def test_message_model():
    """Test Message model."""
    message = Message(
        id="msg-123",
        conversation_id="conv-456",
        role=MessageRole.USER,
        content="Hello, world!",
        created_at=datetime.now(),
    )

    assert message.id == "msg-123"
    assert message.role == MessageRole.USER
    assert message.content == "Hello, world!"


def test_conversation_model():
    """Test Conversation model."""
    now = datetime.now()
    conversation = Conversation(
        id="conv-123",
        user_id="user-456",
        message_count=5,
        created_at=now,
        updated_at=now,
    )

    assert conversation.id == "conv-123"
    assert conversation.message_count == 5


def test_workflow_status_enum():
    """Test WorkflowStatus enum."""
    assert WorkflowStatus.PENDING == "pending"
    assert WorkflowStatus.RUNNING == "running"
    assert WorkflowStatus.COMPLETED == "completed"
    assert WorkflowStatus.FAILED == "failed"


def test_context_type_enum():
    """Test ContextType enum."""
    assert ContextType.FILE == "file"
    assert ContextType.URL == "url"
    assert ContextType.CODE == "code"


def test_api_key_scope_enum():
    """Test ApiKeyScope enum."""
    assert ApiKeyScope.READ == "read"
    assert ApiKeyScope.WRITE == "write"
    assert ApiKeyScope.ADMIN == "admin"


def test_login_response_model():
    """Test LoginResponse model."""
    now = datetime.now()
    response = LoginResponse(
        access_token="access-token",
        refresh_token="refresh-token",
        expires_in=900,
        refresh_expires_in=604800,
        user=User(
            id="user-123",
            username="testuser",
            email="test@example.com",
            created_at=now,
        ),
    )

    assert response.access_token == "access-token"
    assert response.user.username == "testuser"
