"""
Client classes for the LLM CoPilot Agent SDK.
"""

import asyncio
from typing import Any, Optional, Union

import httpx

from llm_copilot.exceptions import (
    ConnectionError,
    TimeoutError,
    raise_for_status,
)
from llm_copilot.models import (
    ApiKey,
    ApiKeyCreate,
    ApiKeyWithSecret,
    Conversation,
    ConversationCreate,
    ContextItem,
    ContextItemCreate,
    HealthStatus,
    LoginRequest,
    LoginResponse,
    Message,
    MessageCreate,
    RegisterRequest,
    TokenPair,
    User,
    WorkflowDefinition,
    WorkflowDefinitionCreate,
    WorkflowRun,
    WorkflowRunCreate,
)
from llm_copilot.streaming import StreamingResponse


class AsyncCoPilotClient:
    """
    Async client for the LLM CoPilot Agent API.

    Usage:
        async with AsyncCoPilotClient(base_url="http://localhost:8080") as client:
            # Login to get tokens
            response = await client.login("username", "password")

            # Create a conversation
            conversation = await client.create_conversation()

            # Send a message
            message = await client.send_message(conversation.id, "Hello!")
    """

    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        api_key: Optional[str] = None,
        access_token: Optional[str] = None,
        timeout: float = 30.0,
        verify_ssl: bool = True,
    ):
        """
        Initialize the client.

        Args:
            base_url: Base URL of the CoPilot API.
            api_key: API key for authentication (optional if using access_token).
            access_token: JWT access token (optional if using api_key).
            timeout: Request timeout in seconds.
            verify_ssl: Whether to verify SSL certificates.
        """
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key
        self.access_token = access_token
        self.timeout = timeout
        self._client: Optional[httpx.AsyncClient] = None
        self._verify_ssl = verify_ssl

    async def __aenter__(self) -> "AsyncCoPilotClient":
        """Enter async context."""
        self._client = httpx.AsyncClient(
            base_url=self.base_url,
            timeout=self.timeout,
            verify=self._verify_ssl,
        )
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit async context."""
        if self._client:
            await self._client.aclose()
            self._client = None

    @property
    def client(self) -> httpx.AsyncClient:
        """Get the HTTP client."""
        if self._client is None:
            raise RuntimeError("Client not initialized. Use 'async with' context manager.")
        return self._client

    def _get_headers(self) -> dict[str, str]:
        """Get headers for requests."""
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json",
        }
        if self.api_key:
            headers["X-API-Key"] = self.api_key
        elif self.access_token:
            headers["Authorization"] = f"Bearer {self.access_token}"
        return headers

    async def _request(
        self,
        method: str,
        path: str,
        data: Optional[dict[str, Any]] = None,
        params: Optional[dict[str, Any]] = None,
        stream: bool = False,
    ) -> Union[dict[str, Any], httpx.Response]:
        """Make an HTTP request."""
        try:
            response = await self.client.request(
                method=method,
                url=path,
                json=data,
                params=params,
                headers=self._get_headers(),
            )

            if stream:
                return response

            if response.status_code >= 400:
                try:
                    error_data = response.json()
                except Exception:
                    error_data = {"error": response.text}
                raise_for_status(response.status_code, error_data)

            if response.status_code == 204:
                return {}

            return response.json()

        except httpx.ConnectError as e:
            raise ConnectionError(f"Failed to connect to {self.base_url}: {e}")
        except httpx.TimeoutException as e:
            raise TimeoutError(f"Request timed out: {e}")

    # ==================== Authentication ====================

    async def login(
        self, username_or_email: str, password: str
    ) -> LoginResponse:
        """
        Login with username/email and password.

        Args:
            username_or_email: Username or email address.
            password: Password.

        Returns:
            LoginResponse with tokens and user info.
        """
        data = await self._request(
            "POST",
            "/api/v1/auth/login",
            data={"username_or_email": username_or_email, "password": password},
        )
        response = LoginResponse.model_validate(data)
        self.access_token = response.access_token
        return response

    async def register(
        self, username: str, email: str, password: str, tenant_id: Optional[str] = None
    ) -> User:
        """
        Register a new user.

        Args:
            username: Unique username.
            email: Email address.
            password: Password.
            tenant_id: Optional tenant ID.

        Returns:
            The created user.
        """
        request = RegisterRequest(
            username=username,
            email=email,
            password=password,
            tenant_id=tenant_id,
        )
        data = await self._request(
            "POST",
            "/api/v1/auth/register",
            data=request.model_dump(),
        )
        return User.model_validate(data)

    async def refresh_tokens(self, refresh_token: str) -> TokenPair:
        """
        Refresh access tokens.

        Args:
            refresh_token: The refresh token.

        Returns:
            New token pair.
        """
        data = await self._request(
            "POST",
            "/api/v1/auth/refresh",
            data={"refresh_token": refresh_token},
        )
        response = TokenPair.model_validate(data)
        self.access_token = response.access_token
        return response

    async def logout(self) -> None:
        """Logout and invalidate tokens."""
        await self._request("POST", "/api/v1/auth/logout")
        self.access_token = None

    async def get_current_user(self) -> User:
        """Get the current authenticated user."""
        data = await self._request("GET", "/api/v1/auth/me")
        return User.model_validate(data)

    # ==================== API Keys ====================

    async def create_api_key(self, request: ApiKeyCreate) -> ApiKeyWithSecret:
        """
        Create a new API key.

        Args:
            request: API key creation request.

        Returns:
            The created API key with the secret (only shown once).
        """
        data = await self._request(
            "POST",
            "/api/v1/api-keys",
            data=request.model_dump(),
        )
        return ApiKeyWithSecret.model_validate(data)

    async def list_api_keys(self) -> list[ApiKey]:
        """List all API keys for the current user."""
        data = await self._request("GET", "/api/v1/api-keys")
        return [ApiKey.model_validate(key) for key in data]

    async def revoke_api_key(self, key_id: str) -> None:
        """Revoke an API key."""
        await self._request("DELETE", f"/api/v1/api-keys/{key_id}")

    # ==================== Conversations ====================

    async def create_conversation(
        self, request: Optional[ConversationCreate] = None
    ) -> Conversation:
        """
        Create a new conversation.

        Args:
            request: Optional conversation creation request.

        Returns:
            The created conversation.
        """
        request = request or ConversationCreate()
        data = await self._request(
            "POST",
            "/api/v1/conversations",
            data=request.model_dump(),
        )
        return Conversation.model_validate(data)

    async def get_conversation(self, conversation_id: str) -> Conversation:
        """Get a conversation by ID."""
        data = await self._request("GET", f"/api/v1/conversations/{conversation_id}")
        return Conversation.model_validate(data)

    async def list_conversations(
        self, limit: int = 20, offset: int = 0
    ) -> list[Conversation]:
        """List conversations."""
        data = await self._request(
            "GET",
            "/api/v1/conversations",
            params={"limit": limit, "offset": offset},
        )
        return [Conversation.model_validate(conv) for conv in data.get("items", data)]

    async def delete_conversation(self, conversation_id: str) -> None:
        """Delete a conversation."""
        await self._request("DELETE", f"/api/v1/conversations/{conversation_id}")

    # ==================== Messages ====================

    async def send_message(
        self,
        conversation_id: str,
        content: str,
        stream: bool = False,
    ) -> Union[Message, StreamingResponse]:
        """
        Send a message in a conversation.

        Args:
            conversation_id: The conversation ID.
            content: Message content.
            stream: Whether to stream the response.

        Returns:
            The assistant's response message or a streaming response.
        """
        request = MessageCreate(content=content)

        if stream:
            response = await self._request(
                "POST",
                f"/api/v1/conversations/{conversation_id}/messages",
                data={**request.model_dump(), "stream": True},
                stream=True,
            )
            return StreamingResponse(response)

        data = await self._request(
            "POST",
            f"/api/v1/conversations/{conversation_id}/messages",
            data=request.model_dump(),
        )
        return Message.model_validate(data)

    async def list_messages(
        self, conversation_id: str, limit: int = 50, offset: int = 0
    ) -> list[Message]:
        """List messages in a conversation."""
        data = await self._request(
            "GET",
            f"/api/v1/conversations/{conversation_id}/messages",
            params={"limit": limit, "offset": offset},
        )
        return [Message.model_validate(msg) for msg in data.get("items", data)]

    # ==================== Workflows ====================

    async def create_workflow(
        self, request: WorkflowDefinitionCreate
    ) -> WorkflowDefinition:
        """Create a new workflow definition."""
        data = await self._request(
            "POST",
            "/api/v1/workflows",
            data=request.model_dump(),
        )
        return WorkflowDefinition.model_validate(data)

    async def get_workflow(self, workflow_id: str) -> WorkflowDefinition:
        """Get a workflow definition."""
        data = await self._request("GET", f"/api/v1/workflows/{workflow_id}")
        return WorkflowDefinition.model_validate(data)

    async def list_workflows(self) -> list[WorkflowDefinition]:
        """List workflow definitions."""
        data = await self._request("GET", "/api/v1/workflows")
        return [WorkflowDefinition.model_validate(wf) for wf in data.get("items", data)]

    async def delete_workflow(self, workflow_id: str) -> None:
        """Delete a workflow definition."""
        await self._request("DELETE", f"/api/v1/workflows/{workflow_id}")

    async def run_workflow(self, request: WorkflowRunCreate) -> WorkflowRun:
        """Start a workflow run."""
        data = await self._request(
            "POST",
            "/api/v1/workflows/runs",
            data=request.model_dump(),
        )
        return WorkflowRun.model_validate(data)

    async def get_workflow_run(self, run_id: str) -> WorkflowRun:
        """Get a workflow run."""
        data = await self._request("GET", f"/api/v1/workflows/runs/{run_id}")
        return WorkflowRun.model_validate(data)

    async def list_workflow_runs(
        self, workflow_id: Optional[str] = None
    ) -> list[WorkflowRun]:
        """List workflow runs."""
        params = {}
        if workflow_id:
            params["workflow_id"] = workflow_id
        data = await self._request("GET", "/api/v1/workflows/runs", params=params)
        return [WorkflowRun.model_validate(run) for run in data.get("items", data)]

    async def cancel_workflow_run(self, run_id: str) -> WorkflowRun:
        """Cancel a workflow run."""
        data = await self._request("POST", f"/api/v1/workflows/runs/{run_id}/cancel")
        return WorkflowRun.model_validate(data)

    # ==================== Context ====================

    async def create_context_item(
        self, request: ContextItemCreate
    ) -> ContextItem:
        """Create a context item."""
        data = await self._request(
            "POST",
            "/api/v1/context",
            data=request.model_dump(),
        )
        return ContextItem.model_validate(data)

    async def get_context_item(self, item_id: str) -> ContextItem:
        """Get a context item."""
        data = await self._request("GET", f"/api/v1/context/{item_id}")
        return ContextItem.model_validate(data)

    async def list_context_items(self) -> list[ContextItem]:
        """List context items."""
        data = await self._request("GET", "/api/v1/context")
        return [ContextItem.model_validate(item) for item in data.get("items", data)]

    async def delete_context_item(self, item_id: str) -> None:
        """Delete a context item."""
        await self._request("DELETE", f"/api/v1/context/{item_id}")

    # ==================== Health ====================

    async def health_check(self) -> HealthStatus:
        """Check API health."""
        data = await self._request("GET", "/health")
        return HealthStatus.model_validate(data)

    async def get_metrics(self) -> str:
        """Get Prometheus metrics."""
        response = await self.client.get("/metrics")
        return response.text


class CoPilotClient:
    """
    Synchronous wrapper for AsyncCoPilotClient.

    Usage:
        client = CoPilotClient(base_url="http://localhost:8080", api_key="...")

        # Create a conversation
        conversation = client.create_conversation()

        # Send a message
        message = client.send_message(conversation.id, "Hello!")
    """

    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        api_key: Optional[str] = None,
        access_token: Optional[str] = None,
        timeout: float = 30.0,
        verify_ssl: bool = True,
    ):
        """Initialize the synchronous client."""
        self._async_client = AsyncCoPilotClient(
            base_url=base_url,
            api_key=api_key,
            access_token=access_token,
            timeout=timeout,
            verify_ssl=verify_ssl,
        )
        self._loop: Optional[asyncio.AbstractEventLoop] = None

    def _get_loop(self) -> asyncio.AbstractEventLoop:
        """Get or create an event loop."""
        try:
            return asyncio.get_running_loop()
        except RuntimeError:
            if self._loop is None or self._loop.is_closed():
                self._loop = asyncio.new_event_loop()
            return self._loop

    def _run(self, coro: Any) -> Any:
        """Run a coroutine synchronously."""
        loop = self._get_loop()
        return loop.run_until_complete(coro)

    def __enter__(self) -> "CoPilotClient":
        """Enter context."""
        self._run(self._async_client.__aenter__())
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit context."""
        self._run(self._async_client.__aexit__(exc_type, exc_val, exc_tb))
        if self._loop and not self._loop.is_running():
            self._loop.close()
            self._loop = None

    # Delegate all methods to async client
    def login(self, username_or_email: str, password: str) -> LoginResponse:
        """Login with username/email and password."""
        return self._run(self._async_client.login(username_or_email, password))

    def register(
        self, username: str, email: str, password: str, tenant_id: Optional[str] = None
    ) -> User:
        """Register a new user."""
        return self._run(self._async_client.register(username, email, password, tenant_id))

    def refresh_tokens(self, refresh_token: str) -> TokenPair:
        """Refresh access tokens."""
        return self._run(self._async_client.refresh_tokens(refresh_token))

    def logout(self) -> None:
        """Logout and invalidate tokens."""
        return self._run(self._async_client.logout())

    def get_current_user(self) -> User:
        """Get the current authenticated user."""
        return self._run(self._async_client.get_current_user())

    def create_api_key(self, request: ApiKeyCreate) -> ApiKeyWithSecret:
        """Create a new API key."""
        return self._run(self._async_client.create_api_key(request))

    def list_api_keys(self) -> list[ApiKey]:
        """List all API keys."""
        return self._run(self._async_client.list_api_keys())

    def revoke_api_key(self, key_id: str) -> None:
        """Revoke an API key."""
        return self._run(self._async_client.revoke_api_key(key_id))

    def create_conversation(
        self, request: Optional[ConversationCreate] = None
    ) -> Conversation:
        """Create a new conversation."""
        return self._run(self._async_client.create_conversation(request))

    def get_conversation(self, conversation_id: str) -> Conversation:
        """Get a conversation by ID."""
        return self._run(self._async_client.get_conversation(conversation_id))

    def list_conversations(self, limit: int = 20, offset: int = 0) -> list[Conversation]:
        """List conversations."""
        return self._run(self._async_client.list_conversations(limit, offset))

    def delete_conversation(self, conversation_id: str) -> None:
        """Delete a conversation."""
        return self._run(self._async_client.delete_conversation(conversation_id))

    def send_message(self, conversation_id: str, content: str) -> Message:
        """Send a message (synchronous, non-streaming only)."""
        return self._run(self._async_client.send_message(conversation_id, content, stream=False))

    def list_messages(
        self, conversation_id: str, limit: int = 50, offset: int = 0
    ) -> list[Message]:
        """List messages in a conversation."""
        return self._run(self._async_client.list_messages(conversation_id, limit, offset))

    def create_workflow(self, request: WorkflowDefinitionCreate) -> WorkflowDefinition:
        """Create a new workflow definition."""
        return self._run(self._async_client.create_workflow(request))

    def get_workflow(self, workflow_id: str) -> WorkflowDefinition:
        """Get a workflow definition."""
        return self._run(self._async_client.get_workflow(workflow_id))

    def list_workflows(self) -> list[WorkflowDefinition]:
        """List workflow definitions."""
        return self._run(self._async_client.list_workflows())

    def delete_workflow(self, workflow_id: str) -> None:
        """Delete a workflow definition."""
        return self._run(self._async_client.delete_workflow(workflow_id))

    def run_workflow(self, request: WorkflowRunCreate) -> WorkflowRun:
        """Start a workflow run."""
        return self._run(self._async_client.run_workflow(request))

    def get_workflow_run(self, run_id: str) -> WorkflowRun:
        """Get a workflow run."""
        return self._run(self._async_client.get_workflow_run(run_id))

    def list_workflow_runs(self, workflow_id: Optional[str] = None) -> list[WorkflowRun]:
        """List workflow runs."""
        return self._run(self._async_client.list_workflow_runs(workflow_id))

    def cancel_workflow_run(self, run_id: str) -> WorkflowRun:
        """Cancel a workflow run."""
        return self._run(self._async_client.cancel_workflow_run(run_id))

    def create_context_item(self, request: ContextItemCreate) -> ContextItem:
        """Create a context item."""
        return self._run(self._async_client.create_context_item(request))

    def get_context_item(self, item_id: str) -> ContextItem:
        """Get a context item."""
        return self._run(self._async_client.get_context_item(item_id))

    def list_context_items(self) -> list[ContextItem]:
        """List context items."""
        return self._run(self._async_client.list_context_items())

    def delete_context_item(self, item_id: str) -> None:
        """Delete a context item."""
        return self._run(self._async_client.delete_context_item(item_id))

    def health_check(self) -> HealthStatus:
        """Check API health."""
        return self._run(self._async_client.health_check())
