package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

/**
 * Represents an API error response.
 */
public class ApiError {

    @JsonProperty("code")
    private String code;

    @JsonProperty("message")
    private String message;

    @JsonProperty("details")
    private Map<String, Object> details;

    @JsonProperty("request_id")
    private String requestId;

    // Default constructor for Jackson
    public ApiError() {}

    // Getters
    public String getCode() {
        return code;
    }

    public String getMessage() {
        return message;
    }

    public Map<String, Object> getDetails() {
        return details;
    }

    public String getRequestId() {
        return requestId;
    }

    @Override
    public String toString() {
        return "ApiError{" +
                "code='" + code + '\'' +
                ", message='" + message + '\'' +
                ", requestId='" + requestId + '\'' +
                '}';
    }
}
