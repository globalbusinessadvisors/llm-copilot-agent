package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.time.Instant;
import java.util.Map;
import java.util.Objects;

/**
 * Represents a workflow run instance.
 */
public class WorkflowRun {

    @JsonProperty("id")
    private String id;

    @JsonProperty("workflow_id")
    private String workflowId;

    @JsonProperty("status")
    private WorkflowStatus status;

    @JsonProperty("current_step")
    private String currentStep;

    @JsonProperty("inputs")
    private Map<String, Object> inputs;

    @JsonProperty("outputs")
    private Map<String, Object> outputs;

    @JsonProperty("error")
    private String error;

    @JsonProperty("started_at")
    private Instant startedAt;

    @JsonProperty("completed_at")
    private Instant completedAt;

    @JsonProperty("created_at")
    private Instant createdAt;

    // Default constructor for Jackson
    public WorkflowRun() {}

    // Getters
    public String getId() {
        return id;
    }

    public String getWorkflowId() {
        return workflowId;
    }

    public WorkflowStatus getStatus() {
        return status;
    }

    public String getCurrentStep() {
        return currentStep;
    }

    public Map<String, Object> getInputs() {
        return inputs;
    }

    public Map<String, Object> getOutputs() {
        return outputs;
    }

    public String getError() {
        return error;
    }

    public Instant getStartedAt() {
        return startedAt;
    }

    public Instant getCompletedAt() {
        return completedAt;
    }

    public Instant getCreatedAt() {
        return createdAt;
    }

    /**
     * Returns true if the workflow run is completed (success or failure).
     */
    public boolean isTerminal() {
        return status == WorkflowStatus.COMPLETED ||
               status == WorkflowStatus.FAILED ||
               status == WorkflowStatus.CANCELLED;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        WorkflowRun that = (WorkflowRun) o;
        return Objects.equals(id, that.id);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id);
    }

    @Override
    public String toString() {
        return "WorkflowRun{" +
                "id='" + id + '\'' +
                ", workflowId='" + workflowId + '\'' +
                ", status=" + status +
                '}';
    }
}
