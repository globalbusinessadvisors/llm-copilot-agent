package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

/**
 * Request body for creating a new conversation.
 */
public class ConversationCreate {

    @JsonProperty("title")
    private String title;

    @JsonProperty("metadata")
    private Map<String, Object> metadata;

    @JsonProperty("system_prompt")
    private String systemPrompt;

    // Default constructor for Jackson
    public ConversationCreate() {}

    // Builder pattern
    public static Builder builder() {
        return new Builder();
    }

    // Getters and setters
    public String getTitle() {
        return title;
    }

    public void setTitle(String title) {
        this.title = title;
    }

    public Map<String, Object> getMetadata() {
        return metadata;
    }

    public void setMetadata(Map<String, Object> metadata) {
        this.metadata = metadata;
    }

    public String getSystemPrompt() {
        return systemPrompt;
    }

    public void setSystemPrompt(String systemPrompt) {
        this.systemPrompt = systemPrompt;
    }

    /**
     * Builder for creating ConversationCreate instances.
     */
    public static class Builder {
        private final ConversationCreate request = new ConversationCreate();

        public Builder title(String title) {
            request.title = title;
            return this;
        }

        public Builder metadata(Map<String, Object> metadata) {
            request.metadata = metadata;
            return this;
        }

        public Builder systemPrompt(String systemPrompt) {
            request.systemPrompt = systemPrompt;
            return this;
        }

        public ConversationCreate build() {
            return request;
        }
    }
}
