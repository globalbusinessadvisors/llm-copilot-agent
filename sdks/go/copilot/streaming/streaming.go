// Package streaming provides streaming response handling for the LLM CoPilot SDK.
package streaming

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

// EventType represents the type of a streaming event.
type EventType string

const (
	EventMessageStart EventType = "message_start"
	EventContentDelta EventType = "content_delta"
	EventMessageEnd   EventType = "message_end"
	EventToolUse      EventType = "tool_use"
	EventToolResult   EventType = "tool_result"
	EventError        EventType = "error"
	EventPing         EventType = "ping"
)

// Event represents a streaming event.
type Event struct {
	Type      EventType              `json:"type"`
	Data      map[string]interface{} `json:"data,omitempty"`
	MessageID string                 `json:"message_id,omitempty"`
	Delta     *Delta                 `json:"delta,omitempty"`
	Error     string                 `json:"error,omitempty"`
}

// Delta represents the content delta in a streaming event.
type Delta struct {
	Type  string `json:"type,omitempty"`
	Text  string `json:"text,omitempty"`
	Index int    `json:"index,omitempty"`
}

// Content returns the text content from a content delta event.
func (e *Event) Content() string {
	if e.Type == EventContentDelta && e.Delta != nil {
		return e.Delta.Text
	}
	return ""
}

// IsFinal returns true if this is a final event.
func (e *Event) IsFinal() bool {
	return e.Type == EventMessageEnd || e.Type == EventError
}

// Stream represents a streaming response.
type Stream struct {
	response *http.Response
	reader   *bufio.Reader
	events   chan *Event
	err      error
	done     bool
	content  strings.Builder
}

// NewStream creates a new stream from an HTTP response.
func NewStream(resp *http.Response) *Stream {
	s := &Stream{
		response: resp,
		reader:   bufio.NewReader(resp.Body),
		events:   make(chan *Event, 100),
	}
	return s
}

// Events returns a channel for receiving events.
func (s *Stream) Events() <-chan *Event {
	return s.events
}

// Start begins processing the stream in a goroutine.
func (s *Stream) Start(ctx context.Context) {
	go s.process(ctx)
}

// process reads and parses events from the stream.
func (s *Stream) process(ctx context.Context) {
	defer close(s.events)
	defer s.response.Body.Close()

	for {
		select {
		case <-ctx.Done():
			s.err = ctx.Err()
			return
		default:
		}

		line, err := s.reader.ReadString('\n')
		if err != nil {
			if err != io.EOF {
				s.err = err
			}
			return
		}

		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}

		// Parse SSE format
		if strings.HasPrefix(line, "data: ") {
			data := strings.TrimPrefix(line, "data: ")

			// Check for [DONE] marker
			if data == "[DONE]" {
				s.done = true
				return
			}

			event, err := s.parseEvent(data)
			if err != nil {
				continue
			}

			// Accumulate content
			if event.Type == EventContentDelta {
				s.content.WriteString(event.Content())
			}

			select {
			case s.events <- event:
			case <-ctx.Done():
				s.err = ctx.Err()
				return
			}

			if event.IsFinal() {
				s.done = true
				return
			}
		}
	}
}

// parseEvent parses a JSON event from the stream.
func (s *Stream) parseEvent(data string) (*Event, error) {
	var raw map[string]interface{}
	if err := json.Unmarshal([]byte(data), &raw); err != nil {
		return nil, err
	}

	event := &Event{
		Data: raw,
	}

	// Extract type
	if typeVal, ok := raw["type"].(string); ok {
		event.Type = EventType(typeVal)
	} else {
		event.Type = EventContentDelta
	}

	// Extract message ID
	if id, ok := raw["message_id"].(string); ok {
		event.MessageID = id
	} else if id, ok := raw["id"].(string); ok {
		event.MessageID = id
	}

	// Extract delta
	if deltaVal, ok := raw["delta"].(map[string]interface{}); ok {
		event.Delta = &Delta{}
		if text, ok := deltaVal["text"].(string); ok {
			event.Delta.Text = text
		}
		if typ, ok := deltaVal["type"].(string); ok {
			event.Delta.Type = typ
		}
		if idx, ok := deltaVal["index"].(float64); ok {
			event.Delta.Index = int(idx)
		}
	}

	// Extract error
	if errVal, ok := raw["error"].(string); ok {
		event.Error = errVal
	}

	return event, nil
}

// Err returns any error that occurred during streaming.
func (s *Stream) Err() error {
	return s.err
}

// Done returns true if the stream has completed.
func (s *Stream) Done() bool {
	return s.done
}

// AccumulatedContent returns all content received so far.
func (s *Stream) AccumulatedContent() string {
	return s.content.String()
}

// Close closes the stream.
func (s *Stream) Close() error {
	return s.response.Body.Close()
}

// Collect consumes the entire stream and returns all events.
func (s *Stream) Collect(ctx context.Context) ([]*Event, error) {
	var events []*Event

	s.Start(ctx)

	for event := range s.events {
		events = append(events, event)
	}

	if s.err != nil {
		return events, s.err
	}

	return events, nil
}

// CollectContent consumes the stream and returns the complete content.
func (s *Stream) CollectContent(ctx context.Context) (string, error) {
	s.Start(ctx)

	for range s.events {
		// Consume all events
	}

	if s.err != nil {
		return "", s.err
	}

	return s.AccumulatedContent(), nil
}

// StreamCallback is a callback function for stream events.
type StreamCallback func(event *Event) error

// ForEach processes each event with a callback.
func (s *Stream) ForEach(ctx context.Context, callback StreamCallback) error {
	s.Start(ctx)

	for event := range s.events {
		if err := callback(event); err != nil {
			return err
		}
	}

	return s.err
}

// Handler is a convenience type for handling stream events.
type Handler struct {
	OnStart   func(messageID string)
	OnContent func(content string)
	OnEnd     func(messageID string)
	OnError   func(err string)
	OnEvent   func(event *Event)
}

// Handle processes a stream with the configured handlers.
func (h *Handler) Handle(ctx context.Context, stream *Stream) error {
	return stream.ForEach(ctx, func(event *Event) error {
		// Call event-specific handlers
		switch event.Type {
		case EventMessageStart:
			if h.OnStart != nil {
				h.OnStart(event.MessageID)
			}
		case EventContentDelta:
			if h.OnContent != nil {
				h.OnContent(event.Content())
			}
		case EventMessageEnd:
			if h.OnEnd != nil {
				h.OnEnd(event.MessageID)
			}
		case EventError:
			if h.OnError != nil {
				h.OnError(event.Error)
				return fmt.Errorf("stream error: %s", event.Error)
			}
		}

		// Call generic event handler
		if h.OnEvent != nil {
			h.OnEvent(event)
		}

		return nil
	})
}
