package com.llmcopilot.sdk.exceptions;

import com.llmcopilot.sdk.models.ApiError;

/**
 * Exception thrown when authentication fails (401 Unauthorized).
 */
public class AuthenticationException extends CoPilotException {

    public AuthenticationException(String message) {
        super(401, message);
    }

    public AuthenticationException(ApiError error) {
        super(401, error);
    }
}
