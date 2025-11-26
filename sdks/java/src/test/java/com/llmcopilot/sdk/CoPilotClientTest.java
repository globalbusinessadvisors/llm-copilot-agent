package com.llmcopilot.sdk;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;
import com.llmcopilot.sdk.client.CoPilotClient;
import com.llmcopilot.sdk.exceptions.*;
import com.llmcopilot.sdk.models.*;
import okhttp3.mockwebserver.MockResponse;
import okhttp3.mockwebserver.MockWebServer;
import okhttp3.mockwebserver.RecordedRequest;
import org.junit.jupiter.api.*;

import java.io.IOException;
import java.time.Duration;
import java.time.Instant;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;

import static org.junit.jupiter.api.Assertions.*;

class CoPilotClientTest {

    private MockWebServer mockServer;
    private CoPilotClient client;
    private ObjectMapper objectMapper;

    @BeforeEach
    void setUp() throws IOException {
        mockServer = new MockWebServer();
        mockServer.start();

        client = CoPilotClient.builder()
                .baseUrl(mockServer.url("/").toString().replaceAll("/$", ""))
                .apiKey("test-api-key")
                .maxRetries(-1) // Disable retries for predictable tests
                .build();

        objectMapper = new ObjectMapper()
                .registerModule(new JavaTimeModule());
    }

    @AfterEach
    void tearDown() throws IOException {
        client.close();
        mockServer.shutdown();
    }

    @Test
    void testHealthCheck() throws Exception {
        HealthStatus expected = new HealthStatus();
        mockServer.enqueue(new MockResponse()
                .setBody("{\"status\":\"healthy\",\"version\":\"1.0.0\",\"uptime_seconds\":3600}")
                .setHeader("Content-Type", "application/json"));

        HealthStatus result = client.healthCheck();

        assertEquals("healthy", result.getStatus());
        assertEquals("1.0.0", result.getVersion());
        assertEquals(3600, result.getUptimeSeconds());
        assertTrue(result.isHealthy());

        RecordedRequest request = mockServer.takeRequest();
        assertEquals("GET", request.getMethod());
        assertEquals("/health", request.getPath());
        assertEquals("test-api-key", request.getHeader("X-API-Key"));
    }

    @Test
    void testCreateConversation() throws Exception {
        String responseJson = "{\"id\":\"conv-123\",\"user_id\":\"user-456\",\"message_count\":0}";
        mockServer.enqueue(new MockResponse()
                .setBody(responseJson)
                .setHeader("Content-Type", "application/json"));

        Conversation result = client.createConversation(null);

        assertEquals("conv-123", result.getId());
        assertEquals("user-456", result.getUserId());
        assertEquals(0, result.getMessageCount());

        RecordedRequest request = mockServer.takeRequest();
        assertEquals("POST", request.getMethod());
        assertEquals("/api/v1/conversations", request.getPath());
    }

    @Test
    void testSendMessage() throws Exception {
        String responseJson = "{\"id\":\"msg-789\",\"conversation_id\":\"conv-123\"," +
                "\"role\":\"assistant\",\"content\":\"Hello! How can I help you?\"}";
        mockServer.enqueue(new MockResponse()
                .setBody(responseJson)
                .setHeader("Content-Type", "application/json"));

        Message result = client.sendMessage("conv-123", "Hello!");

        assertEquals("msg-789", result.getId());
        assertEquals("conv-123", result.getConversationId());
        assertEquals(MessageRole.ASSISTANT, result.getRole());
        assertEquals("Hello! How can I help you?", result.getContent());

        RecordedRequest request = mockServer.takeRequest();
        assertEquals("POST", request.getMethod());
        assertEquals("/api/v1/conversations/conv-123/messages", request.getPath());
        assertTrue(request.getBody().readUtf8().contains("\"content\":\"Hello!\""));
    }

    @Test
    void testSendMessageAsync() throws Exception {
        String responseJson = "{\"id\":\"msg-async\",\"conversation_id\":\"conv-123\"," +
                "\"role\":\"assistant\",\"content\":\"Async response\"}";
        mockServer.enqueue(new MockResponse()
                .setBody(responseJson)
                .setHeader("Content-Type", "application/json"));

        CompletableFuture<Message> future = client.sendMessageAsync("conv-123", "Hello async!");
        Message result = future.get(5, TimeUnit.SECONDS);

        assertEquals("msg-async", result.getId());
        assertEquals("Async response", result.getContent());
    }

    @Test
    void testLogin() throws Exception {
        String responseJson = "{\"access_token\":\"access-123\",\"refresh_token\":\"refresh-456\"," +
                "\"token_type\":\"Bearer\",\"expires_in\":3600,\"refresh_expires_in\":86400," +
                "\"user\":{\"id\":\"user-123\",\"username\":\"testuser\",\"email\":\"test@example.com\"}}";
        mockServer.enqueue(new MockResponse()
                .setBody(responseJson)
                .setHeader("Content-Type", "application/json"));

        LoginResponse result = client.login("testuser", "password123");

        assertEquals("access-123", result.getAccessToken());
        assertEquals("refresh-456", result.getRefreshToken());
        assertEquals("Bearer", result.getTokenType());
        assertEquals("testuser", result.getUser().getUsername());

        RecordedRequest request = mockServer.takeRequest();
        assertEquals("POST", request.getMethod());
        assertEquals("/api/v1/auth/login", request.getPath());
    }

    @Test
    void testUnauthorizedError() {
        mockServer.enqueue(new MockResponse()
                .setResponseCode(401)
                .setBody("{\"code\":\"UNAUTHORIZED\",\"message\":\"Invalid credentials\"}")
                .setHeader("Content-Type", "application/json"));

        AuthenticationException ex = assertThrows(AuthenticationException.class, () -> {
            client.healthCheck();
        });

        assertEquals(401, ex.getStatusCode());
        assertTrue(ex.isUnauthorized());
    }

    @Test
    void testNotFoundError() {
        mockServer.enqueue(new MockResponse()
                .setResponseCode(404)
                .setBody("{\"code\":\"NOT_FOUND\",\"message\":\"Resource not found\"}")
                .setHeader("Content-Type", "application/json"));

        NotFoundException ex = assertThrows(NotFoundException.class, () -> {
            client.getConversation("nonexistent");
        });

        assertEquals(404, ex.getStatusCode());
        assertTrue(ex.isNotFound());
    }

    @Test
    void testRateLimitError() {
        mockServer.enqueue(new MockResponse()
                .setResponseCode(429)
                .setBody("{\"code\":\"RATE_LIMITED\",\"message\":\"Too many requests\"}")
                .setHeader("Content-Type", "application/json"));

        RateLimitException ex = assertThrows(RateLimitException.class, () -> {
            client.healthCheck();
        });

        assertEquals(429, ex.getStatusCode());
        assertTrue(ex.isRateLimited());
    }

    @Test
    void testServerError() {
        mockServer.enqueue(new MockResponse()
                .setResponseCode(500)
                .setBody("{\"code\":\"SERVER_ERROR\",\"message\":\"Internal server error\"}")
                .setHeader("Content-Type", "application/json"));

        ServerException ex = assertThrows(ServerException.class, () -> {
            client.healthCheck();
        });

        assertEquals(500, ex.getStatusCode());
        assertTrue(ex.isServerError());
    }

    @Test
    void testCreateWorkflow() throws Exception {
        String responseJson = "{\"id\":\"wf-123\",\"name\":\"Test Workflow\",\"version\":\"1.0.0\"}";
        mockServer.enqueue(new MockResponse()
                .setBody(responseJson)
                .setHeader("Content-Type", "application/json"));

        WorkflowDefinitionCreate request = WorkflowDefinitionCreate.builder()
                .name("Test Workflow")
                .version("1.0.0")
                .entryPoint("step-1")
                .build();

        WorkflowDefinition result = client.createWorkflow(request);

        assertEquals("wf-123", result.getId());
        assertEquals("Test Workflow", result.getName());
    }

    @Test
    void testRunWorkflow() throws Exception {
        String responseJson = "{\"id\":\"run-123\",\"workflow_id\":\"wf-123\",\"status\":\"running\"}";
        mockServer.enqueue(new MockResponse()
                .setBody(responseJson)
                .setHeader("Content-Type", "application/json"));

        WorkflowRunCreate request = new WorkflowRunCreate("wf-123", Map.of("input", "value"));

        WorkflowRun result = client.runWorkflow(request);

        assertEquals("run-123", result.getId());
        assertEquals("wf-123", result.getWorkflowId());
        assertEquals(WorkflowStatus.RUNNING, result.getStatus());
    }

    @Test
    void testCreateContextItem() throws Exception {
        String responseJson = "{\"id\":\"ctx-123\",\"name\":\"test.txt\",\"type\":\"text\"}";
        mockServer.enqueue(new MockResponse()
                .setBody(responseJson)
                .setHeader("Content-Type", "application/json"));

        ContextItemCreate request = ContextItemCreate.builder()
                .name("test.txt")
                .type(ContextType.TEXT)
                .content("Hello, World!")
                .build();

        ContextItem result = client.createContextItem(request);

        assertEquals("ctx-123", result.getId());
        assertEquals("test.txt", result.getName());
        assertEquals(ContextType.TEXT, result.getType());
    }

    @Test
    void testListConversations() throws Exception {
        String responseJson = "{\"items\":[{\"id\":\"conv-1\"},{\"id\":\"conv-2\"}],\"total\":2}";
        mockServer.enqueue(new MockResponse()
                .setBody(responseJson)
                .setHeader("Content-Type", "application/json"));

        List<Conversation> result = client.listConversations(10, 0);

        assertEquals(2, result.size());
        assertEquals("conv-1", result.get(0).getId());
        assertEquals("conv-2", result.get(1).getId());

        RecordedRequest request = mockServer.takeRequest();
        assertEquals("/api/v1/conversations?limit=10&offset=0", request.getPath());
    }

    @Test
    void testDeleteConversation() throws Exception {
        mockServer.enqueue(new MockResponse().setResponseCode(204));

        assertDoesNotThrow(() -> client.deleteConversation("conv-123"));

        RecordedRequest request = mockServer.takeRequest();
        assertEquals("DELETE", request.getMethod());
        assertEquals("/api/v1/conversations/conv-123", request.getPath());
    }

    @Test
    void testBuilderPattern() {
        Message message = Message.builder()
                .id("msg-123")
                .conversationId("conv-456")
                .role(MessageRole.USER)
                .content("Hello!")
                .build();

        assertEquals("msg-123", message.getId());
        assertEquals("conv-456", message.getConversationId());
        assertEquals(MessageRole.USER, message.getRole());
        assertEquals("Hello!", message.getContent());
    }
}
