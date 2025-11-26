# Getting Started with LLM CoPilot Agent

This guide will help you get up and running with the LLM CoPilot Agent in minutes.

## Prerequisites

- Docker and Docker Compose (for local development)
- Node.js 18+ (for TypeScript SDK)
- Python 3.11+ (for Python SDK)
- Go 1.21+ (for Go SDK)
- Java 17+ (for Java SDK)

## Quick Start

### Option 1: Using Docker Compose (Recommended)

The fastest way to get started is using Docker Compose:

```bash
# Clone the repository
git clone https://github.com/llm-copilot-agent/llm-copilot-agent.git
cd llm-copilot-agent

# Start all services
docker-compose up -d

# Check the health status
curl http://localhost:8080/health
```

### Option 2: Manual Installation

1. **Start the Backend Services**

   ```bash
   # Start PostgreSQL
   docker run -d --name copilot-postgres \
     -e POSTGRES_USER=copilot \
     -e POSTGRES_PASSWORD=copilot \
     -e POSTGRES_DB=copilot \
     -p 5432:5432 \
     postgres:16

   # Start Redis
   docker run -d --name copilot-redis \
     -p 6379:6379 \
     redis:7-alpine

   # Start the API server
   cd backend
   npm install
   npm run start
   ```

2. **Verify the Installation**

   ```bash
   curl http://localhost:8080/health
   ```

   Expected response:
   ```json
   {
     "status": "healthy",
     "version": "1.0.0",
     "components": {
       "database": "healthy",
       "cache": "healthy",
       "llm": "healthy"
     }
   }
   ```

## Authentication

The CoPilot Agent supports two authentication methods:

### API Key Authentication

Best for server-to-server communication:

```bash
# Create an API key (admin only)
curl -X POST http://localhost:8080/api/v1/auth/api-keys \
  -H "Authorization: Bearer <admin-token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-api-key", "scopes": ["read", "write", "chat"]}'
```

Use the API key in requests:

```bash
curl http://localhost:8080/api/v1/conversations \
  -H "X-API-Key: <your-api-key>"
```

### JWT Token Authentication

Best for user-facing applications:

```bash
# Login to get tokens
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username_or_email": "user@example.com", "password": "your-password"}'
```

Use the access token in requests:

```bash
curl http://localhost:8080/api/v1/conversations \
  -H "Authorization: Bearer <access-token>"
```

## Using the SDKs

### TypeScript/JavaScript

```typescript
import { CoPilotClient } from '@llm-copilot/sdk';

// Create a client
const client = new CoPilotClient({
  baseUrl: 'http://localhost:8080',
  apiKey: 'your-api-key',
});

// Create a conversation
const conversation = await client.conversations.create();

// Send a message
const response = await client.messages.send(conversation.id, {
  content: 'Hello! What can you help me with?',
});

console.log(response.content);
```

### Python

```python
from llm_copilot import CoPilotClient

# Create a client
client = CoPilotClient(
    base_url="http://localhost:8080",
    api_key="your-api-key"
)

# Create a conversation
conversation = await client.create_conversation()

# Send a message
response = await client.send_message(
    conversation_id=conversation.id,
    content="Hello! What can you help me with?"
)

print(response.content)
```

### Go

```go
package main

import (
    "context"
    "fmt"
    "github.com/llm-copilot-agent/sdk-go/copilot"
)

func main() {
    // Create a client
    client := copilot.NewClient(
        "http://localhost:8080",
        copilot.WithAPIKey("your-api-key"),
    )

    ctx := context.Background()

    // Create a conversation
    conv, err := client.CreateConversation(ctx, nil)
    if err != nil {
        panic(err)
    }

    // Send a message
    msg, err := client.SendMessage(ctx, conv.ID, "Hello! What can you help me with?")
    if err != nil {
        panic(err)
    }

    fmt.Println(msg.Content)
}
```

### Java

```java
import com.llmcopilot.sdk.client.CoPilotClient;
import com.llmcopilot.sdk.models.*;

public class Example {
    public static void main(String[] args) {
        // Create a client
        CoPilotClient client = CoPilotClient.builder()
            .baseUrl("http://localhost:8080")
            .apiKey("your-api-key")
            .build();

        // Create a conversation
        Conversation conversation = client.createConversation(null);

        // Send a message
        Message response = client.sendMessage(
            conversation.getId(),
            "Hello! What can you help me with?"
        );

        System.out.println(response.getContent());

        // Clean up
        client.close();
    }
}
```

## Core Concepts

### Conversations

Conversations are the primary way to interact with the CoPilot Agent. Each conversation maintains context across multiple messages, allowing for coherent multi-turn interactions.

```python
# Create a conversation with a system prompt
conversation = await client.create_conversation(
    title="Code Review Assistant",
    system_prompt="You are a helpful code review assistant specializing in Python best practices."
)
```

### Messages

Messages are the individual exchanges within a conversation. Each message has a role (user, assistant, or system) and content.

```python
# Send a user message and get an assistant response
response = await client.send_message(
    conversation_id=conversation.id,
    content="Review this function: def add(a, b): return a + b"
)
```

### Streaming

For real-time responses, use streaming to receive content as it's generated:

```python
# Stream a response
async for chunk in client.send_message_stream(conversation_id, content):
    print(chunk.content, end="", flush=True)
```

### Workflows

Workflows allow you to define complex, multi-step AI operations:

```python
# Create a workflow definition
workflow = await client.create_workflow(
    name="Code Analysis Pipeline",
    steps=[
        {"id": "analyze", "type": "llm", "config": {"prompt": "Analyze the code"}},
        {"id": "suggest", "type": "llm", "config": {"prompt": "Suggest improvements"}},
    ],
    entry_point="analyze"
)

# Run the workflow
run = await client.run_workflow(workflow.id, inputs={"code": "..."})
```

### Context Items

Context items provide additional knowledge to the AI:

```python
# Add a document as context
context = await client.create_context_item(
    name="coding-standards.md",
    type="document",
    content="# Our Coding Standards\n..."
)

# Use in a conversation
conversation = await client.create_conversation(
    context_ids=[context.id]
)
```

## Error Handling

All SDKs provide consistent error handling:

```python
from llm_copilot import CoPilotClient, AuthenticationError, RateLimitError, ServerError

try:
    response = await client.send_message(conversation_id, content)
except AuthenticationError:
    print("Invalid credentials or expired token")
except RateLimitError as e:
    print(f"Rate limited. Retry after {e.retry_after} seconds")
except ServerError:
    print("Server error. Please try again later")
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `COPILOT_BASE_URL` | API base URL | `http://localhost:8080` |
| `COPILOT_API_KEY` | API key for authentication | - |
| `COPILOT_TIMEOUT` | Request timeout in seconds | `30` |
| `COPILOT_MAX_RETRIES` | Maximum retry attempts | `3` |

### Client Configuration

```python
from llm_copilot import CoPilotClient, RetryConfig

client = CoPilotClient(
    base_url="http://localhost:8080",
    api_key="your-api-key",
    timeout=60.0,
    retry_config=RetryConfig(
        max_retries=5,
        initial_delay=1.0,
        max_delay=30.0
    )
)
```

## Next Steps

- [API Reference](../api/reference.md) - Complete API documentation
- [Architecture Overview](../architecture/overview.md) - System architecture
- [Deployment Guide](../deployment/production.md) - Production deployment
- [CLI Reference](../cli/reference.md) - Command-line interface
