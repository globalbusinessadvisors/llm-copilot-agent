package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.time.Instant;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * Represents a workflow definition.
 */
public class WorkflowDefinition {

    @JsonProperty("id")
    private String id;

    @JsonProperty("name")
    private String name;

    @JsonProperty("description")
    private String description;

    @JsonProperty("version")
    private String version;

    @JsonProperty("steps")
    private List<WorkflowStep> steps;

    @JsonProperty("entry_point")
    private String entryPoint;

    @JsonProperty("metadata")
    private Map<String, Object> metadata;

    @JsonProperty("created_at")
    private Instant createdAt;

    @JsonProperty("updated_at")
    private Instant updatedAt;

    // Default constructor for Jackson
    public WorkflowDefinition() {}

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

    public String getDescription() {
        return description;
    }

    public String getVersion() {
        return version;
    }

    public List<WorkflowStep> getSteps() {
        return steps;
    }

    public String getEntryPoint() {
        return entryPoint;
    }

    public Map<String, Object> getMetadata() {
        return metadata;
    }

    public Instant getCreatedAt() {
        return createdAt;
    }

    public Instant getUpdatedAt() {
        return updatedAt;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        WorkflowDefinition that = (WorkflowDefinition) o;
        return Objects.equals(id, that.id);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id);
    }

    @Override
    public String toString() {
        return "WorkflowDefinition{" +
                "id='" + id + '\'' +
                ", name='" + name + '\'' +
                ", version='" + version + '\'' +
                '}';
    }

    /**
     * Builder for creating WorkflowDefinition instances.
     */
    public static class Builder {
        private final WorkflowDefinition workflow = new WorkflowDefinition();

        public Builder id(String id) {
            workflow.id = id;
            return this;
        }

        public Builder name(String name) {
            workflow.name = name;
            return this;
        }

        public Builder description(String description) {
            workflow.description = description;
            return this;
        }

        public Builder version(String version) {
            workflow.version = version;
            return this;
        }

        public Builder steps(List<WorkflowStep> steps) {
            workflow.steps = steps;
            return this;
        }

        public Builder entryPoint(String entryPoint) {
            workflow.entryPoint = entryPoint;
            return this;
        }

        public Builder metadata(Map<String, Object> metadata) {
            workflow.metadata = metadata;
            return this;
        }

        public Builder createdAt(Instant createdAt) {
            workflow.createdAt = createdAt;
            return this;
        }

        public Builder updatedAt(Instant updatedAt) {
            workflow.updatedAt = updatedAt;
            return this;
        }

        public WorkflowDefinition build() {
            return workflow;
        }
    }
}
