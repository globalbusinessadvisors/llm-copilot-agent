package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.time.Instant;
import java.util.Map;
import java.util.Objects;

/**
 * Represents a message in a conversation.
 */
public class Message {

    @JsonProperty("id")
    private String id;

    @JsonProperty("conversation_id")
    private String conversationId;

    @JsonProperty("role")
    private MessageRole role;

    @JsonProperty("content")
    private String content;

    @JsonProperty("metadata")
    private Map<String, Object> metadata;

    @JsonProperty("tokens_used")
    private Integer tokensUsed;

    @JsonProperty("model")
    private String model;

    @JsonProperty("created_at")
    private Instant createdAt;

    // Default constructor for Jackson
    public Message() {}

    // Builder pattern
    public static Builder builder() {
        return new Builder();
    }

    // Getters
    public String getId() {
        return id;
    }

    public String getConversationId() {
        return conversationId;
    }

    public MessageRole getRole() {
        return role;
    }

    public String getContent() {
        return content;
    }

    public Map<String, Object> getMetadata() {
        return metadata;
    }

    public Integer getTokensUsed() {
        return tokensUsed;
    }

    public String getModel() {
        return model;
    }

    public Instant getCreatedAt() {
        return createdAt;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        Message message = (Message) o;
        return Objects.equals(id, message.id);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id);
    }

    @Override
    public String toString() {
        return "Message{" +
                "id='" + id + '\'' +
                ", conversationId='" + conversationId + '\'' +
                ", role=" + role +
                ", content='" + (content != null && content.length() > 50 ? content.substring(0, 50) + "..." : content) + '\'' +
                '}';
    }

    /**
     * Builder for creating Message instances.
     */
    public static class Builder {
        private final Message message = new Message();

        public Builder id(String id) {
            message.id = id;
            return this;
        }

        public Builder conversationId(String conversationId) {
            message.conversationId = conversationId;
            return this;
        }

        public Builder role(MessageRole role) {
            message.role = role;
            return this;
        }

        public Builder content(String content) {
            message.content = content;
            return this;
        }

        public Builder metadata(Map<String, Object> metadata) {
            message.metadata = metadata;
            return this;
        }

        public Builder tokensUsed(Integer tokensUsed) {
            message.tokensUsed = tokensUsed;
            return this;
        }

        public Builder model(String model) {
            message.model = model;
            return this;
        }

        public Builder createdAt(Instant createdAt) {
            message.createdAt = createdAt;
            return this;
        }

        public Message build() {
            return message;
        }
    }
}
