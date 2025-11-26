package com.llmcopilot.sdk.exceptions;

import com.llmcopilot.sdk.models.ApiError;

/**
 * Exception thrown when rate limited (429 Too Many Requests).
 */
public class RateLimitException extends CoPilotException {

    private final Long retryAfter;

    public RateLimitException(String message) {
        super(429, message);
        this.retryAfter = null;
    }

    public RateLimitException(ApiError error) {
        super(429, error);
        this.retryAfter = extractRetryAfter(error);
    }

    public RateLimitException(ApiError error, Long retryAfter) {
        super(429, error);
        this.retryAfter = retryAfter;
    }

    /**
     * Returns the number of seconds to wait before retrying.
     */
    public Long getRetryAfter() {
        return retryAfter;
    }

    private Long extractRetryAfter(ApiError error) {
        if (error.getDetails() != null) {
            Object retryValue = error.getDetails().get("retry_after");
            if (retryValue instanceof Number) {
                return ((Number) retryValue).longValue();
            }
        }
        return null;
    }
}
