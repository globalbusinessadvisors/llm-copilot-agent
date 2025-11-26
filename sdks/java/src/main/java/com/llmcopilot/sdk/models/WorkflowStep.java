package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;
import java.util.Map;

/**
 * Represents a step in a workflow definition.
 */
public class WorkflowStep {

    @JsonProperty("id")
    private String id;

    @JsonProperty("name")
    private String name;

    @JsonProperty("type")
    private StepType type;

    @JsonProperty("config")
    private Map<String, Object> config;

    @JsonProperty("next_steps")
    private List<String> nextSteps;

    @JsonProperty("on_error")
    private String onError;

    @JsonProperty("timeout")
    private Integer timeout;

    @JsonProperty("retry_count")
    private Integer retryCount;

    // Default constructor for Jackson
    public WorkflowStep() {}

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

    public StepType getType() {
        return type;
    }

    public Map<String, Object> getConfig() {
        return config;
    }

    public List<String> getNextSteps() {
        return nextSteps;
    }

    public String getOnError() {
        return onError;
    }

    public Integer getTimeout() {
        return timeout;
    }

    public Integer getRetryCount() {
        return retryCount;
    }

    /**
     * Builder for creating WorkflowStep instances.
     */
    public static class Builder {
        private final WorkflowStep step = new WorkflowStep();

        public Builder id(String id) {
            step.id = id;
            return this;
        }

        public Builder name(String name) {
            step.name = name;
            return this;
        }

        public Builder type(StepType type) {
            step.type = type;
            return this;
        }

        public Builder config(Map<String, Object> config) {
            step.config = config;
            return this;
        }

        public Builder nextSteps(List<String> nextSteps) {
            step.nextSteps = nextSteps;
            return this;
        }

        public Builder onError(String onError) {
            step.onError = onError;
            return this;
        }

        public Builder timeout(Integer timeout) {
            step.timeout = timeout;
            return this;
        }

        public Builder retryCount(Integer retryCount) {
            step.retryCount = retryCount;
            return this;
        }

        public WorkflowStep build() {
            return step;
        }
    }
}
