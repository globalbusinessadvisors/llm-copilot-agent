package com.llmcopilot.sdk.models;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.List;

/**
 * Generic paginated response wrapper.
 *
 * @param <T> the type of items in the response
 */
public class PaginatedResponse<T> {

    @JsonProperty("items")
    private List<T> items;

    @JsonProperty("total")
    private int total;

    @JsonProperty("limit")
    private int limit;

    @JsonProperty("offset")
    private int offset;

    @JsonProperty("has_more")
    private boolean hasMore;

    // Default constructor for Jackson
    public PaginatedResponse() {}

    // Getters
    public List<T> getItems() {
        return items;
    }

    public int getTotal() {
        return total;
    }

    public int getLimit() {
        return limit;
    }

    public int getOffset() {
        return offset;
    }

    public boolean hasMore() {
        return hasMore;
    }

    /**
     * Returns true if there are items in the response.
     */
    public boolean isEmpty() {
        return items == null || items.isEmpty();
    }

    /**
     * Returns the number of items in this page.
     */
    public int size() {
        return items != null ? items.size() : 0;
    }

    @Override
    public String toString() {
        return "PaginatedResponse{" +
                "items=" + (items != null ? items.size() : 0) + " items" +
                ", total=" + total +
                ", offset=" + offset +
                ", hasMore=" + hasMore +
                '}';
    }
}
