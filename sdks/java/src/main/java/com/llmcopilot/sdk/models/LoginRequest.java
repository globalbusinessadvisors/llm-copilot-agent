package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Request body for user login.
 */
public class LoginRequest {

    @JsonProperty("username_or_email")
    private String usernameOrEmail;

    @JsonProperty("password")
    private String password;

    // Default constructor for Jackson
    public LoginRequest() {}

    public LoginRequest(String usernameOrEmail, String password) {
        this.usernameOrEmail = usernameOrEmail;
        this.password = password;
    }

    // Getters and setters
    public String getUsernameOrEmail() {
        return usernameOrEmail;
    }

    public void setUsernameOrEmail(String usernameOrEmail) {
        this.usernameOrEmail = usernameOrEmail;
    }

    public String getPassword() {
        return password;
    }

    public void setPassword(String password) {
        this.password = password;
    }
}
