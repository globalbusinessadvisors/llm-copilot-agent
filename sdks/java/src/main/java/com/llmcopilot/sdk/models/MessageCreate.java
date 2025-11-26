package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

/**
 * Request body for creating a new message.
 */
public class MessageCreate {

    @JsonProperty("role")
    private MessageRole role;

    @JsonProperty("content")
    private String content;

    @JsonProperty("metadata")
    private Map<String, Object> metadata;

    // Default constructor for Jackson
    public MessageCreate() {}

    public MessageCreate(String content) {
        this.role = MessageRole.USER;
        this.content = content;
    }

    public MessageCreate(MessageRole role, String content) {
        this.role = role;
        this.content = content;
    }

    // Builder pattern
    public static Builder builder() {
        return new Builder();
    }

    // Getters and setters
    public MessageRole getRole() {
        return role;
    }

    public void setRole(MessageRole role) {
        this.role = role;
    }

    public String getContent() {
        return content;
    }

    public void setContent(String content) {
        this.content = content;
    }

    public Map<String, Object> getMetadata() {
        return metadata;
    }

    public void setMetadata(Map<String, Object> metadata) {
        this.metadata = metadata;
    }

    /**
     * Builder for creating MessageCreate instances.
     */
    public static class Builder {
        private final MessageCreate request = new MessageCreate();

        public Builder role(MessageRole role) {
            request.role = role;
            return this;
        }

        public Builder content(String content) {
            request.content = content;
            return this;
        }

        public Builder metadata(Map<String, Object> metadata) {
            request.metadata = metadata;
            return this;
        }

        public MessageCreate build() {
            if (request.role == null) {
                request.role = MessageRole.USER;
            }
            return request;
        }
    }
}
