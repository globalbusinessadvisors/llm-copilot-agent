// Package client provides the HTTP client for the LLM CoPilot API.
package client

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"time"

	"github.com/llm-copilot-agent/sdk-go/copilot/models"
)

// Config holds the client configuration.
type Config struct {
	// BaseURL is the API base URL.
	BaseURL string
	// APIKey for authentication.
	APIKey string
	// AccessToken for JWT authentication.
	AccessToken string
	// Timeout for HTTP requests.
	Timeout time.Duration
	// HTTPClient allows using a custom HTTP client.
	HTTPClient *http.Client
	// MaxRetries is the maximum number of retries for failed requests.
	MaxRetries int
	// RetryWaitMin is the minimum wait time between retries.
	RetryWaitMin time.Duration
	// RetryWaitMax is the maximum wait time between retries.
	RetryWaitMax time.Duration
}

// DefaultConfig returns a default configuration.
func DefaultConfig() *Config {
	return &Config{
		BaseURL:      "http://localhost:8080",
		Timeout:      30 * time.Second,
		MaxRetries:   3,
		RetryWaitMin: 1 * time.Second,
		RetryWaitMax: 30 * time.Second,
	}
}

// Client is the CoPilot API client.
type Client struct {
	config     *Config
	httpClient *http.Client
}

// New creates a new CoPilot client with the given configuration.
func New(config *Config) *Client {
	if config == nil {
		config = DefaultConfig()
	}

	httpClient := config.HTTPClient
	if httpClient == nil {
		httpClient = &http.Client{
			Timeout: config.Timeout,
		}
	}

	return &Client{
		config:     config,
		httpClient: httpClient,
	}
}

// NewWithAPIKey creates a new client with API key authentication.
func NewWithAPIKey(baseURL, apiKey string) *Client {
	config := DefaultConfig()
	config.BaseURL = baseURL
	config.APIKey = apiKey
	return New(config)
}

// NewWithToken creates a new client with token authentication.
func NewWithToken(baseURL, accessToken string) *Client {
	config := DefaultConfig()
	config.BaseURL = baseURL
	config.AccessToken = accessToken
	return New(config)
}

// SetAccessToken updates the access token.
func (c *Client) SetAccessToken(token string) {
	c.config.AccessToken = token
}

// request makes an HTTP request with retry logic.
func (c *Client) request(ctx context.Context, method, path string, body interface{}, result interface{}) error {
	// If retries are disabled (MaxRetries < 0), just make a single request
	if c.config.MaxRetries < 0 {
		return c.doRequest(ctx, method, path, body, result)
	}

	var lastErr error

	for attempt := 0; attempt <= c.config.MaxRetries; attempt++ {
		if attempt > 0 {
			// Calculate backoff delay
			delay := c.calculateBackoff(attempt)
			select {
			case <-ctx.Done():
				return ctx.Err()
			case <-time.After(delay):
			}
		}

		err := c.doRequest(ctx, method, path, body, result)
		if err == nil {
			return nil
		}

		lastErr = err

		// Check if error is retryable
		if !c.isRetryable(err) {
			return err
		}
	}

	return fmt.Errorf("max retries exceeded: %w", lastErr)
}

// doRequest performs a single HTTP request.
func (c *Client) doRequest(ctx context.Context, method, path string, body interface{}, result interface{}) error {
	fullURL := c.config.BaseURL + path

	var bodyReader io.Reader
	if body != nil {
		jsonBody, err := json.Marshal(body)
		if err != nil {
			return fmt.Errorf("failed to marshal request body: %w", err)
		}
		bodyReader = bytes.NewReader(jsonBody)
	}

	req, err := http.NewRequestWithContext(ctx, method, fullURL, bodyReader)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	// Set headers
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")

	if c.config.APIKey != "" {
		req.Header.Set("X-API-Key", c.config.APIKey)
	} else if c.config.AccessToken != "" {
		req.Header.Set("Authorization", "Bearer "+c.config.AccessToken)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	// Read response body
	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read response body: %w", err)
	}

	// Handle error responses
	if resp.StatusCode >= 400 {
		var apiErr models.APIError
		if err := json.Unmarshal(respBody, &apiErr); err != nil {
			return &CoPilotError{
				StatusCode: resp.StatusCode,
				Message:    string(respBody),
			}
		}
		return &CoPilotError{
			StatusCode: resp.StatusCode,
			Code:       apiErr.Code,
			Message:    apiErr.Message,
			Details:    apiErr.Details,
			RequestID:  apiErr.RequestID,
		}
	}

	// Parse successful response
	if result != nil && len(respBody) > 0 {
		if err := json.Unmarshal(respBody, result); err != nil {
			return fmt.Errorf("failed to parse response: %w", err)
		}
	}

	return nil
}

// calculateBackoff calculates the backoff delay for the given attempt.
func (c *Client) calculateBackoff(attempt int) time.Duration {
	delay := c.config.RetryWaitMin * time.Duration(1<<uint(attempt-1))
	if delay > c.config.RetryWaitMax {
		delay = c.config.RetryWaitMax
	}
	return delay
}

// isRetryable checks if an error should be retried.
func (c *Client) isRetryable(err error) bool {
	if copilotErr, ok := err.(*CoPilotError); ok {
		// Retry on server errors and rate limits
		return copilotErr.StatusCode >= 500 || copilotErr.StatusCode == 429
	}
	return false
}

// get performs a GET request.
func (c *Client) get(ctx context.Context, path string, result interface{}) error {
	return c.request(ctx, http.MethodGet, path, nil, result)
}

// post performs a POST request.
func (c *Client) post(ctx context.Context, path string, body interface{}, result interface{}) error {
	return c.request(ctx, http.MethodPost, path, body, result)
}

// delete performs a DELETE request.
func (c *Client) delete(ctx context.Context, path string) error {
	return c.request(ctx, http.MethodDelete, path, nil, nil)
}

// CoPilotError represents an API error.
type CoPilotError struct {
	StatusCode int
	Code       string
	Message    string
	Details    map[string]interface{}
	RequestID  string
}

// Error implements the error interface.
func (e *CoPilotError) Error() string {
	if e.Code != "" {
		return fmt.Sprintf("[%d] %s: %s", e.StatusCode, e.Code, e.Message)
	}
	return fmt.Sprintf("[%d] %s", e.StatusCode, e.Message)
}

// IsNotFound returns true if the error is a 404.
func (e *CoPilotError) IsNotFound() bool {
	return e.StatusCode == 404
}

// IsUnauthorized returns true if the error is a 401.
func (e *CoPilotError) IsUnauthorized() bool {
	return e.StatusCode == 401
}

// IsForbidden returns true if the error is a 403.
func (e *CoPilotError) IsForbidden() bool {
	return e.StatusCode == 403
}

// IsRateLimited returns true if the error is a 429.
func (e *CoPilotError) IsRateLimited() bool {
	return e.StatusCode == 429
}

// IsServerError returns true if the error is a 5xx.
func (e *CoPilotError) IsServerError() bool {
	return e.StatusCode >= 500
}

// ================================
// Authentication Methods
// ================================

// Login authenticates with username/email and password.
func (c *Client) Login(ctx context.Context, usernameOrEmail, password string) (*models.LoginResponse, error) {
	req := models.LoginRequest{
		UsernameOrEmail: usernameOrEmail,
		Password:        password,
	}

	var resp models.LoginResponse
	if err := c.post(ctx, "/api/v1/auth/login", req, &resp); err != nil {
		return nil, err
	}

	// Store the access token for subsequent requests
	c.config.AccessToken = resp.AccessToken

	return &resp, nil
}

// RefreshTokens refreshes the access tokens.
func (c *Client) RefreshTokens(ctx context.Context, refreshToken string) (*models.TokenPair, error) {
	req := map[string]string{"refresh_token": refreshToken}

	var resp models.TokenPair
	if err := c.post(ctx, "/api/v1/auth/refresh", req, &resp); err != nil {
		return nil, err
	}

	c.config.AccessToken = resp.AccessToken
	return &resp, nil
}

// Logout logs out the user.
func (c *Client) Logout(ctx context.Context) error {
	if err := c.post(ctx, "/api/v1/auth/logout", nil, nil); err != nil {
		return err
	}
	c.config.AccessToken = ""
	return nil
}

// GetCurrentUser returns the current authenticated user.
func (c *Client) GetCurrentUser(ctx context.Context) (*models.User, error) {
	var user models.User
	if err := c.get(ctx, "/api/v1/auth/me", &user); err != nil {
		return nil, err
	}
	return &user, nil
}

// ================================
// Conversation Methods
// ================================

// CreateConversation creates a new conversation.
func (c *Client) CreateConversation(ctx context.Context, req *models.ConversationCreate) (*models.Conversation, error) {
	if req == nil {
		req = &models.ConversationCreate{}
	}

	var conv models.Conversation
	if err := c.post(ctx, "/api/v1/conversations", req, &conv); err != nil {
		return nil, err
	}
	return &conv, nil
}

// GetConversation retrieves a conversation by ID.
func (c *Client) GetConversation(ctx context.Context, id string) (*models.Conversation, error) {
	var conv models.Conversation
	if err := c.get(ctx, "/api/v1/conversations/"+id, &conv); err != nil {
		return nil, err
	}
	return &conv, nil
}

// ListConversations lists conversations with pagination.
func (c *Client) ListConversations(ctx context.Context, limit, offset int) ([]models.Conversation, error) {
	path := fmt.Sprintf("/api/v1/conversations?limit=%d&offset=%d", limit, offset)

	var resp struct {
		Items []models.Conversation `json:"items"`
	}
	if err := c.get(ctx, path, &resp); err != nil {
		return nil, err
	}
	return resp.Items, nil
}

// DeleteConversation deletes a conversation.
func (c *Client) DeleteConversation(ctx context.Context, id string) error {
	return c.delete(ctx, "/api/v1/conversations/"+id)
}

// SendMessage sends a message in a conversation.
func (c *Client) SendMessage(ctx context.Context, conversationID, content string) (*models.Message, error) {
	req := models.MessageCreate{
		Role:    models.RoleUser,
		Content: content,
	}

	var msg models.Message
	path := fmt.Sprintf("/api/v1/conversations/%s/messages", conversationID)
	if err := c.post(ctx, path, req, &msg); err != nil {
		return nil, err
	}
	return &msg, nil
}

// ListMessages lists messages in a conversation.
func (c *Client) ListMessages(ctx context.Context, conversationID string, limit, offset int) ([]models.Message, error) {
	path := fmt.Sprintf("/api/v1/conversations/%s/messages?limit=%d&offset=%d", conversationID, limit, offset)

	var resp struct {
		Items []models.Message `json:"items"`
	}
	if err := c.get(ctx, path, &resp); err != nil {
		return nil, err
	}
	return resp.Items, nil
}

// ================================
// Workflow Methods
// ================================

// CreateWorkflow creates a new workflow definition.
func (c *Client) CreateWorkflow(ctx context.Context, req *models.WorkflowDefinitionCreate) (*models.WorkflowDefinition, error) {
	var wf models.WorkflowDefinition
	if err := c.post(ctx, "/api/v1/workflows", req, &wf); err != nil {
		return nil, err
	}
	return &wf, nil
}

// GetWorkflow retrieves a workflow definition.
func (c *Client) GetWorkflow(ctx context.Context, id string) (*models.WorkflowDefinition, error) {
	var wf models.WorkflowDefinition
	if err := c.get(ctx, "/api/v1/workflows/"+id, &wf); err != nil {
		return nil, err
	}
	return &wf, nil
}

// ListWorkflows lists workflow definitions.
func (c *Client) ListWorkflows(ctx context.Context) ([]models.WorkflowDefinition, error) {
	var resp struct {
		Items []models.WorkflowDefinition `json:"items"`
	}
	if err := c.get(ctx, "/api/v1/workflows", &resp); err != nil {
		return nil, err
	}
	return resp.Items, nil
}

// DeleteWorkflow deletes a workflow definition.
func (c *Client) DeleteWorkflow(ctx context.Context, id string) error {
	return c.delete(ctx, "/api/v1/workflows/"+id)
}

// RunWorkflow starts a workflow run.
func (c *Client) RunWorkflow(ctx context.Context, req *models.WorkflowRunCreate) (*models.WorkflowRun, error) {
	var run models.WorkflowRun
	if err := c.post(ctx, "/api/v1/workflows/runs", req, &run); err != nil {
		return nil, err
	}
	return &run, nil
}

// GetWorkflowRun retrieves a workflow run.
func (c *Client) GetWorkflowRun(ctx context.Context, id string) (*models.WorkflowRun, error) {
	var run models.WorkflowRun
	if err := c.get(ctx, "/api/v1/workflows/runs/"+id, &run); err != nil {
		return nil, err
	}
	return &run, nil
}

// ListWorkflowRuns lists workflow runs.
func (c *Client) ListWorkflowRuns(ctx context.Context, workflowID string) ([]models.WorkflowRun, error) {
	path := "/api/v1/workflows/runs"
	if workflowID != "" {
		path += "?workflow_id=" + url.QueryEscape(workflowID)
	}

	var resp struct {
		Items []models.WorkflowRun `json:"items"`
	}
	if err := c.get(ctx, path, &resp); err != nil {
		return nil, err
	}
	return resp.Items, nil
}

// CancelWorkflowRun cancels a workflow run.
func (c *Client) CancelWorkflowRun(ctx context.Context, id string) (*models.WorkflowRun, error) {
	var run models.WorkflowRun
	if err := c.post(ctx, "/api/v1/workflows/runs/"+id+"/cancel", nil, &run); err != nil {
		return nil, err
	}
	return &run, nil
}

// ================================
// Context Methods
// ================================

// CreateContextItem creates a context item.
func (c *Client) CreateContextItem(ctx context.Context, req *models.ContextItemCreate) (*models.ContextItem, error) {
	var item models.ContextItem
	if err := c.post(ctx, "/api/v1/context", req, &item); err != nil {
		return nil, err
	}
	return &item, nil
}

// GetContextItem retrieves a context item.
func (c *Client) GetContextItem(ctx context.Context, id string) (*models.ContextItem, error) {
	var item models.ContextItem
	if err := c.get(ctx, "/api/v1/context/"+id, &item); err != nil {
		return nil, err
	}
	return &item, nil
}

// ListContextItems lists context items.
func (c *Client) ListContextItems(ctx context.Context) ([]models.ContextItem, error) {
	var resp struct {
		Items []models.ContextItem `json:"items"`
	}
	if err := c.get(ctx, "/api/v1/context", &resp); err != nil {
		return nil, err
	}
	return resp.Items, nil
}

// DeleteContextItem deletes a context item.
func (c *Client) DeleteContextItem(ctx context.Context, id string) error {
	return c.delete(ctx, "/api/v1/context/"+id)
}

// ================================
// Health Methods
// ================================

// HealthCheck performs a health check.
func (c *Client) HealthCheck(ctx context.Context) (*models.HealthStatus, error) {
	var status models.HealthStatus
	if err := c.get(ctx, "/health", &status); err != nil {
		return nil, err
	}
	return &status, nil
}
