"""
Tests for the CoPilot client.
"""

import pytest
from datetime import datetime
from unittest.mock import AsyncMock, MagicMock, patch

import httpx

from llm_copilot import (
    AsyncCoPilotClient,
    CoPilotClient,
    Conversation,
    Message,
    MessageRole,
    User,
    LoginResponse,
    CoPilotError,
    AuthenticationError,
    NotFoundError,
    RateLimitError,
    ServerError,
)


class TestAsyncCoPilotClient:
    """Tests for AsyncCoPilotClient."""

    @pytest.fixture
    def mock_response(self):
        """Create a mock response."""
        response = MagicMock(spec=httpx.Response)
        response.status_code = 200
        response.headers = {}
        return response

    @pytest.mark.asyncio
    async def test_client_context_manager(self):
        """Test client can be used as context manager."""
        async with AsyncCoPilotClient(base_url="http://localhost:8080") as client:
            assert client._client is not None
        assert client._client is None

    @pytest.mark.asyncio
    async def test_client_not_initialized_error(self):
        """Test error when client not initialized."""
        client = AsyncCoPilotClient(base_url="http://localhost:8080")
        with pytest.raises(RuntimeError, match="Client not initialized"):
            _ = client.client

    def test_get_headers_with_api_key(self):
        """Test headers include API key."""
        client = AsyncCoPilotClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        )
        headers = client._get_headers()
        assert headers["X-API-Key"] == "test-key"
        assert "Authorization" not in headers

    def test_get_headers_with_access_token(self):
        """Test headers include access token."""
        client = AsyncCoPilotClient(
            base_url="http://localhost:8080",
            access_token="test-token",
        )
        headers = client._get_headers()
        assert headers["Authorization"] == "Bearer test-token"
        assert "X-API-Key" not in headers

    @pytest.mark.asyncio
    async def test_login_success(self):
        """Test successful login."""
        mock_data = {
            "access_token": "access-123",
            "refresh_token": "refresh-456",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_expires_in": 86400,
            "user": {
                "id": "user-1",
                "username": "testuser",
                "email": "test@example.com",
                "roles": ["user"],
                "is_active": True,
                "email_verified": True,
                "created_at": "2024-01-01T00:00:00Z",
            },
        }

        async with AsyncCoPilotClient(base_url="http://localhost:8080") as client:
            with patch.object(client, "_request", new_callable=AsyncMock) as mock_request:
                mock_request.return_value = mock_data

                response = await client.login("testuser", "password123")

                assert response.access_token == "access-123"
                assert response.user.username == "testuser"
                assert client.access_token == "access-123"

    @pytest.mark.asyncio
    async def test_create_conversation(self):
        """Test creating a conversation."""
        mock_data = {
            "id": "conv-123",
            "user_id": "user-1",
            "message_count": 0,
            "metadata": {},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
        }

        async with AsyncCoPilotClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        ) as client:
            with patch.object(client, "_request", new_callable=AsyncMock) as mock_request:
                mock_request.return_value = mock_data

                conversation = await client.create_conversation()

                assert conversation.id == "conv-123"
                mock_request.assert_called_once()

    @pytest.mark.asyncio
    async def test_send_message(self):
        """Test sending a message."""
        mock_data = {
            "id": "msg-123",
            "conversation_id": "conv-123",
            "role": "assistant",
            "content": "Hello! How can I help?",
            "metadata": {},
            "created_at": "2024-01-01T00:00:00Z",
        }

        async with AsyncCoPilotClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        ) as client:
            with patch.object(client, "_request", new_callable=AsyncMock) as mock_request:
                mock_request.return_value = mock_data

                message = await client.send_message("conv-123", "Hello!")

                assert message.content == "Hello! How can I help?"
                assert message.role == MessageRole.ASSISTANT


class TestErrorHandling:
    """Tests for error handling."""

    @pytest.mark.asyncio
    async def test_authentication_error(self):
        """Test 401 raises AuthenticationError."""
        from llm_copilot.exceptions import raise_for_status

        with pytest.raises(AuthenticationError) as exc_info:
            raise_for_status(401, {"error": "Invalid credentials"})
        assert exc_info.value.status_code == 401

    @pytest.mark.asyncio
    async def test_not_found_error(self):
        """Test 404 raises NotFoundError."""
        from llm_copilot.exceptions import raise_for_status

        with pytest.raises(NotFoundError) as exc_info:
            raise_for_status(404, {"error": "Resource not found"})
        assert exc_info.value.status_code == 404

    @pytest.mark.asyncio
    async def test_rate_limit_error(self):
        """Test 429 raises RateLimitError."""
        from llm_copilot.exceptions import raise_for_status

        with pytest.raises(RateLimitError) as exc_info:
            raise_for_status(429, {"error": "Rate limited", "retry_after": 60})
        assert exc_info.value.status_code == 429
        assert exc_info.value.retry_after == 60

    @pytest.mark.asyncio
    async def test_server_error(self):
        """Test 500 raises ServerError."""
        from llm_copilot.exceptions import raise_for_status

        with pytest.raises(ServerError) as exc_info:
            raise_for_status(500, {"error": "Internal server error"})
        assert exc_info.value.status_code == 500


class TestSyncClient:
    """Tests for synchronous CoPilotClient."""

    def test_sync_client_delegates_to_async(self):
        """Test sync client properly delegates to async client."""
        client = CoPilotClient(
            base_url="http://localhost:8080",
            api_key="test-key",
        )
        assert client._async_client.api_key == "test-key"
        assert client._async_client.base_url == "http://localhost:8080"

    def test_sync_client_context_manager(self):
        """Test sync client can be used as context manager."""
        with patch.object(
            AsyncCoPilotClient, "__aenter__", new_callable=AsyncMock
        ) as mock_enter:
            with patch.object(
                AsyncCoPilotClient, "__aexit__", new_callable=AsyncMock
            ) as mock_exit:
                mock_enter.return_value = AsyncCoPilotClient(base_url="http://localhost:8080")

                with CoPilotClient(base_url="http://localhost:8080") as client:
                    pass

                mock_enter.assert_called_once()
                mock_exit.assert_called_once()
