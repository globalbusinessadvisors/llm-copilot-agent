package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;
import java.util.Map;

/**
 * Request body for creating a new workflow definition.
 */
public class WorkflowDefinitionCreate {

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

    // Default constructor for Jackson
    public WorkflowDefinitionCreate() {}

    // Builder pattern
    public static Builder builder() {
        return new Builder();
    }

    // Getters and setters
    public String getName() {
        return name;
    }

    public void setName(String name) {
        this.name = name;
    }

    public String getDescription() {
        return description;
    }

    public void setDescription(String description) {
        this.description = description;
    }

    public String getVersion() {
        return version;
    }

    public void setVersion(String version) {
        this.version = version;
    }

    public List<WorkflowStep> getSteps() {
        return steps;
    }

    public void setSteps(List<WorkflowStep> steps) {
        this.steps = steps;
    }

    public String getEntryPoint() {
        return entryPoint;
    }

    public void setEntryPoint(String entryPoint) {
        this.entryPoint = entryPoint;
    }

    public Map<String, Object> getMetadata() {
        return metadata;
    }

    public void setMetadata(Map<String, Object> metadata) {
        this.metadata = metadata;
    }

    /**
     * Builder for creating WorkflowDefinitionCreate instances.
     */
    public static class Builder {
        private final WorkflowDefinitionCreate request = new WorkflowDefinitionCreate();

        public Builder name(String name) {
            request.name = name;
            return this;
        }

        public Builder description(String description) {
            request.description = description;
            return this;
        }

        public Builder version(String version) {
            request.version = version;
            return this;
        }

        public Builder steps(List<WorkflowStep> steps) {
            request.steps = steps;
            return this;
        }

        public Builder entryPoint(String entryPoint) {
            request.entryPoint = entryPoint;
            return this;
        }

        public Builder metadata(Map<String, Object> metadata) {
            request.metadata = metadata;
            return this;
        }

        public WorkflowDefinitionCreate build() {
            return request;
        }
    }
}
