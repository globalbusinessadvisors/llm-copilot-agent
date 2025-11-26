package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Represents the type of a context item.
 */
public enum ContextType {
    FILE("file"),
    URL("url"),
    TEXT("text"),
    CODE("code"),
    DOCUMENT("document");

    private final String value;

    ContextType(String value) {
        this.value = value;
    }

    @JsonValue
    public String getValue() {
        return value;
    }
}
