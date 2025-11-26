# LLM CoPilot CLI Reference

The `copilot` command-line interface provides powerful tools for interacting with the LLM CoPilot Agent from your terminal.

## Installation

### Using npm (Node.js)

```bash
npm install -g @llm-copilot/cli
```

### Using pip (Python)

```bash
pip install llm-copilot-cli
```

### Using Homebrew (macOS)

```bash
brew tap llm-copilot/tap
brew install copilot-cli
```

### Binary Download

Download the latest binary from the [releases page](https://github.com/llm-copilot-agent/cli/releases).

## Configuration

### Environment Variables

| Variable | Description |
|----------|-------------|
| `COPILOT_API_KEY` | API key for authentication |
| `COPILOT_BASE_URL` | API base URL (default: http://localhost:8080) |
| `COPILOT_CONFIG_PATH` | Path to config file |

### Configuration File

Create `~/.copilot/config.yaml`:

```yaml
api_key: your-api-key
base_url: http://localhost:8080
default_model: claude-3-sonnet
output_format: json
```

### Initialize Configuration

```bash
copilot config init
```

This will interactively set up your configuration.

---

## Commands

### Global Options

| Option | Short | Description |
|--------|-------|-------------|
| `--api-key` | `-k` | API key for authentication |
| `--base-url` | `-u` | API base URL |
| `--output` | `-o` | Output format: json, yaml, table |
| `--quiet` | `-q` | Suppress non-essential output |
| `--verbose` | `-v` | Enable verbose logging |
| `--help` | `-h` | Show help |
| `--version` | | Show version |

---

## Chat Commands

### copilot chat

Start an interactive chat session.

```bash
copilot chat [options]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--conversation-id` | Resume an existing conversation |
| `--system-prompt` | Set a system prompt |
| `--model` | Model to use |
| `--temperature` | Sampling temperature (0-1) |

**Examples:**

```bash
# Start a new chat
copilot chat

# Resume a conversation
copilot chat --conversation-id conv-123

# With system prompt
copilot chat --system-prompt "You are a Python expert"
```

### copilot ask

Send a single message and get a response.

```bash
copilot ask <message> [options]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--conversation-id` | Use existing conversation context |
| `--stream` | Stream the response |
| `--file` | Include file contents as context |

**Examples:**

```bash
# Simple question
copilot ask "What is Python?"

# With file context
copilot ask "Review this code" --file ./main.py

# Stream response
copilot ask "Explain quantum computing in detail" --stream
```

---

## Conversation Commands

### copilot conversations list

List your conversations.

```bash
copilot conversations list [options]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--limit` | Maximum results (default: 20) |
| `--offset` | Skip results |

**Example:**

```bash
copilot conversations list --limit 10
```

### copilot conversations show

Show conversation details.

```bash
copilot conversations show <conversation-id> [options]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--messages` | Include messages |
| `--limit` | Message limit |

**Example:**

```bash
copilot conversations show conv-123 --messages
```

### copilot conversations create

Create a new conversation.

```bash
copilot conversations create [options]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--title` | Conversation title |
| `--system-prompt` | System prompt |

**Example:**

```bash
copilot conversations create --title "Code Review" --system-prompt "You are a code reviewer"
```

### copilot conversations delete

Delete a conversation.

```bash
copilot conversations delete <conversation-id>
```

---

## Workflow Commands

### copilot workflows list

List workflow definitions.

```bash
copilot workflows list
```

### copilot workflows show

Show workflow details.

```bash
copilot workflows show <workflow-id>
```

### copilot workflows create

Create a workflow from a YAML file.

```bash
copilot workflows create --file workflow.yaml
```

**Example workflow.yaml:**

```yaml
name: Code Analysis
description: Analyze and improve code
version: "1.0"
entry_point: analyze

steps:
  - id: analyze
    name: Analyze Code
    type: llm
    config:
      prompt: "Analyze the following code for issues: {{input.code}}"
    next_steps:
      - improve

  - id: improve
    name: Suggest Improvements
    type: llm
    config:
      prompt: "Based on the analysis, suggest improvements"
```

### copilot workflows run

Run a workflow.

```bash
copilot workflows run <workflow-id> [options]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--input` | Input key=value pair (can repeat) |
| `--input-file` | JSON file with inputs |
| `--wait` | Wait for completion |
| `--timeout` | Wait timeout in seconds |

**Examples:**

```bash
# Run with inputs
copilot workflows run wf-123 --input code="def add(a,b): return a+b"

# Run with input file
copilot workflows run wf-123 --input-file inputs.json --wait

# Wait for completion with timeout
copilot workflows run wf-123 --wait --timeout 300
```

### copilot workflows runs list

List workflow runs.

```bash
copilot workflows runs list [options]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--workflow-id` | Filter by workflow |
| `--status` | Filter by status |

### copilot workflows runs show

Show run details.

```bash
copilot workflows runs show <run-id>
```

### copilot workflows runs cancel

Cancel a running workflow.

```bash
copilot workflows runs cancel <run-id>
```

---

## Context Commands

### copilot context list

List context items.

```bash
copilot context list
```

### copilot context add

Add a context item.

```bash
copilot context add [options]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--name` | Item name |
| `--type` | Type: file, url, text, code, document |
| `--content` | Content string |
| `--file` | File to upload |
| `--url` | URL to fetch |

**Examples:**

```bash
# Add a file
copilot context add --name "main.py" --file ./main.py --type code

# Add text
copilot context add --name "guidelines" --content "Always use snake_case" --type text

# Add URL
copilot context add --name "docs" --url "https://example.com/docs" --type url
```

### copilot context show

Show context item details.

```bash
copilot context show <item-id>
```

### copilot context delete

Delete a context item.

```bash
copilot context delete <item-id>
```

---

## Auth Commands

### copilot auth login

Log in with username/email and password.

```bash
copilot auth login [options]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--username` | Username or email |
| `--password` | Password (prompt if not provided) |

**Example:**

```bash
copilot auth login --username user@example.com
```

### copilot auth logout

Log out and clear credentials.

```bash
copilot auth logout
```

### copilot auth status

Show current authentication status.

```bash
copilot auth status
```

### copilot auth token

Display the current access token.

```bash
copilot auth token
```

---

## Config Commands

### copilot config init

Initialize configuration interactively.

```bash
copilot config init
```

### copilot config show

Show current configuration.

```bash
copilot config show
```

### copilot config set

Set a configuration value.

```bash
copilot config set <key> <value>
```

**Examples:**

```bash
copilot config set base_url http://localhost:8080
copilot config set default_model claude-3-sonnet
```

### copilot config get

Get a configuration value.

```bash
copilot config get <key>
```

---

## Utility Commands

### copilot health

Check API health status.

```bash
copilot health
```

**Output:**

```
Status: healthy
Version: 1.0.0
Uptime: 3h 24m 15s

Components:
  database: healthy
  cache: healthy
  llm: healthy
```

### copilot version

Show CLI version.

```bash
copilot version
```

### copilot completion

Generate shell completion scripts.

```bash
# Bash
copilot completion bash > /etc/bash_completion.d/copilot

# Zsh
copilot completion zsh > ~/.zsh/completions/_copilot

# Fish
copilot completion fish > ~/.config/fish/completions/copilot.fish
```

---

## Examples

### Interactive Code Review

```bash
# Start a code review session
copilot chat --system-prompt "You are a senior code reviewer"

> Review this function for bugs:
> def divide(a, b): return a / b

The function has a potential division by zero bug...
```

### Batch Processing with Workflows

```bash
# Create a code analysis workflow
copilot workflows create --file analyze-workflow.yaml

# Run on multiple files
for file in src/*.py; do
  copilot workflows run wf-analyze --input code="$(cat $file)" --wait
done
```

### Piping Input

```bash
# Pipe file contents
cat main.py | copilot ask "Explain this code"

# Pipe git diff
git diff HEAD~1 | copilot ask "Summarize these changes"
```

### JSON Output for Scripting

```bash
# Get conversation as JSON
copilot conversations show conv-123 --output json | jq '.messages'

# Parse workflow results
copilot workflows runs show run-456 --output json | jq '.outputs'
```

---

## Troubleshooting

### Connection Issues

```bash
# Test connectivity
copilot health --verbose

# Check configuration
copilot config show
```

### Authentication Problems

```bash
# Re-authenticate
copilot auth logout
copilot auth login

# Verify token
copilot auth status
```

### Debug Mode

```bash
# Enable debug logging
COPILOT_DEBUG=true copilot chat
```
