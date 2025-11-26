package com.llmcopilot.sdk.streaming;

/**
 * Handler interface for processing streaming events.
 */
public interface StreamHandler {

    /**
     * Called when a message stream starts.
     *
     * @param messageId the ID of the message
     */
    default void onStart(String messageId) {}

    /**
     * Called when content is received.
     *
     * @param content the incremental content
     */
    default void onContent(String content) {}

    /**
     * Called when a message stream ends.
     *
     * @param messageId the ID of the message
     */
    default void onEnd(String messageId) {}

    /**
     * Called when an error occurs.
     *
     * @param error the error message
     */
    default void onError(String error) {}

    /**
     * Called for every event received.
     *
     * @param event the stream event
     */
    default void onEvent(StreamEvent event) {}
}
