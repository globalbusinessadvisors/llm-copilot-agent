package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Represents the scope/permission of an API key.
 */
public enum ApiKeyScope {
    READ("read"),
    WRITE("write"),
    CHAT("chat"),
    WORKFLOWS("workflows"),
    CONTEXT("context"),
    SANDBOX("sandbox"),
    ADMIN("admin");

    private final String value;

    ApiKeyScope(String value) {
        this.value = value;
    }

    @JsonValue
    public String getValue() {
        return value;
    }
}
