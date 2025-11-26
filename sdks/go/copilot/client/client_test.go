package client

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/llm-copilot-agent/sdk-go/copilot/models"
)

func TestNewClient(t *testing.T) {
	t.Run("default config", func(t *testing.T) {
		client := New(nil)
		if client == nil {
			t.Fatal("expected non-nil client")
		}
		if client.config.BaseURL != "http://localhost:8080" {
			t.Errorf("expected default base URL, got %s", client.config.BaseURL)
		}
	})

	t.Run("custom config", func(t *testing.T) {
		config := &Config{
			BaseURL: "https://api.example.com",
			APIKey:  "test-key",
			Timeout: 60 * time.Second,
		}
		client := New(config)
		if client.config.BaseURL != "https://api.example.com" {
			t.Errorf("expected custom base URL, got %s", client.config.BaseURL)
		}
		if client.config.APIKey != "test-key" {
			t.Errorf("expected API key, got %s", client.config.APIKey)
		}
	})
}

func TestNewWithAPIKey(t *testing.T) {
	client := NewWithAPIKey("https://api.example.com", "my-api-key")
	if client.config.APIKey != "my-api-key" {
		t.Errorf("expected API key 'my-api-key', got %s", client.config.APIKey)
	}
}

func TestNewWithToken(t *testing.T) {
	client := NewWithToken("https://api.example.com", "my-token")
	if client.config.AccessToken != "my-token" {
		t.Errorf("expected access token 'my-token', got %s", client.config.AccessToken)
	}
}

func TestSetAccessToken(t *testing.T) {
	client := New(nil)
	client.SetAccessToken("new-token")
	if client.config.AccessToken != "new-token" {
		t.Errorf("expected access token 'new-token', got %s", client.config.AccessToken)
	}
}

func TestHealthCheck(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/health" {
			t.Errorf("expected path /health, got %s", r.URL.Path)
		}
		if r.Method != http.MethodGet {
			t.Errorf("expected GET, got %s", r.Method)
		}

		response := models.HealthStatus{
			Status:        "healthy",
			Version:       "1.0.0",
			UptimeSeconds: 3600,
			Components: map[string]string{
				"database": "healthy",
				"cache":    "healthy",
			},
		}
		json.NewEncoder(w).Encode(response)
	}))
	defer server.Close()

	client := NewWithAPIKey(server.URL, "test-key")
	ctx := context.Background()

	status, err := client.HealthCheck(ctx)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if status.Status != "healthy" {
		t.Errorf("expected status 'healthy', got %s", status.Status)
	}
	if status.Version != "1.0.0" {
		t.Errorf("expected version '1.0.0', got %s", status.Version)
	}
}

func TestCreateConversation(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/v1/conversations" {
			t.Errorf("expected path /api/v1/conversations, got %s", r.URL.Path)
		}
		if r.Method != http.MethodPost {
			t.Errorf("expected POST, got %s", r.Method)
		}

		// Verify API key header
		if r.Header.Get("X-API-Key") != "test-key" {
			t.Errorf("expected API key header, got %s", r.Header.Get("X-API-Key"))
		}

		response := models.Conversation{
			ID:           "conv-123",
			UserID:       "user-456",
			MessageCount: 0,
			CreatedAt:    time.Now(),
			UpdatedAt:    time.Now(),
		}
		json.NewEncoder(w).Encode(response)
	}))
	defer server.Close()

	client := NewWithAPIKey(server.URL, "test-key")
	ctx := context.Background()

	conv, err := client.CreateConversation(ctx, nil)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if conv.ID != "conv-123" {
		t.Errorf("expected ID 'conv-123', got %s", conv.ID)
	}
}

func TestSendMessage(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		expectedPath := "/api/v1/conversations/conv-123/messages"
		if r.URL.Path != expectedPath {
			t.Errorf("expected path %s, got %s", expectedPath, r.URL.Path)
		}

		// Verify request body
		var req models.MessageCreate
		json.NewDecoder(r.Body).Decode(&req)
		if req.Content != "Hello!" {
			t.Errorf("expected content 'Hello!', got %s", req.Content)
		}

		response := models.Message{
			ID:             "msg-789",
			ConversationID: "conv-123",
			Role:           models.RoleAssistant,
			Content:        "Hello! How can I help you?",
			CreatedAt:      time.Now(),
		}
		json.NewEncoder(w).Encode(response)
	}))
	defer server.Close()

	client := NewWithAPIKey(server.URL, "test-key")
	ctx := context.Background()

	msg, err := client.SendMessage(ctx, "conv-123", "Hello!")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if msg.ID != "msg-789" {
		t.Errorf("expected ID 'msg-789', got %s", msg.ID)
	}
	if msg.Role != models.RoleAssistant {
		t.Errorf("expected role 'assistant', got %s", msg.Role)
	}
}

func TestLogin(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/v1/auth/login" {
			t.Errorf("expected path /api/v1/auth/login, got %s", r.URL.Path)
		}

		var req models.LoginRequest
		json.NewDecoder(r.Body).Decode(&req)
		if req.UsernameOrEmail != "testuser" {
			t.Errorf("expected username 'testuser', got %s", req.UsernameOrEmail)
		}

		response := models.LoginResponse{
			AccessToken:      "access-token-123",
			RefreshToken:     "refresh-token-456",
			TokenType:        "Bearer",
			ExpiresIn:        3600,
			RefreshExpiresIn: 86400,
			User: models.User{
				ID:       "user-123",
				Username: "testuser",
				Email:    "test@example.com",
				Roles:    []string{"user"},
			},
		}
		json.NewEncoder(w).Encode(response)
	}))
	defer server.Close()

	client := New(&Config{BaseURL: server.URL})
	ctx := context.Background()

	resp, err := client.Login(ctx, "testuser", "password123")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if resp.AccessToken != "access-token-123" {
		t.Errorf("expected access token 'access-token-123', got %s", resp.AccessToken)
	}
	if client.config.AccessToken != "access-token-123" {
		t.Errorf("expected client access token to be set")
	}
}

func TestErrorHandling(t *testing.T) {
	t.Run("401 unauthorized", func(t *testing.T) {
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.WriteHeader(http.StatusUnauthorized)
			json.NewEncoder(w).Encode(models.APIError{
				Code:    "UNAUTHORIZED",
				Message: "Invalid credentials",
			})
		}))
		defer server.Close()

		client := NewWithAPIKey(server.URL, "invalid-key")
		ctx := context.Background()

		_, err := client.HealthCheck(ctx)
		if err == nil {
			t.Fatal("expected error, got nil")
		}

		copilotErr, ok := err.(*CoPilotError)
		if !ok {
			t.Fatalf("expected CoPilotError, got %T", err)
		}
		if !copilotErr.IsUnauthorized() {
			t.Errorf("expected unauthorized error")
		}
	})

	t.Run("404 not found", func(t *testing.T) {
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.WriteHeader(http.StatusNotFound)
			json.NewEncoder(w).Encode(models.APIError{
				Code:    "NOT_FOUND",
				Message: "Resource not found",
			})
		}))
		defer server.Close()

		client := NewWithAPIKey(server.URL, "test-key")
		ctx := context.Background()

		_, err := client.GetConversation(ctx, "nonexistent")
		if err == nil {
			t.Fatal("expected error, got nil")
		}

		copilotErr, ok := err.(*CoPilotError)
		if !ok {
			t.Fatalf("expected CoPilotError, got %T", err)
		}
		if !copilotErr.IsNotFound() {
			t.Errorf("expected not found error")
		}
	})

	t.Run("500 server error", func(t *testing.T) {
		server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(models.APIError{
				Code:    "SERVER_ERROR",
				Message: "Internal server error",
			})
		}))
		defer server.Close()

		config := &Config{
			BaseURL:      server.URL,
			APIKey:       "test-key",
			MaxRetries:   -1, // Disable retries completely
			Timeout:      5 * time.Second,
			RetryWaitMin: 1 * time.Second,
			RetryWaitMax: 30 * time.Second,
		}
		client := New(config)
		ctx := context.Background()

		_, err := client.HealthCheck(ctx)
		if err == nil {
			t.Fatal("expected error, got nil")
		}

		// With retries disabled, we get the raw CoPilotError
		copilotErr, ok := err.(*CoPilotError)
		if !ok {
			t.Fatalf("expected CoPilotError, got %T: %v", err, err)
		}
		if !copilotErr.IsServerError() {
			t.Errorf("expected server error")
		}
	})
}

func TestCalculateBackoff(t *testing.T) {
	client := New(&Config{
		RetryWaitMin: 1 * time.Second,
		RetryWaitMax: 30 * time.Second,
	})

	tests := []struct {
		attempt  int
		expected time.Duration
	}{
		{1, 1 * time.Second},
		{2, 2 * time.Second},
		{3, 4 * time.Second},
		{4, 8 * time.Second},
		{5, 16 * time.Second},
		{6, 30 * time.Second}, // Capped at max
	}

	for _, tt := range tests {
		delay := client.calculateBackoff(tt.attempt)
		if delay != tt.expected {
			t.Errorf("attempt %d: expected %v, got %v", tt.attempt, tt.expected, delay)
		}
	}
}

func TestIsRetryable(t *testing.T) {
	client := New(nil)

	tests := []struct {
		name       string
		err        error
		retryable  bool
	}{
		{
			name:      "server error",
			err:       &CoPilotError{StatusCode: 500},
			retryable: true,
		},
		{
			name:      "rate limited",
			err:       &CoPilotError{StatusCode: 429},
			retryable: true,
		},
		{
			name:      "bad request",
			err:       &CoPilotError{StatusCode: 400},
			retryable: false,
		},
		{
			name:      "unauthorized",
			err:       &CoPilotError{StatusCode: 401},
			retryable: false,
		},
		{
			name:      "not found",
			err:       &CoPilotError{StatusCode: 404},
			retryable: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			retryable := client.isRetryable(tt.err)
			if retryable != tt.retryable {
				t.Errorf("expected retryable=%v, got %v", tt.retryable, retryable)
			}
		})
	}
}
