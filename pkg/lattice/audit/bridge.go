package audit

import (
	"context"
	"encoding/json"

	"github.com/miguelcsx/lattice/pkg/lattice/events"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// EventBridgeSink wraps an inner Sink and fans out every audit record as an
// audit.emitted Event on the provided Bus.
type EventBridgeSink struct {
	inner Sink
	bus   events.Bus
}

// NewEventBridge creates a Sink that forwards audit records to both the
// inner Sink and the event bus.
func NewEventBridge(inner Sink, bus events.Bus) Sink {
	return &EventBridgeSink{inner: inner, bus: bus}
}

func (s *EventBridgeSink) Write(ctx context.Context, batch []types.AuditRecord) error {
	if s.bus != nil {
		for i := range batch {
			body, _ := json.Marshal(batch[i])
			_ = s.bus.Publish(ctx, events.Event{
				Kind:        events.KindAuditEmitted,
				WorkspaceID: batch[i].WorkspaceID,
				ObjectType:  batch[i].ResourceAPIName,
				Actor:       batch[i].ActorUserID,
				Body:        body,
			})
		}
	}
	if s.inner == nil {
		return nil
	}
	return s.inner.Write(ctx, batch)
}
