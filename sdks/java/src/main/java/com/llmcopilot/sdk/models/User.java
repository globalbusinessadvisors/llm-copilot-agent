package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.time.Instant;
import java.util.List;
import java.util.Objects;

/**
 * Represents a user.
 */
public class User {

    @JsonProperty("id")
    private String id;

    @JsonProperty("username")
    private String username;

    @JsonProperty("email")
    private String email;

    @JsonProperty("roles")
    private List<String> roles;

    @JsonProperty("tenant_id")
    private String tenantId;

    @JsonProperty("is_active")
    private boolean isActive;

    @JsonProperty("email_verified")
    private boolean emailVerified;

    @JsonProperty("created_at")
    private Instant createdAt;

    @JsonProperty("last_login_at")
    private Instant lastLoginAt;

    // Default constructor for Jackson
    public User() {}

    // Getters
    public String getId() {
        return id;
    }

    public String getUsername() {
        return username;
    }

    public String getEmail() {
        return email;
    }

    public List<String> getRoles() {
        return roles;
    }

    public String getTenantId() {
        return tenantId;
    }

    public boolean isActive() {
        return isActive;
    }

    public boolean isEmailVerified() {
        return emailVerified;
    }

    public Instant getCreatedAt() {
        return createdAt;
    }

    public Instant getLastLoginAt() {
        return lastLoginAt;
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        User user = (User) o;
        return Objects.equals(id, user.id);
    }

    @Override
    public int hashCode() {
        return Objects.hash(id);
    }

    @Override
    public String toString() {
        return "User{" +
                "id='" + id + '\'' +
                ", username='" + username + '\'' +
                ", email='" + email + '\'' +
                '}';
    }
}
