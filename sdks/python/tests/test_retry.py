"""
Tests for retry handling.
"""

import pytest
import asyncio
from unittest.mock import MagicMock, AsyncMock

from llm_copilot.retry import (
    RetryConfig,
    RetryState,
    retry_async,
    retry_sync,
    with_retry,
)
from llm_copilot.exceptions import (
    ServerError,
    RateLimitError,
    AuthenticationError,
    CoPilotError,
)


class TestRetryConfig:
    """Tests for RetryConfig."""

    def test_default_config(self):
        """Test default configuration values."""
        config = RetryConfig()
        assert config.max_retries == 3
        assert config.initial_delay == 1.0
        assert config.max_delay == 60.0
        assert config.exponential_base == 2.0
        assert config.jitter is True

    def test_should_retry_server_error(self):
        """Test server errors are retryable."""
        config = RetryConfig()
        assert config.should_retry(ServerError("error"))

    def test_should_retry_rate_limit(self):
        """Test rate limit errors are retryable."""
        config = RetryConfig()
        assert config.should_retry(RateLimitError("rate limited"))

    def test_should_not_retry_auth_error(self):
        """Test auth errors are not retryable."""
        config = RetryConfig()
        assert not config.should_retry(AuthenticationError("invalid"))

    def test_should_retry_by_status_code(self):
        """Test retrying based on status code."""
        config = RetryConfig()
        error = CoPilotError("error", status_code=503)
        assert config.should_retry(error)

        error = CoPilotError("error", status_code=400)
        assert not config.should_retry(error)

    def test_calculate_delay_exponential(self):
        """Test exponential backoff calculation."""
        config = RetryConfig(
            initial_delay=1.0,
            exponential_base=2.0,
            jitter=False,
        )
        assert config.calculate_delay(0) == 1.0
        assert config.calculate_delay(1) == 2.0
        assert config.calculate_delay(2) == 4.0
        assert config.calculate_delay(3) == 8.0

    def test_calculate_delay_max_cap(self):
        """Test delay is capped at max_delay."""
        config = RetryConfig(
            initial_delay=1.0,
            max_delay=5.0,
            exponential_base=2.0,
            jitter=False,
        )
        assert config.calculate_delay(10) == 5.0

    def test_calculate_delay_respects_retry_after(self):
        """Test retry_after header is respected."""
        config = RetryConfig(initial_delay=1.0, jitter=False)
        assert config.calculate_delay(0, retry_after=30) == 30.0


class TestRetryState:
    """Tests for RetryState."""

    def test_initial_state(self):
        """Test initial retry state."""
        config = RetryConfig(max_retries=3)
        state = RetryState(config)
        assert state.attempt == 0
        assert state.last_exception is None
        assert not state.exhausted

    def test_record_failure(self):
        """Test recording failures."""
        config = RetryConfig(max_retries=3)
        state = RetryState(config)
        error = ServerError("error")

        state.record_failure(error)
        assert state.attempt == 1
        assert state.last_exception is error
        assert not state.exhausted

    def test_exhausted_after_max_retries(self):
        """Test state is exhausted after max retries."""
        config = RetryConfig(max_retries=2)
        state = RetryState(config)

        state.record_failure(ServerError("1"))
        assert not state.exhausted

        state.record_failure(ServerError("2"))
        assert state.exhausted


class TestRetryAsync:
    """Tests for async retry logic."""

    @pytest.mark.asyncio
    async def test_success_no_retry(self):
        """Test successful call without retry."""
        async def success():
            return "result"

        result = await retry_async(success)
        assert result == "result"

    @pytest.mark.asyncio
    async def test_retry_on_server_error(self):
        """Test retry on server error."""
        call_count = 0

        async def fail_then_succeed():
            nonlocal call_count
            call_count += 1
            if call_count < 3:
                raise ServerError("error")
            return "result"

        config = RetryConfig(
            max_retries=5,
            initial_delay=0.01,
            jitter=False,
        )
        result = await retry_async(fail_then_succeed, config=config)
        assert result == "result"
        assert call_count == 3

    @pytest.mark.asyncio
    async def test_no_retry_on_auth_error(self):
        """Test no retry on authentication error."""
        call_count = 0

        async def auth_fail():
            nonlocal call_count
            call_count += 1
            raise AuthenticationError("invalid")

        config = RetryConfig(max_retries=3)
        with pytest.raises(AuthenticationError):
            await retry_async(auth_fail, config=config)
        assert call_count == 1

    @pytest.mark.asyncio
    async def test_exhausted_retries(self):
        """Test exception raised when retries exhausted."""
        call_count = 0

        async def always_fail():
            nonlocal call_count
            call_count += 1
            raise ServerError("error")

        config = RetryConfig(
            max_retries=3,
            initial_delay=0.01,
            jitter=False,
        )
        with pytest.raises(ServerError):
            await retry_async(always_fail, config=config)
        assert call_count == 4  # Initial + 3 retries

    @pytest.mark.asyncio
    async def test_on_retry_callback(self):
        """Test on_retry callback is called."""
        retry_calls = []

        def on_retry(attempt, error, delay):
            retry_calls.append((attempt, type(error).__name__, delay))

        call_count = 0

        async def fail_twice():
            nonlocal call_count
            call_count += 1
            if call_count < 3:
                raise ServerError("error")
            return "result"

        config = RetryConfig(
            max_retries=5,
            initial_delay=0.01,
            jitter=False,
        )
        await retry_async(fail_twice, config=config, on_retry=on_retry)

        assert len(retry_calls) == 2
        assert retry_calls[0][1] == "ServerError"
        assert retry_calls[1][1] == "ServerError"


class TestRetrySync:
    """Tests for sync retry logic."""

    def test_success_no_retry(self):
        """Test successful call without retry."""
        def success():
            return "result"

        result = retry_sync(success)
        assert result == "result"

    def test_retry_on_error(self):
        """Test retry on retryable error."""
        call_count = 0

        def fail_then_succeed():
            nonlocal call_count
            call_count += 1
            if call_count < 2:
                raise ServerError("error")
            return "result"

        config = RetryConfig(
            max_retries=3,
            initial_delay=0.01,
            jitter=False,
        )
        result = retry_sync(fail_then_succeed, config=config)
        assert result == "result"
        assert call_count == 2


class TestWithRetryDecorator:
    """Tests for with_retry decorator."""

    @pytest.mark.asyncio
    async def test_decorator_on_async_function(self):
        """Test decorator works on async functions."""
        call_count = 0

        @with_retry(config=RetryConfig(max_retries=3, initial_delay=0.01, jitter=False))
        async def decorated_async():
            nonlocal call_count
            call_count += 1
            if call_count < 2:
                raise ServerError("error")
            return "result"

        result = await decorated_async()
        assert result == "result"
        assert call_count == 2

    def test_decorator_on_sync_function(self):
        """Test decorator works on sync functions."""
        call_count = 0

        @with_retry(config=RetryConfig(max_retries=3, initial_delay=0.01, jitter=False))
        def decorated_sync():
            nonlocal call_count
            call_count += 1
            if call_count < 2:
                raise ServerError("error")
            return "result"

        result = decorated_sync()
        assert result == "result"
        assert call_count == 2
