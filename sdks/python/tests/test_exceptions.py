"""Tests for SDK exceptions."""

import pytest
from llm_copilot.exceptions import (
    CoPilotError,
    AuthenticationError,
    AuthorizationError,
    NotFoundError,
    ValidationError,
    RateLimitError,
    ServerError,
    raise_for_status,
)


def test_copilot_error():
    """Test base CoPilotError."""
    error = CoPilotError("Something went wrong", status_code=400)

    assert str(error) == "[400] Something went wrong"
    assert error.message == "Something went wrong"
    assert error.status_code == 400


def test_authentication_error():
    """Test AuthenticationError."""
    error = AuthenticationError()

    assert error.status_code == 401
    assert "Authentication failed" in str(error)


def test_authorization_error():
    """Test AuthorizationError."""
    error = AuthorizationError("Insufficient permissions")

    assert error.status_code == 403
    assert "Insufficient permissions" in str(error)


def test_not_found_error():
    """Test NotFoundError."""
    error = NotFoundError("User not found")

    assert error.status_code == 404
    assert "User not found" in str(error)


def test_validation_error():
    """Test ValidationError with field errors."""
    errors = [
        {"field": "email", "message": "Invalid email format"},
        {"field": "password", "message": "Too short"},
    ]
    error = ValidationError("Validation failed", errors=errors)

    assert error.status_code == 422
    assert len(error.errors) == 2
    assert "email: Invalid email format" in str(error)


def test_rate_limit_error():
    """Test RateLimitError with retry_after."""
    error = RateLimitError(retry_after=30)

    assert error.status_code == 429
    assert error.retry_after == 30
    assert "retry after 30s" in str(error)


def test_server_error():
    """Test ServerError."""
    error = ServerError("Internal server error")

    assert error.status_code == 500


def test_raise_for_status_401():
    """Test raise_for_status for 401."""
    with pytest.raises(AuthenticationError):
        raise_for_status(401, {"error": "Invalid token"})


def test_raise_for_status_403():
    """Test raise_for_status for 403."""
    with pytest.raises(AuthorizationError):
        raise_for_status(403, {"error": "Forbidden"})


def test_raise_for_status_404():
    """Test raise_for_status for 404."""
    with pytest.raises(NotFoundError):
        raise_for_status(404, {"error": "Not found"})


def test_raise_for_status_422():
    """Test raise_for_status for 422."""
    with pytest.raises(ValidationError):
        raise_for_status(422, {"error": "Invalid", "errors": []})


def test_raise_for_status_429():
    """Test raise_for_status for 429."""
    with pytest.raises(RateLimitError):
        raise_for_status(429, {"error": "Rate limited", "retry_after": 60})


def test_raise_for_status_500():
    """Test raise_for_status for 500."""
    with pytest.raises(ServerError):
        raise_for_status(500, {"error": "Server error"})
