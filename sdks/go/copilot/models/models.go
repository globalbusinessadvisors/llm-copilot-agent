// Package models provides data types for the LLM CoPilot SDK.
package models

import (
	"time"
)

// MessageRole represents the role of a message sender.
type MessageRole string

const (
	RoleUser      MessageRole = "user"
	RoleAssistant MessageRole = "assistant"
	RoleSystem    MessageRole = "system"
)

// Message represents a single message in a conversation.
type Message struct {
	ID             string                 `json:"id"`
	ConversationID string                 `json:"conversation_id"`
	Role           MessageRole            `json:"role"`
	Content        string                 `json:"content"`
	Metadata       map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt      time.Time              `json:"created_at"`
}

// MessageCreate represents a request to create a new message.
type MessageCreate struct {
	Role     MessageRole            `json:"role,omitempty"`
	Content  string                 `json:"content"`
	Metadata map[string]interface{} `json:"metadata,omitempty"`
}

// Conversation represents a conversation session.
type Conversation struct {
	ID           string                 `json:"id"`
	Title        string                 `json:"title,omitempty"`
	UserID       string                 `json:"user_id"`
	TenantID     string                 `json:"tenant_id,omitempty"`
	Metadata     map[string]interface{} `json:"metadata,omitempty"`
	MessageCount int                    `json:"message_count"`
	CreatedAt    time.Time              `json:"created_at"`
	UpdatedAt    time.Time              `json:"updated_at"`
}

// ConversationCreate represents a request to create a new conversation.
type ConversationCreate struct {
	Title        string                 `json:"title,omitempty"`
	Metadata     map[string]interface{} `json:"metadata,omitempty"`
	SystemPrompt string                 `json:"system_prompt,omitempty"`
}

// WorkflowStatus represents the status of a workflow run.
type WorkflowStatus string

const (
	WorkflowStatusPending   WorkflowStatus = "pending"
	WorkflowStatusRunning   WorkflowStatus = "running"
	WorkflowStatusCompleted WorkflowStatus = "completed"
	WorkflowStatusFailed    WorkflowStatus = "failed"
	WorkflowStatusCancelled WorkflowStatus = "cancelled"
)

// WorkflowStepType represents the type of a workflow step.
type WorkflowStepType string

const (
	StepTypeLLM         WorkflowStepType = "llm"
	StepTypeTool        WorkflowStepType = "tool"
	StepTypeCondition   WorkflowStepType = "condition"
	StepTypeParallel    WorkflowStepType = "parallel"
	StepTypeLoop        WorkflowStepType = "loop"
	StepTypeHumanReview WorkflowStepType = "human_review"
)

// WorkflowStep represents a step in a workflow definition.
type WorkflowStep struct {
	ID        string                 `json:"id"`
	Name      string                 `json:"name"`
	Type      WorkflowStepType       `json:"type"`
	Config    map[string]interface{} `json:"config,omitempty"`
	NextSteps []string               `json:"next_steps,omitempty"`
	OnError   string                 `json:"on_error,omitempty"`
}

// WorkflowDefinition represents a workflow definition.
type WorkflowDefinition struct {
	ID          string                 `json:"id"`
	Name        string                 `json:"name"`
	Description string                 `json:"description,omitempty"`
	Version     string                 `json:"version"`
	Steps       []WorkflowStep         `json:"steps"`
	EntryPoint  string                 `json:"entry_point"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
	CreatedAt   time.Time              `json:"created_at"`
	UpdatedAt   time.Time              `json:"updated_at"`
}

// WorkflowDefinitionCreate represents a request to create a workflow.
type WorkflowDefinitionCreate struct {
	Name        string                 `json:"name"`
	Description string                 `json:"description,omitempty"`
	Version     string                 `json:"version,omitempty"`
	Steps       []WorkflowStep         `json:"steps"`
	EntryPoint  string                 `json:"entry_point"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
}

// WorkflowRun represents a workflow run instance.
type WorkflowRun struct {
	ID          string                 `json:"id"`
	WorkflowID  string                 `json:"workflow_id"`
	Status      WorkflowStatus         `json:"status"`
	InputData   map[string]interface{} `json:"input_data,omitempty"`
	OutputData  map[string]interface{} `json:"output_data,omitempty"`
	Error       string                 `json:"error,omitempty"`
	CurrentStep string                 `json:"current_step,omitempty"`
	StartedAt   time.Time              `json:"started_at"`
	CompletedAt *time.Time             `json:"completed_at,omitempty"`
}

// WorkflowRunCreate represents a request to start a workflow run.
type WorkflowRunCreate struct {
	WorkflowID string                 `json:"workflow_id"`
	InputData  map[string]interface{} `json:"input_data,omitempty"`
}

// ContextType represents the type of a context item.
type ContextType string

const (
	ContextTypeFile     ContextType = "file"
	ContextTypeURL      ContextType = "url"
	ContextTypeText     ContextType = "text"
	ContextTypeCode     ContextType = "code"
	ContextTypeDocument ContextType = "document"
)

// ContextItem represents a context item for conversation or workflow.
type ContextItem struct {
	ID          string                 `json:"id"`
	Type        ContextType            `json:"type"`
	Name        string                 `json:"name"`
	Content     string                 `json:"content,omitempty"`
	URL         string                 `json:"url,omitempty"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
	EmbeddingID string                 `json:"embedding_id,omitempty"`
	CreatedAt   time.Time              `json:"created_at"`
}

// ContextItemCreate represents a request to create a context item.
type ContextItemCreate struct {
	Type     ContextType            `json:"type"`
	Name     string                 `json:"name"`
	Content  string                 `json:"content,omitempty"`
	URL      string                 `json:"url,omitempty"`
	Metadata map[string]interface{} `json:"metadata,omitempty"`
}

// User represents a user.
type User struct {
	ID            string    `json:"id"`
	Username      string    `json:"username"`
	Email         string    `json:"email"`
	Roles         []string  `json:"roles"`
	TenantID      string    `json:"tenant_id,omitempty"`
	IsActive      bool      `json:"is_active"`
	EmailVerified bool      `json:"email_verified"`
	CreatedAt     time.Time `json:"created_at"`
	LastLoginAt   time.Time `json:"last_login_at,omitempty"`
}

// LoginRequest represents a login request.
type LoginRequest struct {
	UsernameOrEmail string `json:"username_or_email"`
	Password        string `json:"password"`
}

// LoginResponse represents a login response.
type LoginResponse struct {
	AccessToken      string `json:"access_token"`
	RefreshToken     string `json:"refresh_token"`
	TokenType        string `json:"token_type"`
	ExpiresIn        int    `json:"expires_in"`
	RefreshExpiresIn int    `json:"refresh_expires_in"`
	User             User   `json:"user"`
}

// TokenPair represents an access/refresh token pair.
type TokenPair struct {
	AccessToken      string `json:"access_token"`
	RefreshToken     string `json:"refresh_token"`
	TokenType        string `json:"token_type"`
	ExpiresIn        int    `json:"expires_in"`
	RefreshExpiresIn int    `json:"refresh_expires_in"`
}

// ApiKeyScope represents an API key scope.
type ApiKeyScope string

const (
	ScopeRead      ApiKeyScope = "read"
	ScopeWrite     ApiKeyScope = "write"
	ScopeChat      ApiKeyScope = "chat"
	ScopeWorkflows ApiKeyScope = "workflows"
	ScopeContext   ApiKeyScope = "context"
	ScopeSandbox   ApiKeyScope = "sandbox"
	ScopeAdmin     ApiKeyScope = "admin"
)

// ApiKeyCreate represents a request to create an API key.
type ApiKeyCreate struct {
	Name          string        `json:"name"`
	Scopes        []ApiKeyScope `json:"scopes,omitempty"`
	ExpiresInDays int           `json:"expires_in_days,omitempty"`
}

// ApiKey represents API key information.
type ApiKey struct {
	ID           string        `json:"id"`
	Name         string        `json:"name"`
	Prefix       string        `json:"prefix"`
	Scopes       []ApiKeyScope `json:"scopes"`
	CreatedAt    time.Time     `json:"created_at"`
	ExpiresAt    *time.Time    `json:"expires_at,omitempty"`
	LastUsedAt   *time.Time    `json:"last_used_at,omitempty"`
	IsActive     bool          `json:"is_active"`
	RequestCount int64         `json:"request_count"`
}

// ApiKeyWithSecret represents an API key with the secret (only returned on creation).
type ApiKeyWithSecret struct {
	ApiKey
	Key string `json:"key"`
}

// HealthStatus represents health status response.
type HealthStatus struct {
	Status        string            `json:"status"`
	Version       string            `json:"version"`
	UptimeSeconds float64           `json:"uptime_seconds"`
	Components    map[string]string `json:"components,omitempty"`
}

// PaginatedResponse represents a paginated API response.
type PaginatedResponse[T any] struct {
	Items      []T    `json:"items"`
	Total      int    `json:"total"`
	Page       int    `json:"page"`
	PageSize   int    `json:"page_size"`
	HasMore    bool   `json:"has_more"`
	NextCursor string `json:"next_cursor,omitempty"`
}

// APIError represents an API error response.
type APIError struct {
	Code      string                 `json:"code"`
	Message   string                 `json:"message"`
	Details   map[string]interface{} `json:"details,omitempty"`
	RequestID string                 `json:"request_id,omitempty"`
}

// Error implements the error interface.
func (e *APIError) Error() string {
	return e.Message
}
