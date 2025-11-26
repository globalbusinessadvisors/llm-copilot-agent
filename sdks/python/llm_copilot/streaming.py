"""
Streaming response handling for the LLM CoPilot Agent SDK.
"""

import json
from dataclasses import dataclass
from enum import Enum
from typing import Any, AsyncIterator, Optional


class StreamEventType(str, Enum):
    """Type of streaming event."""

    MESSAGE_START = "message_start"
    CONTENT_DELTA = "content_delta"
    MESSAGE_END = "message_end"
    TOOL_USE = "tool_use"
    TOOL_RESULT = "tool_result"
    ERROR = "error"
    PING = "ping"


@dataclass
class StreamEvent:
    """A single event in a streaming response."""

    event_type: StreamEventType
    data: dict[str, Any]
    message_id: Optional[str] = None

    @property
    def content(self) -> Optional[str]:
        """Get the content from a content_delta event."""
        if self.event_type == StreamEventType.CONTENT_DELTA:
            return self.data.get("delta", {}).get("text", "")
        return None

    @property
    def is_final(self) -> bool:
        """Check if this is the final event."""
        return self.event_type in (StreamEventType.MESSAGE_END, StreamEventType.ERROR)

    @property
    def error(self) -> Optional[str]:
        """Get error message if this is an error event."""
        if self.event_type == StreamEventType.ERROR:
            return self.data.get("error", "Unknown error")
        return None


class StreamingResponse:
    """Handles streaming responses from the API."""

    def __init__(self, response: Any):
        """
        Initialize with an httpx Response object.

        Args:
            response: The httpx Response object with streaming enabled.
        """
        self._response = response
        self._buffer = ""
        self._complete_content = ""
        self._message_id: Optional[str] = None

    async def __aiter__(self) -> AsyncIterator[StreamEvent]:
        """Iterate over streaming events."""
        async for line in self._response.aiter_lines():
            if not line:
                continue

            if line.startswith("data: "):
                data_str = line[6:]
                if data_str == "[DONE]":
                    break

                try:
                    data = json.loads(data_str)
                    event = self._parse_event(data)
                    if event:
                        yield event
                except json.JSONDecodeError:
                    continue

    def _parse_event(self, data: dict[str, Any]) -> Optional[StreamEvent]:
        """Parse a raw event into a StreamEvent."""
        event_type_str = data.get("type", "content_delta")

        try:
            event_type = StreamEventType(event_type_str)
        except ValueError:
            event_type = StreamEventType.CONTENT_DELTA

        message_id = data.get("message_id") or data.get("id")
        if message_id:
            self._message_id = message_id

        # Accumulate content for content_delta events
        if event_type == StreamEventType.CONTENT_DELTA:
            content = data.get("delta", {}).get("text", "")
            self._complete_content += content

        return StreamEvent(
            event_type=event_type,
            data=data,
            message_id=self._message_id,
        )

    async def get_complete_content(self) -> str:
        """
        Consume the entire stream and return the complete content.

        This will exhaust the stream iterator.
        """
        async for event in self:
            pass  # Events are processed in __aiter__
        return self._complete_content

    async def get_final_message(self) -> dict[str, Any]:
        """
        Consume the stream and return the final message.
        """
        final_data: dict[str, Any] = {}

        async for event in self:
            if event.event_type == StreamEventType.MESSAGE_END:
                final_data = event.data
            elif event.event_type == StreamEventType.ERROR:
                raise RuntimeError(event.error)

        final_data["content"] = self._complete_content
        final_data["message_id"] = self._message_id

        return final_data

    @property
    def message_id(self) -> Optional[str]:
        """Get the message ID if available."""
        return self._message_id

    @property
    def accumulated_content(self) -> str:
        """Get the content accumulated so far."""
        return self._complete_content


async def parse_sse_stream(response: Any) -> AsyncIterator[StreamEvent]:
    """
    Parse a Server-Sent Events stream.

    Args:
        response: An httpx Response object with streaming enabled.

    Yields:
        StreamEvent objects parsed from the stream.
    """
    streaming = StreamingResponse(response)
    async for event in streaming:
        yield event
