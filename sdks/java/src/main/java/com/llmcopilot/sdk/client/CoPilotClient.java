package com.llmcopilot.sdk.client;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;
import com.llmcopilot.sdk.exceptions.*;
import com.llmcopilot.sdk.models.*;
import com.llmcopilot.sdk.streaming.*;
import okhttp3.*;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.StringReader;
import java.time.Duration;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import java.util.function.Consumer;

/**
 * Main client for the LLM CoPilot API.
 *
 * <p>Example usage:</p>
 * <pre>{@code
 * // Create client with API key
 * CoPilotClient client = CoPilotClient.builder()
 *     .baseUrl("http://localhost:8080")
 *     .apiKey("your-api-key")
 *     .build();
 *
 * // Create a conversation
 * Conversation conv = client.createConversation(null);
 *
 * // Send a message
 * Message response = client.sendMessage(conv.getId(), "Hello!");
 * System.out.println(response.getContent());
 *
 * // Close the client when done
 * client.close();
 * }</pre>
 */
public class CoPilotClient implements AutoCloseable {

    private static final Logger logger = LoggerFactory.getLogger(CoPilotClient.class);
    private static final MediaType JSON = MediaType.parse("application/json");

    private final CoPilotClientConfig config;
    private final OkHttpClient httpClient;
    private final ObjectMapper objectMapper;
    private volatile String accessToken;

    private CoPilotClient(CoPilotClientConfig config) {
        this.config = config;
        this.accessToken = config.getAccessToken();

        this.httpClient = new OkHttpClient.Builder()
                .connectTimeout(config.getTimeout().toMillis(), TimeUnit.MILLISECONDS)
                .readTimeout(config.getTimeout().toMillis(), TimeUnit.MILLISECONDS)
                .writeTimeout(config.getTimeout().toMillis(), TimeUnit.MILLISECONDS)
                .build();

        this.objectMapper = new ObjectMapper()
                .registerModule(new JavaTimeModule())
                .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
    }

    /**
     * Creates a new client builder.
     */
    public static Builder builder() {
        return new Builder();
    }

    /**
     * Creates a new client with API key authentication.
     */
    public static CoPilotClient withApiKey(String baseUrl, String apiKey) {
        return builder()
                .baseUrl(baseUrl)
                .apiKey(apiKey)
                .build();
    }

    /**
     * Creates a new client with access token authentication.
     */
    public static CoPilotClient withAccessToken(String baseUrl, String accessToken) {
        return builder()
                .baseUrl(baseUrl)
                .accessToken(accessToken)
                .build();
    }

    /**
     * Sets the access token for authentication.
     */
    public void setAccessToken(String accessToken) {
        this.accessToken = accessToken;
    }

    // ================================
    // Health Methods
    // ================================

    /**
     * Performs a health check.
     */
    public HealthStatus healthCheck() {
        return executeSync("GET", "/health", null, HealthStatus.class);
    }

    /**
     * Performs a health check asynchronously.
     */
    public CompletableFuture<HealthStatus> healthCheckAsync() {
        return executeAsync("GET", "/health", null, HealthStatus.class);
    }

    // ================================
    // Authentication Methods
    // ================================

    /**
     * Logs in with username/email and password.
     */
    public LoginResponse login(String usernameOrEmail, String password) {
        LoginRequest request = new LoginRequest(usernameOrEmail, password);
        LoginResponse response = executeSync("POST", "/api/v1/auth/login", request, LoginResponse.class);
        this.accessToken = response.getAccessToken();
        return response;
    }

    /**
     * Logs in asynchronously.
     */
    public CompletableFuture<LoginResponse> loginAsync(String usernameOrEmail, String password) {
        LoginRequest request = new LoginRequest(usernameOrEmail, password);
        return executeAsync("POST", "/api/v1/auth/login", request, LoginResponse.class)
                .thenApply(response -> {
                    this.accessToken = response.getAccessToken();
                    return response;
                });
    }

    /**
     * Refreshes the access tokens.
     */
    public TokenPair refreshTokens(String refreshToken) {
        Map<String, String> request = Map.of("refresh_token", refreshToken);
        TokenPair response = executeSync("POST", "/api/v1/auth/refresh", request, TokenPair.class);
        this.accessToken = response.getAccessToken();
        return response;
    }

    /**
     * Logs out the current user.
     */
    public void logout() {
        executeSync("POST", "/api/v1/auth/logout", null, Void.class);
        this.accessToken = null;
    }

    /**
     * Gets the current authenticated user.
     */
    public User getCurrentUser() {
        return executeSync("GET", "/api/v1/auth/me", null, User.class);
    }

    // ================================
    // Conversation Methods
    // ================================

    /**
     * Creates a new conversation.
     */
    public Conversation createConversation(ConversationCreate request) {
        return executeSync("POST", "/api/v1/conversations",
                request != null ? request : new ConversationCreate(), Conversation.class);
    }

    /**
     * Creates a new conversation asynchronously.
     */
    public CompletableFuture<Conversation> createConversationAsync(ConversationCreate request) {
        return executeAsync("POST", "/api/v1/conversations",
                request != null ? request : new ConversationCreate(), Conversation.class);
    }

    /**
     * Gets a conversation by ID.
     */
    public Conversation getConversation(String conversationId) {
        return executeSync("GET", "/api/v1/conversations/" + conversationId, null, Conversation.class);
    }

    /**
     * Lists conversations with pagination.
     */
    public List<Conversation> listConversations(int limit, int offset) {
        String path = String.format("/api/v1/conversations?limit=%d&offset=%d", limit, offset);
        PaginatedResponse<Conversation> response = executeSync("GET", path, null,
                new TypeReference<PaginatedResponse<Conversation>>() {});
        return response.getItems();
    }

    /**
     * Deletes a conversation.
     */
    public void deleteConversation(String conversationId) {
        executeSync("DELETE", "/api/v1/conversations/" + conversationId, null, Void.class);
    }

    /**
     * Sends a message in a conversation.
     */
    public Message sendMessage(String conversationId, String content) {
        MessageCreate request = new MessageCreate(content);
        String path = String.format("/api/v1/conversations/%s/messages", conversationId);
        return executeSync("POST", path, request, Message.class);
    }

    /**
     * Sends a message asynchronously.
     */
    public CompletableFuture<Message> sendMessageAsync(String conversationId, String content) {
        MessageCreate request = new MessageCreate(content);
        String path = String.format("/api/v1/conversations/%s/messages", conversationId);
        return executeAsync("POST", path, request, Message.class);
    }

    /**
     * Sends a message with streaming response.
     */
    public void sendMessageStream(String conversationId, String content, StreamHandler handler) {
        MessageCreate request = new MessageCreate(content);
        request.setMetadata(Map.of("stream", true));
        String path = String.format("/api/v1/conversations/%s/messages/stream", conversationId);
        executeStream("POST", path, request, handler);
    }

    /**
     * Lists messages in a conversation.
     */
    public List<Message> listMessages(String conversationId, int limit, int offset) {
        String path = String.format("/api/v1/conversations/%s/messages?limit=%d&offset=%d",
                conversationId, limit, offset);
        PaginatedResponse<Message> response = executeSync("GET", path, null,
                new TypeReference<PaginatedResponse<Message>>() {});
        return response.getItems();
    }

    // ================================
    // Workflow Methods
    // ================================

    /**
     * Creates a new workflow definition.
     */
    public WorkflowDefinition createWorkflow(WorkflowDefinitionCreate request) {
        return executeSync("POST", "/api/v1/workflows", request, WorkflowDefinition.class);
    }

    /**
     * Gets a workflow definition by ID.
     */
    public WorkflowDefinition getWorkflow(String workflowId) {
        return executeSync("GET", "/api/v1/workflows/" + workflowId, null, WorkflowDefinition.class);
    }

    /**
     * Lists workflow definitions.
     */
    public List<WorkflowDefinition> listWorkflows() {
        PaginatedResponse<WorkflowDefinition> response = executeSync("GET", "/api/v1/workflows", null,
                new TypeReference<PaginatedResponse<WorkflowDefinition>>() {});
        return response.getItems();
    }

    /**
     * Deletes a workflow definition.
     */
    public void deleteWorkflow(String workflowId) {
        executeSync("DELETE", "/api/v1/workflows/" + workflowId, null, Void.class);
    }

    /**
     * Starts a workflow run.
     */
    public WorkflowRun runWorkflow(WorkflowRunCreate request) {
        return executeSync("POST", "/api/v1/workflows/runs", request, WorkflowRun.class);
    }

    /**
     * Starts a workflow run asynchronously.
     */
    public CompletableFuture<WorkflowRun> runWorkflowAsync(WorkflowRunCreate request) {
        return executeAsync("POST", "/api/v1/workflows/runs", request, WorkflowRun.class);
    }

    /**
     * Gets a workflow run by ID.
     */
    public WorkflowRun getWorkflowRun(String runId) {
        return executeSync("GET", "/api/v1/workflows/runs/" + runId, null, WorkflowRun.class);
    }

    /**
     * Lists workflow runs.
     */
    public List<WorkflowRun> listWorkflowRuns(String workflowId) {
        String path = "/api/v1/workflows/runs";
        if (workflowId != null && !workflowId.isEmpty()) {
            path += "?workflow_id=" + workflowId;
        }
        PaginatedResponse<WorkflowRun> response = executeSync("GET", path, null,
                new TypeReference<PaginatedResponse<WorkflowRun>>() {});
        return response.getItems();
    }

    /**
     * Cancels a workflow run.
     */
    public WorkflowRun cancelWorkflowRun(String runId) {
        return executeSync("POST", "/api/v1/workflows/runs/" + runId + "/cancel", null, WorkflowRun.class);
    }

    // ================================
    // Context Methods
    // ================================

    /**
     * Creates a context item.
     */
    public ContextItem createContextItem(ContextItemCreate request) {
        return executeSync("POST", "/api/v1/context", request, ContextItem.class);
    }

    /**
     * Gets a context item by ID.
     */
    public ContextItem getContextItem(String itemId) {
        return executeSync("GET", "/api/v1/context/" + itemId, null, ContextItem.class);
    }

    /**
     * Lists context items.
     */
    public List<ContextItem> listContextItems() {
        PaginatedResponse<ContextItem> response = executeSync("GET", "/api/v1/context", null,
                new TypeReference<PaginatedResponse<ContextItem>>() {});
        return response.getItems();
    }

    /**
     * Deletes a context item.
     */
    public void deleteContextItem(String itemId) {
        executeSync("DELETE", "/api/v1/context/" + itemId, null, Void.class);
    }

    // ================================
    // Internal Methods
    // ================================

    private <T> T executeSync(String method, String path, Object body, Class<T> responseType) {
        return executeWithRetry(() -> doRequest(method, path, body, responseType));
    }

    private <T> T executeSync(String method, String path, Object body, TypeReference<T> responseType) {
        return executeWithRetry(() -> doRequest(method, path, body, responseType));
    }

    private <T> CompletableFuture<T> executeAsync(String method, String path, Object body, Class<T> responseType) {
        return CompletableFuture.supplyAsync(() -> executeSync(method, path, body, responseType));
    }

    private <T> T executeWithRetry(RequestExecutor<T> executor) {
        int maxRetries = config.getMaxRetries();

        // If retries are disabled, just execute once
        if (maxRetries < 0) {
            return executor.execute();
        }

        CoPilotException lastException = null;

        for (int attempt = 0; attempt <= maxRetries; attempt++) {
            try {
                if (attempt > 0) {
                    long delay = calculateBackoff(attempt);
                    Thread.sleep(delay);
                }
                return executor.execute();
            } catch (CoPilotException e) {
                lastException = e;
                if (!e.isRetryable()) {
                    throw e;
                }
                logger.warn("Request failed (attempt {}/{}): {}", attempt + 1, maxRetries + 1, e.getMessage());
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                throw new CoPilotException("Request interrupted", e);
            }
        }

        throw new CoPilotException("Max retries exceeded: " + lastException.getMessage(), lastException);
    }

    private long calculateBackoff(int attempt) {
        long minDelay = config.getRetryMinDelay().toMillis();
        long maxDelay = config.getRetryMaxDelay().toMillis();
        long delay = minDelay * (1L << (attempt - 1));
        return Math.min(delay, maxDelay);
    }

    private <T> T doRequest(String method, String path, Object body, Class<T> responseType) {
        try {
            Response response = executeHttpRequest(method, path, body);
            return parseResponse(response, responseType);
        } catch (IOException e) {
            throw new CoPilotException("Request failed: " + e.getMessage(), e);
        }
    }

    private <T> T doRequest(String method, String path, Object body, TypeReference<T> responseType) {
        try {
            Response response = executeHttpRequest(method, path, body);
            return parseResponse(response, responseType);
        } catch (IOException e) {
            throw new CoPilotException("Request failed: " + e.getMessage(), e);
        }
    }

    private Response executeHttpRequest(String method, String path, Object body) throws IOException {
        String url = config.getBaseUrl() + path;

        Request.Builder requestBuilder = new Request.Builder()
                .url(url)
                .header("Content-Type", "application/json")
                .header("Accept", "application/json");

        // Add authentication
        if (config.getApiKey() != null) {
            requestBuilder.header("X-API-Key", config.getApiKey());
        } else if (accessToken != null) {
            requestBuilder.header("Authorization", "Bearer " + accessToken);
        }

        RequestBody requestBody = null;
        if (body != null) {
            requestBody = RequestBody.create(objectMapper.writeValueAsString(body), JSON);
        }

        switch (method.toUpperCase()) {
            case "GET":
                requestBuilder.get();
                break;
            case "POST":
                requestBuilder.post(requestBody != null ? requestBody : RequestBody.create("", JSON));
                break;
            case "PUT":
                requestBuilder.put(requestBody != null ? requestBody : RequestBody.create("", JSON));
                break;
            case "DELETE":
                requestBuilder.delete(requestBody);
                break;
            default:
                throw new IllegalArgumentException("Unsupported HTTP method: " + method);
        }

        return httpClient.newCall(requestBuilder.build()).execute();
    }

    private <T> T parseResponse(Response response, Class<T> responseType) throws IOException {
        try (ResponseBody responseBody = response.body()) {
            String bodyString = responseBody != null ? responseBody.string() : "";

            if (!response.isSuccessful()) {
                handleErrorResponse(response.code(), bodyString);
            }

            if (responseType == Void.class || bodyString.isEmpty()) {
                return null;
            }

            return objectMapper.readValue(bodyString, responseType);
        }
    }

    private <T> T parseResponse(Response response, TypeReference<T> responseType) throws IOException {
        try (ResponseBody responseBody = response.body()) {
            String bodyString = responseBody != null ? responseBody.string() : "";

            if (!response.isSuccessful()) {
                handleErrorResponse(response.code(), bodyString);
            }

            if (bodyString.isEmpty()) {
                return null;
            }

            return objectMapper.readValue(bodyString, responseType);
        }
    }

    private void handleErrorResponse(int statusCode, String body) {
        ApiError error = null;
        try {
            error = objectMapper.readValue(body, ApiError.class);
        } catch (JsonProcessingException e) {
            // Ignore JSON parsing errors
        }

        if (error == null) {
            error = new ApiError();
        }

        switch (statusCode) {
            case 401:
                throw new AuthenticationException(error);
            case 404:
                throw new NotFoundException(error);
            case 429:
                throw new RateLimitException(error);
            default:
                if (statusCode >= 500) {
                    throw new ServerException(statusCode, error);
                }
                throw new CoPilotException(statusCode, error);
        }
    }

    private void executeStream(String method, String path, Object body, StreamHandler handler) {
        try {
            String url = config.getBaseUrl() + path;

            Request.Builder requestBuilder = new Request.Builder()
                    .url(url)
                    .header("Content-Type", "application/json")
                    .header("Accept", "text/event-stream");

            if (config.getApiKey() != null) {
                requestBuilder.header("X-API-Key", config.getApiKey());
            } else if (accessToken != null) {
                requestBuilder.header("Authorization", "Bearer " + accessToken);
            }

            RequestBody requestBody = null;
            if (body != null) {
                requestBody = RequestBody.create(objectMapper.writeValueAsString(body), JSON);
            }
            requestBuilder.post(requestBody != null ? requestBody : RequestBody.create("", JSON));

            Response response = httpClient.newCall(requestBuilder.build()).execute();

            if (!response.isSuccessful()) {
                String bodyString = response.body() != null ? response.body().string() : "";
                handleErrorResponse(response.code(), bodyString);
            }

            try (ResponseBody responseBody = response.body()) {
                if (responseBody == null) return;

                BufferedReader reader = new BufferedReader(new StringReader(responseBody.string()));
                String line;

                while ((line = reader.readLine()) != null) {
                    line = line.trim();
                    if (line.isEmpty()) continue;

                    if (line.startsWith("data: ")) {
                        String data = line.substring(6);
                        if ("[DONE]".equals(data)) {
                            break;
                        }

                        StreamEvent event = parseStreamEvent(data);
                        if (event != null) {
                            handler.onEvent(event);

                            switch (event.getType()) {
                                case MESSAGE_START:
                                    handler.onStart(event.getMessageId());
                                    break;
                                case CONTENT_DELTA:
                                    handler.onContent(event.getContent());
                                    break;
                                case MESSAGE_END:
                                    handler.onEnd(event.getMessageId());
                                    break;
                                case ERROR:
                                    handler.onError(event.getError());
                                    break;
                            }

                            if (event.isFinal()) {
                                break;
                            }
                        }
                    }
                }
            }
        } catch (IOException e) {
            throw new CoPilotException("Stream request failed: " + e.getMessage(), e);
        }
    }

    private StreamEvent parseStreamEvent(String data) {
        try {
            Map<String, Object> raw = objectMapper.readValue(data, new TypeReference<>() {});
            StreamEvent event = new StreamEvent();

            String typeStr = (String) raw.get("type");
            event.setType(typeStr != null ? StreamEventType.fromValue(typeStr) : StreamEventType.CONTENT_DELTA);

            event.setData(raw);
            event.setMessageId((String) raw.getOrDefault("message_id", raw.get("id")));
            event.setError((String) raw.get("error"));

            Object deltaObj = raw.get("delta");
            if (deltaObj instanceof Map) {
                @SuppressWarnings("unchecked")
                Map<String, Object> deltaMap = (Map<String, Object>) deltaObj;
                StreamDelta delta = objectMapper.convertValue(deltaMap, StreamDelta.class);
                event.setDelta(delta);
            }

            return event;
        } catch (JsonProcessingException e) {
            logger.warn("Failed to parse stream event: {}", e.getMessage());
            return null;
        }
    }

    @Override
    public void close() {
        httpClient.dispatcher().executorService().shutdown();
        httpClient.connectionPool().evictAll();
    }

    @FunctionalInterface
    private interface RequestExecutor<T> {
        T execute();
    }

    /**
     * Builder for creating CoPilotClient instances.
     */
    public static class Builder {
        private final CoPilotClientConfig.Builder configBuilder = CoPilotClientConfig.builder();

        public Builder baseUrl(String baseUrl) {
            configBuilder.baseUrl(baseUrl);
            return this;
        }

        public Builder apiKey(String apiKey) {
            configBuilder.apiKey(apiKey);
            return this;
        }

        public Builder accessToken(String accessToken) {
            configBuilder.accessToken(accessToken);
            return this;
        }

        public Builder timeout(Duration timeout) {
            configBuilder.timeout(timeout);
            return this;
        }

        public Builder maxRetries(int maxRetries) {
            configBuilder.maxRetries(maxRetries);
            return this;
        }

        public Builder retryMinDelay(Duration retryMinDelay) {
            configBuilder.retryMinDelay(retryMinDelay);
            return this;
        }

        public Builder retryMaxDelay(Duration retryMaxDelay) {
            configBuilder.retryMaxDelay(retryMaxDelay);
            return this;
        }

        public CoPilotClient build() {
            return new CoPilotClient(configBuilder.build());
        }
    }
}
