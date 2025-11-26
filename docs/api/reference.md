# LLM CoPilot Agent API Reference

This document provides a complete reference for the LLM CoPilot Agent REST API.

## Base URL

```
https://api.llmcopilot.dev/api/v1
```

For local development:
```
http://localhost:8080/api/v1
```

## Authentication

### API Key Authentication

Include your API key in the `X-API-Key` header:

```http
X-API-Key: your-api-key
```

### Bearer Token Authentication

Include your access token in the `Authorization` header:

```http
Authorization: Bearer your-access-token
```

## Common Response Codes

| Code | Description |
|------|-------------|
| 200 | Success |
| 201 | Created |
| 204 | No Content |
| 400 | Bad Request |
| 401 | Unauthorized |
| 403 | Forbidden |
| 404 | Not Found |
| 429 | Rate Limited |
| 500 | Server Error |

## Error Response Format

```json
{
  "code": "ERROR_CODE",
  "message": "Human-readable error message",
  "details": {},
  "request_id": "req-abc123"
}
```

---

## Health

### GET /health

Check the API health status.

**Response:**

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "components": {
    "database": "healthy",
    "cache": "healthy",
    "llm": "healthy"
  }
}
```

---

## Authentication

### POST /api/v1/auth/login

Authenticate with username/email and password.

**Request Body:**

```json
{
  "username_or_email": "user@example.com",
  "password": "your-password"
}
```

**Response:**

```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_expires_in": 86400,
  "user": {
    "id": "user-123",
    "username": "johndoe",
    "email": "user@example.com",
    "roles": ["user"]
  }
}
```

### POST /api/v1/auth/refresh

Refresh access tokens.

**Request Body:**

```json
{
  "refresh_token": "eyJ..."
}
```

**Response:**

```json
{
  "access_token": "eyJ...",
  "refresh_token": "eyJ...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

### POST /api/v1/auth/logout

Log out the current user.

**Response:** `204 No Content`

### GET /api/v1/auth/me

Get the current authenticated user.

**Response:**

```json
{
  "id": "user-123",
  "username": "johndoe",
  "email": "user@example.com",
  "roles": ["user"],
  "tenant_id": "tenant-456",
  "is_active": true,
  "email_verified": true,
  "created_at": "2024-01-01T00:00:00Z",
  "last_login_at": "2024-01-15T12:00:00Z"
}
```

---

## Conversations

### POST /api/v1/conversations

Create a new conversation.

**Request Body:**

```json
{
  "title": "My Conversation",
  "system_prompt": "You are a helpful assistant",
  "metadata": {
    "project": "test"
  }
}
```

**Response:**

```json
{
  "id": "conv-123",
  "title": "My Conversation",
  "user_id": "user-456",
  "tenant_id": "tenant-789",
  "metadata": {
    "project": "test"
  },
  "message_count": 0,
  "created_at": "2024-01-15T12:00:00Z",
  "updated_at": "2024-01-15T12:00:00Z"
}
```

### GET /api/v1/conversations

List conversations with pagination.

**Query Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| limit | integer | 20 | Maximum number of results |
| offset | integer | 0 | Number of results to skip |

**Response:**

```json
{
  "items": [
    {
      "id": "conv-123",
      "title": "My Conversation",
      "message_count": 5,
      "created_at": "2024-01-15T12:00:00Z"
    }
  ],
  "total": 100,
  "limit": 20,
  "offset": 0,
  "has_more": true
}
```

### GET /api/v1/conversations/{id}

Get a conversation by ID.

**Response:**

```json
{
  "id": "conv-123",
  "title": "My Conversation",
  "user_id": "user-456",
  "metadata": {},
  "message_count": 5,
  "created_at": "2024-01-15T12:00:00Z",
  "updated_at": "2024-01-15T14:30:00Z"
}
```

### DELETE /api/v1/conversations/{id}

Delete a conversation.

**Response:** `204 No Content`

---

## Messages

### POST /api/v1/conversations/{conversation_id}/messages

Send a message in a conversation.

**Request Body:**

```json
{
  "content": "Hello, how can you help me?",
  "role": "user",
  "metadata": {}
}
```

**Response:**

```json
{
  "id": "msg-789",
  "conversation_id": "conv-123",
  "role": "assistant",
  "content": "Hello! I'm here to help you with...",
  "tokens_used": 150,
  "model": "claude-3-sonnet",
  "created_at": "2024-01-15T12:01:00Z"
}
```

### POST /api/v1/conversations/{conversation_id}/messages/stream

Send a message with streaming response.

**Request Body:**

```json
{
  "content": "Explain quantum computing",
  "role": "user"
}
```

**Response:** Server-Sent Events (SSE)

```
data: {"type": "message_start", "message_id": "msg-789"}

data: {"type": "content_delta", "delta": {"text": "Quantum "}}

data: {"type": "content_delta", "delta": {"text": "computing "}}

data: {"type": "content_delta", "delta": {"text": "is..."}}

data: {"type": "message_end", "message_id": "msg-789"}

data: [DONE]
```

### GET /api/v1/conversations/{conversation_id}/messages

List messages in a conversation.

**Query Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| limit | integer | 50 | Maximum number of results |
| offset | integer | 0 | Number of results to skip |

**Response:**

```json
{
  "items": [
    {
      "id": "msg-789",
      "role": "user",
      "content": "Hello!",
      "created_at": "2024-01-15T12:00:00Z"
    },
    {
      "id": "msg-790",
      "role": "assistant",
      "content": "Hello! How can I help?",
      "created_at": "2024-01-15T12:00:05Z"
    }
  ],
  "total": 10,
  "limit": 50,
  "offset": 0
}
```

---

## Workflows

### POST /api/v1/workflows

Create a new workflow definition.

**Request Body:**

```json
{
  "name": "Code Review Pipeline",
  "description": "Automated code review workflow",
  "version": "1.0.0",
  "steps": [
    {
      "id": "analyze",
      "name": "Analyze Code",
      "type": "llm",
      "config": {
        "prompt": "Analyze the following code for issues..."
      },
      "next_steps": ["suggest"]
    },
    {
      "id": "suggest",
      "name": "Suggest Improvements",
      "type": "llm",
      "config": {
        "prompt": "Based on the analysis, suggest improvements..."
      }
    }
  ],
  "entry_point": "analyze"
}
```

**Response:**

```json
{
  "id": "wf-123",
  "name": "Code Review Pipeline",
  "description": "Automated code review workflow",
  "version": "1.0.0",
  "steps": [...],
  "entry_point": "analyze",
  "created_at": "2024-01-15T12:00:00Z"
}
```

### GET /api/v1/workflows

List workflow definitions.

**Response:**

```json
{
  "items": [
    {
      "id": "wf-123",
      "name": "Code Review Pipeline",
      "version": "1.0.0",
      "created_at": "2024-01-15T12:00:00Z"
    }
  ],
  "total": 5
}
```

### GET /api/v1/workflows/{id}

Get a workflow definition by ID.

### DELETE /api/v1/workflows/{id}

Delete a workflow definition.

---

## Workflow Runs

### POST /api/v1/workflows/runs

Start a new workflow run.

**Request Body:**

```json
{
  "workflow_id": "wf-123",
  "inputs": {
    "code": "def add(a, b): return a + b"
  }
}
```

**Response:**

```json
{
  "id": "run-456",
  "workflow_id": "wf-123",
  "status": "running",
  "current_step": "analyze",
  "inputs": {
    "code": "def add(a, b): return a + b"
  },
  "started_at": "2024-01-15T12:00:00Z",
  "created_at": "2024-01-15T12:00:00Z"
}
```

### GET /api/v1/workflows/runs/{id}

Get a workflow run by ID.

**Response:**

```json
{
  "id": "run-456",
  "workflow_id": "wf-123",
  "status": "completed",
  "current_step": null,
  "inputs": {...},
  "outputs": {
    "analysis": "...",
    "suggestions": "..."
  },
  "started_at": "2024-01-15T12:00:00Z",
  "completed_at": "2024-01-15T12:01:30Z"
}
```

### GET /api/v1/workflows/runs

List workflow runs.

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| workflow_id | string | Filter by workflow ID |

### POST /api/v1/workflows/runs/{id}/cancel

Cancel a running workflow.

**Response:**

```json
{
  "id": "run-456",
  "status": "cancelled"
}
```

---

## Context Items

### POST /api/v1/context

Create a context item.

**Request Body:**

```json
{
  "name": "coding-standards.md",
  "type": "document",
  "content": "# Coding Standards\n...",
  "metadata": {
    "category": "guidelines"
  }
}
```

**Response:**

```json
{
  "id": "ctx-123",
  "name": "coding-standards.md",
  "type": "document",
  "token_count": 500,
  "created_at": "2024-01-15T12:00:00Z"
}
```

### GET /api/v1/context

List context items.

### GET /api/v1/context/{id}

Get a context item by ID.

### DELETE /api/v1/context/{id}

Delete a context item.

---

## API Keys

### POST /api/v1/auth/api-keys

Create a new API key. (Admin only)

**Request Body:**

```json
{
  "name": "Production API Key",
  "scopes": ["read", "write", "chat"],
  "expires_in_days": 365
}
```

**Response:**

```json
{
  "id": "key-123",
  "name": "Production API Key",
  "key": "cpk_live_abc123...",
  "scopes": ["read", "write", "chat"],
  "expires_at": "2025-01-15T12:00:00Z",
  "created_at": "2024-01-15T12:00:00Z"
}
```

> **Note:** The `key` field is only returned when creating the API key. Store it securely as it cannot be retrieved later.

### GET /api/v1/auth/api-keys

List API keys.

### DELETE /api/v1/auth/api-keys/{id}

Revoke an API key.

---

## Rate Limiting

The API uses a token bucket algorithm for rate limiting. When rate limited, the response includes:

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 30
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1705320000
```

**Rate Limit Response:**

```json
{
  "code": "RATE_LIMITED",
  "message": "Too many requests. Please try again later.",
  "details": {
    "retry_after": 30,
    "limit": 100,
    "remaining": 0
  }
}
```

## Pagination

All list endpoints support pagination with the following parameters:

| Parameter | Type | Default | Max | Description |
|-----------|------|---------|-----|-------------|
| limit | integer | 20 | 100 | Items per page |
| offset | integer | 0 | - | Items to skip |

**Paginated Response Format:**

```json
{
  "items": [...],
  "total": 150,
  "limit": 20,
  "offset": 40,
  "has_more": true
}
```

## Webhook Events

The API can send webhook notifications for various events:

| Event | Description |
|-------|-------------|
| `conversation.created` | New conversation created |
| `message.created` | New message sent |
| `workflow.started` | Workflow run started |
| `workflow.completed` | Workflow run completed |
| `workflow.failed` | Workflow run failed |

**Webhook Payload:**

```json
{
  "event": "workflow.completed",
  "timestamp": "2024-01-15T12:01:30Z",
  "data": {
    "id": "run-456",
    "workflow_id": "wf-123",
    "status": "completed"
  }
}
```

## SDK Support

| Language | Package | Version |
|----------|---------|---------|
| TypeScript | `@llm-copilot/sdk` | 1.0.0 |
| Python | `llm-copilot` | 1.0.0 |
| Go | `github.com/llm-copilot-agent/sdk-go` | 1.0.0 |
| Java | `com.llmcopilot:copilot-sdk` | 1.0.0 |
