package com.llmcopilot.sdk.client;

import java.time.Duration;
import java.util.Objects;

/**
 * Configuration for the CoPilot client.
 */
public class CoPilotClientConfig {

    private final String baseUrl;
    private final String apiKey;
    private final String accessToken;
    private final Duration timeout;
    private final int maxRetries;
    private final Duration retryMinDelay;
    private final Duration retryMaxDelay;

    private CoPilotClientConfig(Builder builder) {
        this.baseUrl = builder.baseUrl;
        this.apiKey = builder.apiKey;
        this.accessToken = builder.accessToken;
        this.timeout = builder.timeout;
        this.maxRetries = builder.maxRetries;
        this.retryMinDelay = builder.retryMinDelay;
        this.retryMaxDelay = builder.retryMaxDelay;
    }

    public static Builder builder() {
        return new Builder();
    }

    public static CoPilotClientConfig defaultConfig() {
        return builder().build();
    }

    public String getBaseUrl() {
        return baseUrl;
    }

    public String getApiKey() {
        return apiKey;
    }

    public String getAccessToken() {
        return accessToken;
    }

    public Duration getTimeout() {
        return timeout;
    }

    public int getMaxRetries() {
        return maxRetries;
    }

    public Duration getRetryMinDelay() {
        return retryMinDelay;
    }

    public Duration getRetryMaxDelay() {
        return retryMaxDelay;
    }

    /**
     * Builder for creating CoPilotClientConfig instances.
     */
    public static class Builder {
        private String baseUrl = "http://localhost:8080";
        private String apiKey;
        private String accessToken;
        private Duration timeout = Duration.ofSeconds(30);
        private int maxRetries = 3;
        private Duration retryMinDelay = Duration.ofSeconds(1);
        private Duration retryMaxDelay = Duration.ofSeconds(30);

        public Builder baseUrl(String baseUrl) {
            this.baseUrl = Objects.requireNonNull(baseUrl, "baseUrl must not be null");
            return this;
        }

        public Builder apiKey(String apiKey) {
            this.apiKey = apiKey;
            return this;
        }

        public Builder accessToken(String accessToken) {
            this.accessToken = accessToken;
            return this;
        }

        public Builder timeout(Duration timeout) {
            this.timeout = Objects.requireNonNull(timeout, "timeout must not be null");
            return this;
        }

        public Builder maxRetries(int maxRetries) {
            this.maxRetries = maxRetries;
            return this;
        }

        public Builder retryMinDelay(Duration retryMinDelay) {
            this.retryMinDelay = Objects.requireNonNull(retryMinDelay, "retryMinDelay must not be null");
            return this;
        }

        public Builder retryMaxDelay(Duration retryMaxDelay) {
            this.retryMaxDelay = Objects.requireNonNull(retryMaxDelay, "retryMaxDelay must not be null");
            return this;
        }

        public CoPilotClientConfig build() {
            return new CoPilotClientConfig(this);
        }
    }
}
