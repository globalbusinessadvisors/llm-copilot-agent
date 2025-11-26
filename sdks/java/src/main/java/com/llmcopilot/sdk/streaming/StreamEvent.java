package com.llmcopilot.sdk.streaming;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Map;

/**
 * Represents a streaming event.
 */
public class StreamEvent {

    @JsonProperty("type")
    private StreamEventType type;

    @JsonProperty("data")
    private Map<String, Object> data;

    @JsonProperty("message_id")
    private String messageId;

    @JsonProperty("delta")
    private StreamDelta delta;

    @JsonProperty("error")
    private String error;

    // Default constructor for Jackson
    public StreamEvent() {}

    public StreamEventType getType() {
        return type;
    }

    public void setType(StreamEventType type) {
        this.type = type;
    }

    public Map<String, Object> getData() {
        return data;
    }

    public void setData(Map<String, Object> data) {
        this.data = data;
    }

    public String getMessageId() {
        return messageId;
    }

    public void setMessageId(String messageId) {
        this.messageId = messageId;
    }

    public StreamDelta getDelta() {
        return delta;
    }

    public void setDelta(StreamDelta delta) {
        this.delta = delta;
    }

    public String getError() {
        return error;
    }

    public void setError(String error) {
        this.error = error;
    }

    /**
     * Returns the text content if this is a content delta event.
     */
    public String getContent() {
        if (type == StreamEventType.CONTENT_DELTA && delta != null) {
            return delta.getText();
        }
        return "";
    }

    /**
     * Returns true if this is a final event (message end or error).
     */
    public boolean isFinal() {
        return type == StreamEventType.MESSAGE_END || type == StreamEventType.ERROR;
    }

    @Override
    public String toString() {
        return "StreamEvent{" +
                "type=" + type +
                ", messageId='" + messageId + '\'' +
                '}';
    }
}
