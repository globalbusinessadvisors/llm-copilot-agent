package com.llmcopilot.sdk.exceptions;

import com.llmcopilot.sdk.models.ApiError;
import java.util.Map;

/**
 * Base exception for all CoPilot API errors.
 */
public class CoPilotException extends RuntimeException {

    private final int statusCode;
    private final String errorCode;
    private final Map<String, Object> details;
    private final String requestId;

    public CoPilotException(String message) {
        super(message);
        this.statusCode = 0;
        this.errorCode = null;
        this.details = null;
        this.requestId = null;
    }

    public CoPilotException(String message, Throwable cause) {
        super(message, cause);
        this.statusCode = 0;
        this.errorCode = null;
        this.details = null;
        this.requestId = null;
    }

    public CoPilotException(int statusCode, String message) {
        super(message);
        this.statusCode = statusCode;
        this.errorCode = null;
        this.details = null;
        this.requestId = null;
    }

    public CoPilotException(int statusCode, ApiError error) {
        super(error.getMessage());
        this.statusCode = statusCode;
        this.errorCode = error.getCode();
        this.details = error.getDetails();
        this.requestId = error.getRequestId();
    }

    public CoPilotException(int statusCode, String errorCode, String message, Map<String, Object> details, String requestId) {
        super(message);
        this.statusCode = statusCode;
        this.errorCode = errorCode;
        this.details = details;
        this.requestId = requestId;
    }

    public int getStatusCode() {
        return statusCode;
    }

    public String getErrorCode() {
        return errorCode;
    }

    public Map<String, Object> getDetails() {
        return details;
    }

    public String getRequestId() {
        return requestId;
    }

    public boolean isUnauthorized() {
        return statusCode == 401;
    }

    public boolean isForbidden() {
        return statusCode == 403;
    }

    public boolean isNotFound() {
        return statusCode == 404;
    }

    public boolean isRateLimited() {
        return statusCode == 429;
    }

    public boolean isServerError() {
        return statusCode >= 500;
    }

    public boolean isRetryable() {
        return statusCode == 429 || statusCode >= 500;
    }

    @Override
    public String toString() {
        StringBuilder sb = new StringBuilder("CoPilotException{");
        if (statusCode > 0) {
            sb.append("statusCode=").append(statusCode);
        }
        if (errorCode != null) {
            sb.append(", errorCode='").append(errorCode).append('\'');
        }
        sb.append(", message='").append(getMessage()).append('\'');
        if (requestId != null) {
            sb.append(", requestId='").append(requestId).append('\'');
        }
        sb.append('}');
        return sb.toString();
    }
}
