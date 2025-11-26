package models

import (
	"encoding/json"
	"testing"
	"time"
)

func TestMessageRole(t *testing.T) {
	tests := []struct {
		role     MessageRole
		expected string
	}{
		{RoleUser, "user"},
		{RoleAssistant, "assistant"},
		{RoleSystem, "system"},
	}

	for _, tt := range tests {
		if string(tt.role) != tt.expected {
			t.Errorf("expected %s, got %s", tt.expected, tt.role)
		}
	}
}

func TestWorkflowStatus(t *testing.T) {
	tests := []struct {
		status   WorkflowStatus
		expected string
	}{
		{WorkflowStatusPending, "pending"},
		{WorkflowStatusRunning, "running"},
		{WorkflowStatusCompleted, "completed"},
		{WorkflowStatusFailed, "failed"},
		{WorkflowStatusCancelled, "cancelled"},
	}

	for _, tt := range tests {
		if string(tt.status) != tt.expected {
			t.Errorf("expected %s, got %s", tt.expected, tt.status)
		}
	}
}

func TestContextType(t *testing.T) {
	tests := []struct {
		ctxType  ContextType
		expected string
	}{
		{ContextTypeFile, "file"},
		{ContextTypeURL, "url"},
		{ContextTypeText, "text"},
		{ContextTypeCode, "code"},
		{ContextTypeDocument, "document"},
	}

	for _, tt := range tests {
		if string(tt.ctxType) != tt.expected {
			t.Errorf("expected %s, got %s", tt.expected, tt.ctxType)
		}
	}
}

func TestMessageSerialization(t *testing.T) {
	msg := Message{
		ID:             "msg-123",
		ConversationID: "conv-456",
		Role:           RoleUser,
		Content:        "Hello, world!",
		Metadata:       map[string]interface{}{"key": "value"},
		CreatedAt:      time.Date(2024, 1, 1, 0, 0, 0, 0, time.UTC),
	}

	data, err := json.Marshal(msg)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	var decoded Message
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("failed to unmarshal: %v", err)
	}

	if decoded.ID != msg.ID {
		t.Errorf("ID mismatch: expected %s, got %s", msg.ID, decoded.ID)
	}
	if decoded.Role != msg.Role {
		t.Errorf("Role mismatch: expected %s, got %s", msg.Role, decoded.Role)
	}
	if decoded.Content != msg.Content {
		t.Errorf("Content mismatch: expected %s, got %s", msg.Content, decoded.Content)
	}
}

func TestConversationSerialization(t *testing.T) {
	now := time.Now().UTC().Truncate(time.Second)
	conv := Conversation{
		ID:           "conv-123",
		Title:        "Test Conversation",
		UserID:       "user-456",
		TenantID:     "tenant-789",
		Metadata:     map[string]interface{}{"project": "test"},
		MessageCount: 5,
		CreatedAt:    now,
		UpdatedAt:    now,
	}

	data, err := json.Marshal(conv)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	var decoded Conversation
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("failed to unmarshal: %v", err)
	}

	if decoded.ID != conv.ID {
		t.Errorf("ID mismatch: expected %s, got %s", conv.ID, decoded.ID)
	}
	if decoded.MessageCount != conv.MessageCount {
		t.Errorf("MessageCount mismatch: expected %d, got %d", conv.MessageCount, decoded.MessageCount)
	}
}

func TestWorkflowDefinitionSerialization(t *testing.T) {
	now := time.Now().UTC().Truncate(time.Second)
	wf := WorkflowDefinition{
		ID:          "wf-123",
		Name:        "Test Workflow",
		Description: "A test workflow",
		Version:     "1.0.0",
		Steps: []WorkflowStep{
			{
				ID:   "step-1",
				Name: "First Step",
				Type: StepTypeLLM,
				Config: map[string]interface{}{
					"prompt": "Hello",
				},
				NextSteps: []string{"step-2"},
			},
		},
		EntryPoint: "step-1",
		CreatedAt:  now,
		UpdatedAt:  now,
	}

	data, err := json.Marshal(wf)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	var decoded WorkflowDefinition
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("failed to unmarshal: %v", err)
	}

	if decoded.ID != wf.ID {
		t.Errorf("ID mismatch")
	}
	if len(decoded.Steps) != len(wf.Steps) {
		t.Errorf("Steps count mismatch")
	}
	if decoded.Steps[0].Type != StepTypeLLM {
		t.Errorf("Step type mismatch")
	}
}

func TestAPIError(t *testing.T) {
	apiErr := &APIError{
		Code:      "NOT_FOUND",
		Message:   "Resource not found",
		RequestID: "req-123",
	}

	if apiErr.Error() != "Resource not found" {
		t.Errorf("expected 'Resource not found', got '%s'", apiErr.Error())
	}
}

func TestUserSerialization(t *testing.T) {
	now := time.Now().UTC().Truncate(time.Second)
	user := User{
		ID:            "user-123",
		Username:      "testuser",
		Email:         "test@example.com",
		Roles:         []string{"user", "admin"},
		TenantID:      "tenant-456",
		IsActive:      true,
		EmailVerified: true,
		CreatedAt:     now,
		LastLoginAt:   now,
	}

	data, err := json.Marshal(user)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	var decoded User
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("failed to unmarshal: %v", err)
	}

	if decoded.Username != user.Username {
		t.Errorf("Username mismatch")
	}
	if len(decoded.Roles) != len(user.Roles) {
		t.Errorf("Roles count mismatch")
	}
	if decoded.IsActive != user.IsActive {
		t.Errorf("IsActive mismatch")
	}
}

func TestApiKeyScopeSerialization(t *testing.T) {
	apiKey := ApiKeyCreate{
		Name:          "Test Key",
		Scopes:        []ApiKeyScope{ScopeRead, ScopeChat},
		ExpiresInDays: 30,
	}

	data, err := json.Marshal(apiKey)
	if err != nil {
		t.Fatalf("failed to marshal: %v", err)
	}

	var decoded ApiKeyCreate
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("failed to unmarshal: %v", err)
	}

	if decoded.Name != apiKey.Name {
		t.Errorf("Name mismatch")
	}
	if len(decoded.Scopes) != len(apiKey.Scopes) {
		t.Errorf("Scopes count mismatch")
	}
}
