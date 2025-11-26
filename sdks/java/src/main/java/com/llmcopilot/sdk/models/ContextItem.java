package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.time.Instant;
import java.util.Map;
import java.util.Objects;

/**
 * Represents a context item.
 */
public class ContextItem {

    @JsonProperty("id")
    private String id;

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

    @JsonProperty("token_count")
    private Integer tokenCount;

    @JsonProperty("created_at")
    private Instant createdAt;

    @JsonProperty("updated_at")
    private Instant updatedAt;

    // Default constructor for Jackson
    public ContextItem() {}

    // Builder pattern
    public static Builder builder() {
        return new Builder();
    }

    // Getters
    public String getId() {
        return id;
    }

    public String getName() {
        return name;
    }

    public ContextType getType() {
        return type;
    }

    public String getContent() {
        return content;
    }

    public String getUrl() {
        return url;
    }

    public Map<String, Object> getMetadata() {
        return metadata;
    }

    public Integer getTokenCount() {
        return tokenCount;
    }

    public Instant getCreatedAt() {
        return createdAt;
    }

    public Instant getUpdatedAt() {
        return updatedAt;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        ContextItem that = (ContextItem) o;
        return Objects.equals(id, that.id);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id);
    }

    @Override
    public String toString() {
        return "ContextItem{" +
                "id='" + id + '\'' +
                ", name='" + name + '\'' +
                ", type=" + type +
                '}';
    }

    /**
     * Builder for creating ContextItem instances.
     */
    public static class Builder {
        private final ContextItem item = new ContextItem();

        public Builder id(String id) {
            item.id = id;
            return this;
        }

        public Builder name(String name) {
            item.name = name;
            return this;
        }

        public Builder type(ContextType type) {
            item.type = type;
            return this;
        }

        public Builder content(String content) {
            item.content = content;
            return this;
        }

        public Builder url(String url) {
            item.url = url;
            return this;
        }

        public Builder metadata(Map<String, Object> metadata) {
            item.metadata = metadata;
            return this;
        }

        public Builder tokenCount(Integer tokenCount) {
            item.tokenCount = tokenCount;
            return this;
        }

        public Builder createdAt(Instant createdAt) {
            item.createdAt = createdAt;
            return this;
        }

        public Builder updatedAt(Instant updatedAt) {
            item.updatedAt = updatedAt;
            return this;
        }

        public ContextItem build() {
            return item;
        }
    }
}
