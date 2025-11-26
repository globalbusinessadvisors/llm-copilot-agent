package com.llmcopilot.sdk;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;
import com.llmcopilot.sdk.models.*;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.time.Instant;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class ModelsTest {

    private ObjectMapper objectMapper;

    @BeforeEach
    void setUp() {
        objectMapper = new ObjectMapper()
                .registerModule(new JavaTimeModule());
    }

    @Test
    void testMessageRoleSerialization() {
        assertEquals("user", MessageRole.USER.getValue());
        assertEquals("assistant", MessageRole.ASSISTANT.getValue());
        assertEquals("system", MessageRole.SYSTEM.getValue());
    }

    @Test
    void testWorkflowStatusSerialization() {
        assertEquals("pending", WorkflowStatus.PENDING.getValue());
        assertEquals("running", WorkflowStatus.RUNNING.getValue());
        assertEquals("completed", WorkflowStatus.COMPLETED.getValue());
        assertEquals("failed", WorkflowStatus.FAILED.getValue());
        assertEquals("cancelled", WorkflowStatus.CANCELLED.getValue());
    }

    @Test
    void testContextTypeSerialization() {
        assertEquals("file", ContextType.FILE.getValue());
        assertEquals("url", ContextType.URL.getValue());
        assertEquals("text", ContextType.TEXT.getValue());
        assertEquals("code", ContextType.CODE.getValue());
        assertEquals("document", ContextType.DOCUMENT.getValue());
    }

    @Test
    void testApiKeyScopeSerialization() {
        assertEquals("read", ApiKeyScope.READ.getValue());
        assertEquals("write", ApiKeyScope.WRITE.getValue());
        assertEquals("chat", ApiKeyScope.CHAT.getValue());
        assertEquals("admin", ApiKeyScope.ADMIN.getValue());
    }

    @Test
    void testMessageSerialization() throws Exception {
        Message message = Message.builder()
                .id("msg-123")
                .conversationId("conv-456")
                .role(MessageRole.USER)
                .content("Hello, world!")
                .metadata(Map.of("key", "value"))
                .build();

        String json = objectMapper.writeValueAsString(message);
        Message deserialized = objectMapper.readValue(json, Message.class);

        assertEquals(message.getId(), deserialized.getId());
        assertEquals(message.getConversationId(), deserialized.getConversationId());
        assertEquals(message.getRole(), deserialized.getRole());
        assertEquals(message.getContent(), deserialized.getContent());
    }

    @Test
    void testConversationSerialization() throws Exception {
        Conversation conv = Conversation.builder()
                .id("conv-123")
                .title("Test Conversation")
                .userId("user-456")
                .messageCount(5)
                .build();

        String json = objectMapper.writeValueAsString(conv);
        Conversation deserialized = objectMapper.readValue(json, Conversation.class);

        assertEquals(conv.getId(), deserialized.getId());
        assertEquals(conv.getTitle(), deserialized.getTitle());
        assertEquals(conv.getUserId(), deserialized.getUserId());
        assertEquals(conv.getMessageCount(), deserialized.getMessageCount());
    }

    @Test
    void testWorkflowStepBuilder() {
        WorkflowStep step = WorkflowStep.builder()
                .id("step-1")
                .name("First Step")
                .type(StepType.LLM)
                .config(Map.of("prompt", "Hello"))
                .nextSteps(List.of("step-2"))
                .timeout(30)
                .retryCount(3)
                .build();

        assertEquals("step-1", step.getId());
        assertEquals("First Step", step.getName());
        assertEquals(StepType.LLM, step.getType());
        assertEquals(30, step.getTimeout());
        assertEquals(3, step.getRetryCount());
    }

    @Test
    void testWorkflowDefinitionSerialization() throws Exception {
        WorkflowStep step = WorkflowStep.builder()
                .id("step-1")
                .name("First Step")
                .type(StepType.LLM)
                .build();

        WorkflowDefinition workflow = WorkflowDefinition.builder()
                .id("wf-123")
                .name("Test Workflow")
                .description("A test workflow")
                .version("1.0.0")
                .steps(List.of(step))
                .entryPoint("step-1")
                .build();

        String json = objectMapper.writeValueAsString(workflow);
        WorkflowDefinition deserialized = objectMapper.readValue(json, WorkflowDefinition.class);

        assertEquals(workflow.getId(), deserialized.getId());
        assertEquals(workflow.getName(), deserialized.getName());
        assertEquals(workflow.getVersion(), deserialized.getVersion());
        assertEquals(1, deserialized.getSteps().size());
        assertEquals(StepType.LLM, deserialized.getSteps().get(0).getType());
    }

    @Test
    void testWorkflowRunTerminalStatus() throws Exception {
        String json = "{\"id\":\"run-1\",\"workflow_id\":\"wf-1\",\"status\":\"completed\"}";
        WorkflowRun run = objectMapper.readValue(json, WorkflowRun.class);
        assertTrue(run.isTerminal());

        json = "{\"id\":\"run-2\",\"workflow_id\":\"wf-1\",\"status\":\"running\"}";
        run = objectMapper.readValue(json, WorkflowRun.class);
        assertFalse(run.isTerminal());
    }

    @Test
    void testContextItemBuilder() {
        ContextItem item = ContextItem.builder()
                .id("ctx-123")
                .name("test.txt")
                .type(ContextType.TEXT)
                .content("Hello, World!")
                .tokenCount(100)
                .build();

        assertEquals("ctx-123", item.getId());
        assertEquals("test.txt", item.getName());
        assertEquals(ContextType.TEXT, item.getType());
        assertEquals("Hello, World!", item.getContent());
        assertEquals(100, item.getTokenCount());
    }

    @Test
    void testUserSerialization() throws Exception {
        String json = "{\"id\":\"user-123\",\"username\":\"testuser\",\"email\":\"test@example.com\"," +
                "\"roles\":[\"user\",\"admin\"],\"is_active\":true,\"email_verified\":true}";

        User user = objectMapper.readValue(json, User.class);

        assertEquals("user-123", user.getId());
        assertEquals("testuser", user.getUsername());
        assertEquals("test@example.com", user.getEmail());
        assertEquals(2, user.getRoles().size());
        assertTrue(user.isActive());
        assertTrue(user.isEmailVerified());
    }

    @Test
    void testHealthStatusSerialization() throws Exception {
        String json = "{\"status\":\"healthy\",\"version\":\"1.0.0\",\"uptime_seconds\":3600," +
                "\"components\":{\"database\":\"healthy\",\"cache\":\"healthy\"}}";

        HealthStatus status = objectMapper.readValue(json, HealthStatus.class);

        assertEquals("healthy", status.getStatus());
        assertEquals("1.0.0", status.getVersion());
        assertEquals(3600, status.getUptimeSeconds());
        assertTrue(status.isHealthy());
        assertEquals(2, status.getComponents().size());
    }

    @Test
    void testLoginResponseSerialization() throws Exception {
        String json = "{\"access_token\":\"token-123\",\"refresh_token\":\"refresh-456\"," +
                "\"token_type\":\"Bearer\",\"expires_in\":3600,\"refresh_expires_in\":86400," +
                "\"user\":{\"id\":\"user-123\",\"username\":\"testuser\"}}";

        LoginResponse response = objectMapper.readValue(json, LoginResponse.class);

        assertEquals("token-123", response.getAccessToken());
        assertEquals("refresh-456", response.getRefreshToken());
        assertEquals("Bearer", response.getTokenType());
        assertEquals(3600, response.getExpiresIn());
        assertEquals("testuser", response.getUser().getUsername());
    }

    @Test
    void testMessageCreateBuilder() {
        MessageCreate request = MessageCreate.builder()
                .role(MessageRole.USER)
                .content("Hello!")
                .metadata(Map.of("source", "test"))
                .build();

        assertEquals(MessageRole.USER, request.getRole());
        assertEquals("Hello!", request.getContent());
    }

    @Test
    void testConversationCreateBuilder() {
        ConversationCreate request = ConversationCreate.builder()
                .title("My Conversation")
                .systemPrompt("You are a helpful assistant")
                .metadata(Map.of("project", "test"))
                .build();

        assertEquals("My Conversation", request.getTitle());
        assertEquals("You are a helpful assistant", request.getSystemPrompt());
    }

    @Test
    void testWorkflowRunCreateBuilder() {
        WorkflowRunCreate request = WorkflowRunCreate.builder()
                .workflowId("wf-123")
                .inputs(Map.of("input", "value"))
                .build();

        assertEquals("wf-123", request.getWorkflowId());
        assertEquals("value", request.getInputs().get("input"));
    }

    @Test
    void testEqualsAndHashCode() {
        Message msg1 = Message.builder().id("msg-123").build();
        Message msg2 = Message.builder().id("msg-123").build();
        Message msg3 = Message.builder().id("msg-456").build();

        assertEquals(msg1, msg2);
        assertNotEquals(msg1, msg3);
        assertEquals(msg1.hashCode(), msg2.hashCode());
    }
}
