package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Represents the role of a message in a conversation.
 */
public enum MessageRole {
    USER("user"),
    ASSISTANT("assistant"),
    SYSTEM("system");

    private final String value;

    MessageRole(String value) {
        this.value = value;
    }

    @JsonValue
    public String getValue() {
        return value;
    }
}
