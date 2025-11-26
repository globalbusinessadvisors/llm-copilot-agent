"""
Exceptions for the LLM CoPilot Agent SDK.
"""

from typing import Any, Optional


class CoPilotError(Exception):
    """Base exception for all CoPilot SDK errors."""

    def __init__(
        self,
        message: str,
        status_code: Optional[int] = None,
        response_data: Optional[dict[str, Any]] = None,
    ):
        super().__init__(message)
        self.message = message
        self.status_code = status_code
        self.response_data = response_data or {}

    def __str__(self) -> str:
        if self.status_code:
            return f"[{self.status_code}] {self.message}"
        return self.message


class AuthenticationError(CoPilotError):
    """Raised when authentication fails."""

    def __init__(
        self,
        message: str = "Authentication failed",
        status_code: int = 401,
        response_data: Optional[dict[str, Any]] = None,
    ):
        super().__init__(message, status_code, response_data)


class AuthorizationError(CoPilotError):
    """Raised when the user is not authorized to perform an action."""

    def __init__(
        self,
        message: str = "Not authorized",
        status_code: int = 403,
        response_data: Optional[dict[str, Any]] = None,
    ):
        super().__init__(message, status_code, response_data)


class NotFoundError(CoPilotError):
    """Raised when a requested resource is not found."""

    def __init__(
        self,
        message: str = "Resource not found",
        status_code: int = 404,
        response_data: Optional[dict[str, Any]] = None,
    ):
        super().__init__(message, status_code, response_data)


class ValidationError(CoPilotError):
    """Raised when request validation fails."""

    def __init__(
        self,
        message: str = "Validation error",
        status_code: int = 422,
        response_data: Optional[dict[str, Any]] = None,
        errors: Optional[list[dict[str, Any]]] = None,
    ):
        super().__init__(message, status_code, response_data)
        self.errors = errors or []

    def __str__(self) -> str:
        base = super().__str__()
        if self.errors:
            error_details = "; ".join(
                f"{e.get('field', 'unknown')}: {e.get('message', 'invalid')}"
                for e in self.errors
            )
            return f"{base} - {error_details}"
        return base


class RateLimitError(CoPilotError):
    """Raised when rate limit is exceeded."""

    def __init__(
        self,
        message: str = "Rate limit exceeded",
        status_code: int = 429,
        response_data: Optional[dict[str, Any]] = None,
        retry_after: Optional[int] = None,
    ):
        super().__init__(message, status_code, response_data)
        self.retry_after = retry_after

    def __str__(self) -> str:
        base = super().__str__()
        if self.retry_after:
            return f"{base} (retry after {self.retry_after}s)"
        return base


class ServerError(CoPilotError):
    """Raised when the server returns a 5xx error."""

    def __init__(
        self,
        message: str = "Server error",
        status_code: int = 500,
        response_data: Optional[dict[str, Any]] = None,
    ):
        super().__init__(message, status_code, response_data)


class ConnectionError(CoPilotError):
    """Raised when unable to connect to the server."""

    def __init__(
        self,
        message: str = "Unable to connect to server",
        response_data: Optional[dict[str, Any]] = None,
    ):
        super().__init__(message, None, response_data)


class TimeoutError(CoPilotError):
    """Raised when a request times out."""

    def __init__(
        self,
        message: str = "Request timed out",
        response_data: Optional[dict[str, Any]] = None,
    ):
        super().__init__(message, None, response_data)


def raise_for_status(status_code: int, response_data: dict[str, Any]) -> None:
    """Raise an appropriate exception based on status code."""
    message = response_data.get("error", response_data.get("message", "Unknown error"))

    if status_code == 401:
        raise AuthenticationError(message, status_code, response_data)
    elif status_code == 403:
        raise AuthorizationError(message, status_code, response_data)
    elif status_code == 404:
        raise NotFoundError(message, status_code, response_data)
    elif status_code == 422:
        errors = response_data.get("errors", [])
        raise ValidationError(message, status_code, response_data, errors)
    elif status_code == 429:
        retry_after = response_data.get("retry_after")
        raise RateLimitError(message, status_code, response_data, retry_after)
    elif 500 <= status_code < 600:
        raise ServerError(message, status_code, response_data)
    elif status_code >= 400:
        raise CoPilotError(message, status_code, response_data)
