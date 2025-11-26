"""
Retry handling utilities for the LLM CoPilot Agent SDK.
"""

import asyncio
import random
from dataclasses import dataclass
from functools import wraps
from typing import Any, Callable, Optional, Sequence, Type, TypeVar, Union

from llm_copilot.exceptions import CoPilotError, RateLimitError, ServerError

T = TypeVar("T")


@dataclass
class RetryConfig:
    """Configuration for retry behavior."""

    max_retries: int = 3
    initial_delay: float = 1.0
    max_delay: float = 60.0
    exponential_base: float = 2.0
    jitter: bool = True
    retryable_exceptions: tuple[Type[Exception], ...] = (
        ServerError,
        RateLimitError,
        ConnectionError,
        TimeoutError,
    )
    retryable_status_codes: tuple[int, ...] = (429, 500, 502, 503, 504)

    def should_retry(self, exception: Exception) -> bool:
        """Check if an exception is retryable."""
        if isinstance(exception, self.retryable_exceptions):
            return True
        if isinstance(exception, CoPilotError) and exception.status_code:
            return exception.status_code in self.retryable_status_codes
        return False

    def calculate_delay(self, attempt: int, retry_after: Optional[int] = None) -> float:
        """Calculate delay for the given attempt number."""
        if retry_after is not None and retry_after > 0:
            return float(retry_after)

        delay = min(
            self.initial_delay * (self.exponential_base ** attempt),
            self.max_delay,
        )

        if self.jitter:
            delay = delay * (0.5 + random.random())

        return delay


class RetryState:
    """Tracks the state of retry attempts."""

    def __init__(self, config: RetryConfig):
        self.config = config
        self.attempt = 0
        self.last_exception: Optional[Exception] = None

    @property
    def exhausted(self) -> bool:
        """Check if all retries have been exhausted."""
        return self.attempt >= self.config.max_retries

    def record_failure(self, exception: Exception) -> None:
        """Record a failed attempt."""
        self.attempt += 1
        self.last_exception = exception

    def should_retry(self, exception: Exception) -> bool:
        """Check if we should retry after this exception."""
        if self.exhausted:
            return False
        return self.config.should_retry(exception)

    def get_delay(self) -> float:
        """Get the delay before the next retry."""
        retry_after = None
        if isinstance(self.last_exception, RateLimitError):
            retry_after = self.last_exception.retry_after
        return self.config.calculate_delay(self.attempt, retry_after)


async def retry_async(
    func: Callable[..., Any],
    *args: Any,
    config: Optional[RetryConfig] = None,
    on_retry: Optional[Callable[[int, Exception, float], None]] = None,
    **kwargs: Any,
) -> Any:
    """
    Execute an async function with retry logic.

    Args:
        func: The async function to execute.
        *args: Arguments to pass to the function.
        config: Retry configuration.
        on_retry: Optional callback called before each retry with (attempt, exception, delay).
        **kwargs: Keyword arguments to pass to the function.

    Returns:
        The result of the function.

    Raises:
        The last exception if all retries are exhausted.
    """
    config = config or RetryConfig()
    state = RetryState(config)

    while True:
        try:
            return await func(*args, **kwargs)
        except Exception as e:
            if not state.should_retry(e):
                raise

            state.record_failure(e)
            delay = state.get_delay()

            if on_retry:
                on_retry(state.attempt, e, delay)

            await asyncio.sleep(delay)


def retry_sync(
    func: Callable[..., T],
    *args: Any,
    config: Optional[RetryConfig] = None,
    on_retry: Optional[Callable[[int, Exception, float], None]] = None,
    **kwargs: Any,
) -> T:
    """
    Execute a sync function with retry logic.

    Args:
        func: The function to execute.
        *args: Arguments to pass to the function.
        config: Retry configuration.
        on_retry: Optional callback called before each retry.
        **kwargs: Keyword arguments to pass to the function.

    Returns:
        The result of the function.

    Raises:
        The last exception if all retries are exhausted.
    """
    import time

    config = config or RetryConfig()
    state = RetryState(config)

    while True:
        try:
            return func(*args, **kwargs)
        except Exception as e:
            if not state.should_retry(e):
                raise

            state.record_failure(e)
            delay = state.get_delay()

            if on_retry:
                on_retry(state.attempt, e, delay)

            time.sleep(delay)


def with_retry(
    config: Optional[RetryConfig] = None,
    on_retry: Optional[Callable[[int, Exception, float], None]] = None,
) -> Callable[[Callable[..., T]], Callable[..., T]]:
    """
    Decorator to add retry logic to async functions.

    Args:
        config: Retry configuration.
        on_retry: Optional callback called before each retry.

    Returns:
        Decorated function with retry logic.
    """
    config = config or RetryConfig()

    def decorator(func: Callable[..., T]) -> Callable[..., T]:
        @wraps(func)
        async def async_wrapper(*args: Any, **kwargs: Any) -> T:
            return await retry_async(func, *args, config=config, on_retry=on_retry, **kwargs)

        @wraps(func)
        def sync_wrapper(*args: Any, **kwargs: Any) -> T:
            return retry_sync(func, *args, config=config, on_retry=on_retry, **kwargs)

        if asyncio.iscoroutinefunction(func):
            return async_wrapper  # type: ignore
        return sync_wrapper  # type: ignore

    return decorator


class RetryableClient:
    """Mixin class that adds retry capabilities to a client."""

    def __init__(self, retry_config: Optional[RetryConfig] = None):
        self._retry_config = retry_config or RetryConfig()
        self._on_retry: Optional[Callable[[int, Exception, float], None]] = None

    def set_retry_config(self, config: RetryConfig) -> None:
        """Set the retry configuration."""
        self._retry_config = config

    def set_on_retry_callback(
        self, callback: Callable[[int, Exception, float], None]
    ) -> None:
        """Set a callback to be called before each retry."""
        self._on_retry = callback

    async def _with_retry_async(
        self, func: Callable[..., Any], *args: Any, **kwargs: Any
    ) -> Any:
        """Execute an async function with retry logic."""
        return await retry_async(
            func,
            *args,
            config=self._retry_config,
            on_retry=self._on_retry,
            **kwargs,
        )

    def _with_retry_sync(
        self, func: Callable[..., T], *args: Any, **kwargs: Any
    ) -> T:
        """Execute a sync function with retry logic."""
        return retry_sync(
            func,
            *args,
            config=self._retry_config,
            on_retry=self._on_retry,
            **kwargs,
        )
