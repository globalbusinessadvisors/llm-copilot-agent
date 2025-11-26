"""
Data models for the LLM CoPilot Agent SDK.
"""

from datetime import datetime
from enum import Enum
from typing import Any, Optional
from pydantic import BaseModel, ConfigDict, Field


class MessageRole(str, Enum):
    """Role of a message in a conversation."""

    USER = "user"
    ASSISTANT = "assistant"
    SYSTEM = "system"


class Message(BaseModel):
    """A single message in a conversation."""

    model_config = ConfigDict(from_attributes=True)

    id: str
    conversation_id: str
    role: MessageRole
    content: str
    metadata: dict[str, Any] = Field(default_factory=dict)
    created_at: datetime


class MessageCreate(BaseModel):
    """Request to create a new message."""

    role: MessageRole = MessageRole.USER
    content: str
    metadata: dict[str, Any] = Field(default_factory=dict)


class Conversation(BaseModel):
    """A conversation session."""

    model_config = ConfigDict(from_attributes=True)

    id: str
    title: Optional[str] = None
    user_id: str
    tenant_id: Optional[str] = None
    metadata: dict[str, Any] = Field(default_factory=dict)
    message_count: int = 0
    created_at: datetime
    updated_at: datetime


class ConversationCreate(BaseModel):
    """Request to create a new conversation."""

    title: Optional[str] = None
    metadata: dict[str, Any] = Field(default_factory=dict)
    system_prompt: Optional[str] = None


class WorkflowStatus(str, Enum):
    """Status of a workflow run."""

    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"


class WorkflowStepType(str, Enum):
    """Type of workflow step."""

    LLM = "llm"
    TOOL = "tool"
    CONDITION = "condition"
    PARALLEL = "parallel"
    LOOP = "loop"
    HUMAN_REVIEW = "human_review"


class WorkflowStep(BaseModel):
    """A step in a workflow definition."""

    id: str
    name: str
    type: WorkflowStepType
    config: dict[str, Any] = Field(default_factory=dict)
    next_steps: list[str] = Field(default_factory=list)
    on_error: Optional[str] = None


class WorkflowDefinition(BaseModel):
    """A workflow definition."""

    model_config = ConfigDict(from_attributes=True)

    id: str
    name: str
    description: Optional[str] = None
    version: str = "1.0.0"
    steps: list[WorkflowStep] = Field(default_factory=list)
    entry_point: str
    metadata: dict[str, Any] = Field(default_factory=dict)
    created_at: datetime
    updated_at: datetime


class WorkflowDefinitionCreate(BaseModel):
    """Request to create a workflow definition."""

    name: str
    description: Optional[str] = None
    version: str = "1.0.0"
    steps: list[WorkflowStep] = Field(default_factory=list)
    entry_point: str
    metadata: dict[str, Any] = Field(default_factory=dict)


class WorkflowRun(BaseModel):
    """A workflow run instance."""

    model_config = ConfigDict(from_attributes=True)

    id: str
    workflow_id: str
    status: WorkflowStatus
    input_data: dict[str, Any] = Field(default_factory=dict)
    output_data: Optional[dict[str, Any]] = None
    error: Optional[str] = None
    current_step: Optional[str] = None
    started_at: datetime
    completed_at: Optional[datetime] = None


class WorkflowRunCreate(BaseModel):
    """Request to start a workflow run."""

    workflow_id: str
    input_data: dict[str, Any] = Field(default_factory=dict)


class ContextType(str, Enum):
    """Type of context item."""

    FILE = "file"
    URL = "url"
    TEXT = "text"
    CODE = "code"
    DOCUMENT = "document"


class ContextItem(BaseModel):
    """A context item for conversation or workflow."""

    model_config = ConfigDict(from_attributes=True)

    id: str
    type: ContextType
    name: str
    content: Optional[str] = None
    url: Optional[str] = None
    metadata: dict[str, Any] = Field(default_factory=dict)
    embedding_id: Optional[str] = None
    created_at: datetime


class ContextItemCreate(BaseModel):
    """Request to create a context item."""

    type: ContextType
    name: str
    content: Optional[str] = None
    url: Optional[str] = None
    metadata: dict[str, Any] = Field(default_factory=dict)


class User(BaseModel):
    """A user."""

    model_config = ConfigDict(from_attributes=True)

    id: str
    username: str
    email: str
    roles: list[str] = Field(default_factory=list)
    tenant_id: Optional[str] = None
    is_active: bool = True
    email_verified: bool = False
    created_at: datetime
    last_login_at: Optional[datetime] = None


class LoginRequest(BaseModel):
    """Login request."""

    username_or_email: str
    password: str


class RegisterRequest(BaseModel):
    """Registration request."""

    username: str
    email: str
    password: str
    tenant_id: Optional[str] = None


class TokenPair(BaseModel):
    """Access and refresh token pair."""

    access_token: str
    refresh_token: str
    token_type: str = "Bearer"
    expires_in: int
    refresh_expires_in: int


class LoginResponse(BaseModel):
    """Login response with tokens and user info."""

    access_token: str
    refresh_token: str
    token_type: str = "Bearer"
    expires_in: int
    refresh_expires_in: int
    user: User


class ApiKeyScope(str, Enum):
    """API key scope."""

    READ = "read"
    WRITE = "write"
    CHAT = "chat"
    WORKFLOWS = "workflows"
    CONTEXT = "context"
    SANDBOX = "sandbox"
    ADMIN = "admin"


class ApiKeyCreate(BaseModel):
    """Request to create an API key."""

    name: str
    scopes: list[ApiKeyScope] = Field(default_factory=lambda: [ApiKeyScope.READ, ApiKeyScope.CHAT])
    expires_in_days: Optional[int] = 365


class ApiKey(BaseModel):
    """API key information (key value only returned on creation)."""

    id: str
    name: str
    prefix: str
    scopes: list[ApiKeyScope]
    created_at: datetime
    expires_at: Optional[datetime] = None
    last_used_at: Optional[datetime] = None
    is_active: bool = True
    request_count: int = 0


class ApiKeyWithSecret(ApiKey):
    """API key with the secret key (only returned on creation)."""

    key: str


class HealthStatus(BaseModel):
    """Health status response."""

    status: str
    version: str
    uptime_seconds: float
    components: dict[str, str] = Field(default_factory=dict)
