package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

/**
 * Request body for starting a new workflow run.
 */
public class WorkflowRunCreate {

    @JsonProperty("workflow_id")
    private String workflowId;

    @JsonProperty("inputs")
    private Map<String, Object> inputs;

    // Default constructor for Jackson
    public WorkflowRunCreate() {}

    public WorkflowRunCreate(String workflowId) {
        this.workflowId = workflowId;
    }

    public WorkflowRunCreate(String workflowId, Map<String, Object> inputs) {
        this.workflowId = workflowId;
        this.inputs = inputs;
    }

    // Builder pattern
    public static Builder builder() {
        return new Builder();
    }

    // Getters and setters
    public String getWorkflowId() {
        return workflowId;
    }

    public void setWorkflowId(String workflowId) {
        this.workflowId = workflowId;
    }

    public Map<String, Object> getInputs() {
        return inputs;
    }

    public void setInputs(Map<String, Object> inputs) {
        this.inputs = inputs;
    }

    /**
     * Builder for creating WorkflowRunCreate instances.
     */
    public static class Builder {
        private final WorkflowRunCreate request = new WorkflowRunCreate();

        public Builder workflowId(String workflowId) {
            request.workflowId = workflowId;
            return this;
        }

        public Builder inputs(Map<String, Object> inputs) {
            request.inputs = inputs;
            return this;
        }

        public WorkflowRunCreate build() {
            return request;
        }
    }
}
