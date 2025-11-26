package com.llmcopilot.sdk.exceptions;

import com.llmcopilot.sdk.models.ApiError;

/**
 * Exception thrown when a server error occurs (5xx).
 */
public class ServerException extends CoPilotException {

    public ServerException(int statusCode, String message) {
        super(statusCode, message);
    }

    public ServerException(int statusCode, ApiError error) {
        super(statusCode, error);
    }
}
