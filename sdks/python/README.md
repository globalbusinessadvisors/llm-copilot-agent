# LLM CoPilot Agent Python SDK

Official Python SDK for the LLM CoPilot Agent API.

## Installation

```bash
pip install llm-copilot
```

Or install from source:

```bash
cd sdks/python
pip install -e .
```

## Quick Start

### Authentication

```python
from llm_copilot import CoPilotClient

# Using API key
client = CoPilotClient(
    base_url="http://localhost:8080",
    api_key="cplt_v1_your_api_key"
)

# Or using username/password
with CoPilotClient(base_url="http://localhost:8080") as client:
    response = client.login("username", "password")
    print(f"Logged in as: {response.user.username}")
```

### Async Client

```python
import asyncio
from llm_copilot import AsyncCoPilotClient

async def main():
    async with AsyncCoPilotClient(
        base_url="http://localhost:8080",
        api_key="cplt_v1_your_api_key"
    ) as client:
        # Create a conversation
        conversation = await client.create_conversation()

        # Send a message
        message = await client.send_message(
            conversation.id,
            "Write a Python function to calculate fibonacci numbers"
        )

        print(f"Assistant: {message.content}")

asyncio.run(main())
```

### Streaming Responses

```python
async with AsyncCoPilotClient(base_url="http://localhost:8080", api_key="...") as client:
    conversation = await client.create_conversation()

    # Stream the response
    stream = await client.send_message(
        conversation.id,
        "Explain quantum computing",
        stream=True
    )

    async for event in stream:
        if event.content:
            print(event.content, end="", flush=True)

    print()  # Newline at the end
```

### Workflows

```python
from llm_copilot import CoPilotClient
from llm_copilot.models import (
    WorkflowDefinitionCreate,
    WorkflowStep,
    WorkflowStepType,
    WorkflowRunCreate,
)

with CoPilotClient(base_url="http://localhost:8080", api_key="...") as client:
    # Create a workflow
    workflow = client.create_workflow(
        WorkflowDefinitionCreate(
            name="Code Review",
            description="Automated code review workflow",
            entry_point="analyze",
            steps=[
                WorkflowStep(
                    id="analyze",
                    name="Analyze Code",
                    type=WorkflowStepType.LLM,
                    config={"prompt": "Analyze this code for issues: {code}"},
                    next_steps=["report"],
                ),
                WorkflowStep(
                    id="report",
                    name="Generate Report",
                    type=WorkflowStepType.LLM,
                    config={"prompt": "Generate a review report based on: {analysis}"},
                ),
            ],
        )
    )

    # Run the workflow
    run = client.run_workflow(
        WorkflowRunCreate(
            workflow_id=workflow.id,
            input_data={"code": "def foo(): pass"},
        )
    )

    print(f"Workflow run started: {run.id}")
```

### Context Management

```python
from llm_copilot import CoPilotClient
from llm_copilot.models import ContextItemCreate, ContextType

with CoPilotClient(base_url="http://localhost:8080", api_key="...") as client:
    # Add context from a file
    context = client.create_context_item(
        ContextItemCreate(
            type=ContextType.CODE,
            name="main.py",
            content="def main():\n    print('Hello, World!')",
        )
    )

    # List all context items
    items = client.list_context_items()
    for item in items:
        print(f"- {item.name} ({item.type})")
```

### API Key Management

```python
from llm_copilot import CoPilotClient
from llm_copilot.models import ApiKeyCreate, ApiKeyScope

with CoPilotClient(base_url="http://localhost:8080", api_key="...") as client:
    # Create a new API key
    new_key = client.create_api_key(
        ApiKeyCreate(
            name="CI/CD Key",
            scopes=[ApiKeyScope.READ, ApiKeyScope.CHAT, ApiKeyScope.WORKFLOWS],
            expires_in_days=90,
        )
    )

    # IMPORTANT: Save the key now, it won't be shown again!
    print(f"New API key: {new_key.key}")

    # List all API keys
    keys = client.list_api_keys()
    for key in keys:
        print(f"- {key.name}: {key.prefix}")
```

## Error Handling

```python
from llm_copilot import CoPilotClient
from llm_copilot.exceptions import (
    AuthenticationError,
    AuthorizationError,
    NotFoundError,
    RateLimitError,
    ValidationError,
)

with CoPilotClient(base_url="http://localhost:8080", api_key="...") as client:
    try:
        conversation = client.get_conversation("non-existent-id")
    except AuthenticationError as e:
        print(f"Authentication failed: {e}")
    except AuthorizationError as e:
        print(f"Not authorized: {e}")
    except NotFoundError as e:
        print(f"Not found: {e}")
    except RateLimitError as e:
        print(f"Rate limited, retry after {e.retry_after}s")
    except ValidationError as e:
        print(f"Validation error: {e}")
        for error in e.errors:
            print(f"  - {error['field']}: {error['message']}")
```

## Configuration

### Environment Variables

You can also configure the client using environment variables:

```bash
export COPILOT_API_URL="http://localhost:8080"
export COPILOT_API_KEY="cplt_v1_your_api_key"
```

```python
import os
from llm_copilot import CoPilotClient

client = CoPilotClient(
    base_url=os.getenv("COPILOT_API_URL", "http://localhost:8080"),
    api_key=os.getenv("COPILOT_API_KEY"),
)
```

### Custom Timeout

```python
client = CoPilotClient(
    base_url="http://localhost:8080",
    api_key="...",
    timeout=60.0,  # 60 seconds
)
```

### Disable SSL Verification (Development Only)

```python
client = CoPilotClient(
    base_url="https://dev-api.example.com",
    api_key="...",
    verify_ssl=False,  # Not recommended for production!
)
```

## Development

### Setup

```bash
cd sdks/python
pip install -e ".[dev]"
```

### Running Tests

```bash
pytest
```

### Type Checking

```bash
mypy llm_copilot
```

### Linting

```bash
ruff check .
black --check .
```

## License

See LICENSE.md in the repository root.
