package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

/**
 * Request body for creating a new context item.
 */
public class ContextItemCreate {

    @JsonProperty("name")
    private String name;

    @JsonProperty("type")
    private ContextType type;

    @JsonProperty("content")
    private String content;

    @JsonProperty("url")
    private String url;

    @JsonProperty("metadata")
    private Map<String, Object> metadata;

    // Default constructor for Jackson
    public ContextItemCreate() {}

    // Builder pattern
    public static Builder builder() {
        return new Builder();
    }

    // Getters and setters
    public String getName() {
        return name;
    }

    public void setName(String name) {
        this.name = name;
    }

    public ContextType getType() {
        return type;
    }

    public void setType(ContextType type) {
        this.type = type;
    }

    public String getContent() {
        return content;
    }

    public void setContent(String content) {
        this.content = content;
    }

    public String getUrl() {
        return url;
    }

    public void setUrl(String url) {
        this.url = url;
    }

    public Map<String, Object> getMetadata() {
        return metadata;
    }

    public void setMetadata(Map<String, Object> metadata) {
        this.metadata = metadata;
    }

    /**
     * Builder for creating ContextItemCreate instances.
     */
    public static class Builder {
        private final ContextItemCreate request = new ContextItemCreate();

        public Builder name(String name) {
            request.name = name;
            return this;
        }

        public Builder type(ContextType type) {
            request.type = type;
            return this;
        }

        public Builder content(String content) {
            request.content = content;
            return this;
        }

        public Builder url(String url) {
            request.url = url;
            return this;
        }

        public Builder metadata(Map<String, Object> metadata) {
            request.metadata = metadata;
            return this;
        }

        public ContextItemCreate build() {
            return request;
        }
    }
}
