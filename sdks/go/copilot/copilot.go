// Package copilot provides the Go SDK for the LLM CoPilot Agent API.
//
// Example usage:
//
//	import "github.com/llm-copilot-agent/sdk-go/copilot"
//
//	// Create a client with API key
//	client := copilot.NewClient("http://localhost:8080", copilot.WithAPIKey("your-api-key"))
//
//	// Create a conversation
//	conv, err := client.CreateConversation(ctx, nil)
//	if err != nil {
//	    log.Fatal(err)
//	}
//
//	// Send a message
//	msg, err := client.SendMessage(ctx, conv.ID, "Hello!")
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Println(msg.Content)
package copilot

import (
	"time"

	"github.com/llm-copilot-agent/sdk-go/copilot/client"
	"github.com/llm-copilot-agent/sdk-go/copilot/models"
	"github.com/llm-copilot-agent/sdk-go/copilot/streaming"
)

// Re-export client types
type (
	Client       = client.Client
	Config       = client.Config
	CoPilotError = client.CoPilotError
)

// Re-export model types
type (
	Message                  = models.Message
	MessageRole              = models.MessageRole
	MessageCreate            = models.MessageCreate
	Conversation             = models.Conversation
	ConversationCreate       = models.ConversationCreate
	WorkflowDefinition       = models.WorkflowDefinition
	WorkflowDefinitionCreate = models.WorkflowDefinitionCreate
	WorkflowRun              = models.WorkflowRun
	WorkflowRunCreate        = models.WorkflowRunCreate
	WorkflowStatus           = models.WorkflowStatus
	WorkflowStep             = models.WorkflowStep
	WorkflowStepType         = models.WorkflowStepType
	ContextItem              = models.ContextItem
	ContextItemCreate        = models.ContextItemCreate
	ContextType              = models.ContextType
	User                     = models.User
	LoginRequest             = models.LoginRequest
	LoginResponse            = models.LoginResponse
	TokenPair                = models.TokenPair
	ApiKey                   = models.ApiKey
	ApiKeyCreate             = models.ApiKeyCreate
	ApiKeyScope              = models.ApiKeyScope
	ApiKeyWithSecret         = models.ApiKeyWithSecret
	HealthStatus             = models.HealthStatus
	APIError                 = models.APIError
)

// Re-export streaming types
type (
	Stream         = streaming.Stream
	StreamEvent    = streaming.Event
	StreamDelta    = streaming.Delta
	StreamEventType = streaming.EventType
	StreamHandler  = streaming.Handler
)

// Re-export constants
const (
	// Message roles
	RoleUser      = models.RoleUser
	RoleAssistant = models.RoleAssistant
	RoleSystem    = models.RoleSystem

	// Workflow statuses
	WorkflowStatusPending   = models.WorkflowStatusPending
	WorkflowStatusRunning   = models.WorkflowStatusRunning
	WorkflowStatusCompleted = models.WorkflowStatusCompleted
	WorkflowStatusFailed    = models.WorkflowStatusFailed
	WorkflowStatusCancelled = models.WorkflowStatusCancelled

	// Step types
	StepTypeLLM         = models.StepTypeLLM
	StepTypeTool        = models.StepTypeTool
	StepTypeCondition   = models.StepTypeCondition
	StepTypeParallel    = models.StepTypeParallel
	StepTypeLoop        = models.StepTypeLoop
	StepTypeHumanReview = models.StepTypeHumanReview

	// Context types
	ContextTypeFile     = models.ContextTypeFile
	ContextTypeURL      = models.ContextTypeURL
	ContextTypeText     = models.ContextTypeText
	ContextTypeCode     = models.ContextTypeCode
	ContextTypeDocument = models.ContextTypeDocument

	// API key scopes
	ScopeRead      = models.ScopeRead
	ScopeWrite     = models.ScopeWrite
	ScopeChat      = models.ScopeChat
	ScopeWorkflows = models.ScopeWorkflows
	ScopeContext   = models.ScopeContext
	ScopeSandbox   = models.ScopeSandbox
	ScopeAdmin     = models.ScopeAdmin

	// Stream event types
	EventMessageStart = streaming.EventMessageStart
	EventContentDelta = streaming.EventContentDelta
	EventMessageEnd   = streaming.EventMessageEnd
	EventToolUse      = streaming.EventToolUse
	EventToolResult   = streaming.EventToolResult
	EventError        = streaming.EventError
	EventPing         = streaming.EventPing
)

// Option configures the client.
type Option func(*client.Config)

// WithAPIKey sets the API key for authentication.
func WithAPIKey(apiKey string) Option {
	return func(c *client.Config) {
		c.APIKey = apiKey
	}
}

// WithAccessToken sets the access token for authentication.
func WithAccessToken(token string) Option {
	return func(c *client.Config) {
		c.AccessToken = token
	}
}

// WithTimeout sets the request timeout.
func WithTimeout(timeout time.Duration) Option {
	return func(c *client.Config) {
		c.Timeout = timeout
	}
}

// WithMaxRetries sets the maximum number of retries.
func WithMaxRetries(retries int) Option {
	return func(c *client.Config) {
		c.MaxRetries = retries
	}
}

// NewClient creates a new CoPilot client with options.
func NewClient(baseURL string, opts ...Option) *Client {
	config := client.DefaultConfig()
	config.BaseURL = baseURL

	for _, opt := range opts {
		opt(config)
	}

	return client.New(config)
}

// NewClientWithConfig creates a new client with full configuration.
func NewClientWithConfig(config *Config) *Client {
	return client.New(config)
}

// DefaultConfig returns a default client configuration.
func DefaultConfig() *Config {
	return client.DefaultConfig()
}
