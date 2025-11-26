package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

/**
 * Represents the health status of the API.
 */
public class HealthStatus {

    @JsonProperty("status")
    private String status;

    @JsonProperty("version")
    private String version;

    @JsonProperty("uptime_seconds")
    private long uptimeSeconds;

    @JsonProperty("components")
    private Map<String, String> components;

    // Default constructor for Jackson
    public HealthStatus() {}

    // Getters
    public String getStatus() {
        return status;
    }

    public String getVersion() {
        return version;
    }

    public long getUptimeSeconds() {
        return uptimeSeconds;
    }

    public Map<String, String> getComponents() {
        return components;
    }

    /**
     * Returns true if the status is "healthy".
     */
    public boolean isHealthy() {
        return "healthy".equalsIgnoreCase(status);
    }

    @Override
    public String toString() {
        return "HealthStatus{" +
                "status='" + status + '\'' +
                ", version='" + version + '\'' +
                ", uptimeSeconds=" + uptimeSeconds +
                '}';
    }
}
