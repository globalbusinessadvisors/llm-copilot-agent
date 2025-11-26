package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Represents the type of a workflow step.
 */
public enum StepType {
    LLM("llm"),
    TOOL("tool"),
    CONDITION("condition"),
    PARALLEL("parallel"),
    LOOP("loop"),
    HUMAN_REVIEW("human_review");

    private final String value;

    StepType(String value) {
        this.value = value;
    }

    @JsonValue
    public String getValue() {
        return value;
    }
}
