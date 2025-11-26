package com.llmcopilot.sdk.streaming;

import com.fasterxml.jackson.annotation.JsonProperty;

/**
 * Represents a delta (incremental content) in a streaming event.
 */
public class StreamDelta {

    @JsonProperty("type")
    private String type;

    @JsonProperty("text")
    private String text;

    @JsonProperty("index")
    private int index;

    // Default constructor for Jackson
    public StreamDelta() {}

    public String getType() {
        return type;
    }

    public String getText() {
        return text;
    }

    public int getIndex() {
        return index;
    }

    @Override
    public String toString() {
        return "StreamDelta{" +
                "type='" + type + '\'' +
                ", text='" + (text != null && text.length() > 20 ? text.substring(0, 20) + "..." : text) + '\'' +
                '}';
    }
}
