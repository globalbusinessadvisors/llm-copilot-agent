package com.llmcopilot.sdk.exceptions;

import com.llmcopilot.sdk.models.ApiError;

/**
 * Exception thrown when a resource is not found (404 Not Found).
 */
public class NotFoundException extends CoPilotException {

    public NotFoundException(String message) {
        super(404, message);
    }

    public NotFoundException(ApiError error) {
        super(404, error);
    }
}
