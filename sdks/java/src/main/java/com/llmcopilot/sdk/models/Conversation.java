package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.time.Instant;
import java.util.Map;
import java.util.Objects;

/**
 * Represents a conversation.
 */
public class Conversation {

    @JsonProperty("id")
    private String id;

    @JsonProperty("title")
    private String title;

    @JsonProperty("user_id")
    private String userId;

    @JsonProperty("tenant_id")
    private String tenantId;

    @JsonProperty("metadata")
    private Map<String, Object> metadata;

    @JsonProperty("message_count")
    private int messageCount;

    @JsonProperty("created_at")
    private Instant createdAt;

    @JsonProperty("updated_at")
    private Instant updatedAt;

    // Default constructor for Jackson
    public Conversation() {}

    // Builder pattern
    public static Builder builder() {
        return new Builder();
    }

    // Getters
    public String getId() {
        return id;
    }

    public String getTitle() {
        return title;
    }

    public String getUserId() {
        return userId;
    }

    public String getTenantId() {
        return tenantId;
    }

    public Map<String, Object> getMetadata() {
        return metadata;
    }

    public int getMessageCount() {
        return messageCount;
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
        Conversation that = (Conversation) o;
        return Objects.equals(id, that.id);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id);
    }

    @Override
    public String toString() {
        return "Conversation{" +
                "id='" + id + '\'' +
                ", title='" + title + '\'' +
                ", messageCount=" + messageCount +
                '}';
    }

    /**
     * Builder for creating Conversation instances.
     */
    public static class Builder {
        private final Conversation conversation = new Conversation();

        public Builder id(String id) {
            conversation.id = id;
            return this;
        }

        public Builder title(String title) {
            conversation.title = title;
            return this;
        }

        public Builder userId(String userId) {
            conversation.userId = userId;
            return this;
        }

        public Builder tenantId(String tenantId) {
            conversation.tenantId = tenantId;
            return this;
        }

        public Builder metadata(Map<String, Object> metadata) {
            conversation.metadata = metadata;
            return this;
        }

        public Builder messageCount(int messageCount) {
            conversation.messageCount = messageCount;
            return this;
        }

        public Builder createdAt(Instant createdAt) {
            conversation.createdAt = createdAt;
            return this;
        }

        public Builder updatedAt(Instant updatedAt) {
            conversation.updatedAt = updatedAt;
            return this;
        }

        public Conversation build() {
            return conversation;
        }
    }
}
