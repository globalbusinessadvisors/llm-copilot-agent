package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Represents the status of a workflow run.
 */
public enum WorkflowStatus {
    PENDING("pending"),
    RUNNING("running"),
    COMPLETED("completed"),
    FAILED("failed"),
    CANCELLED("cancelled");

    private final String value;

    WorkflowStatus(String value) {
        this.value = value;
    }

    @JsonValue
    public String getValue() {
        return value;
    }
}
