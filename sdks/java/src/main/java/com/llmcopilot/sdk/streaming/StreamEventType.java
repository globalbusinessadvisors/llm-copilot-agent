package com.llmcopilot.sdk.streaming;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Types of streaming events.
 */
public enum StreamEventType {
    MESSAGE_START("message_start"),
    CONTENT_DELTA("content_delta"),
    MESSAGE_END("message_end"),
    TOOL_USE("tool_use"),
    TOOL_RESULT("tool_result"),
    ERROR("error"),
    PING("ping");

    private final String value;

    StreamEventType(String value) {
        this.value = value;
    }

    @JsonValue
    public String getValue() {
        return value;
    }

    public static StreamEventType fromValue(String value) {
        for (StreamEventType type : values()) {
            if (type.value.equals(value)) {
                return type;
            }
        }
        return CONTENT_DELTA; // Default
    }
}
