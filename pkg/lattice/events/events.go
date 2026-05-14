// Package events provides a typed event bus for ontology changes,
// audit emissions, action executions and agent run lifecycle. The bus is
// capability-based (any implementation that satisfies Bus is acceptable),
// with an in-memory default. Sinks may be plugged in to fan out to webhooks,
// Kafka, NATS, etc.

package events

import (
	"context"
	"encoding/json"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Kind enumerates event categories. New kinds may be added; consumers
// must ignore unknown kinds.
type Kind string

const (
	KindObjectCreated Kind = "object.created"
	KindObjectUpdated Kind = "object.updated"
	KindObjectDeleted Kind = "object.deleted"
	KindAuditEmitted  Kind = "audit.emitted"
	KindActionStarted Kind = "action.started"
	KindActionEnded   Kind = "action.ended"
	KindAgentStarted  Kind = "agent.started"
	KindAgentEnded    Kind = "agent.ended"
	KindOntologyDiff  Kind = "ontology.diff"
)

// Event is the unit of fan-out. The Body is opaque JSON-serializable
// payload whose schema is determined by Kind.
type Event struct {
	ID          string            `json:"id"`
	Kind        Kind              `json:"kind"`
	OccurredAt  time.Time         `json:"occurred_at"`
	WorkspaceID types.WorkspaceID `json:"workspace_id,omitempty"`
	ObjectType  types.APIName     `json:"object_type,omitempty"`
	PrimaryKey  string            `json:"primary_key,omitempty"`
	Actor       string            `json:"actor,omitempty"`
	Body        json.RawMessage   `json:"body,omitempty"`
}

// Filter narrows a subscription to a subset of events. Zero value matches
// every event.
type Filter struct {
	Kinds       []Kind
	ObjectTypes []types.APIName
}

// Match reports whether e satisfies f.
func (f Filter) Match(e Event) bool {
	if len(f.Kinds) > 0 {
		ok := false
		for _, k := range f.Kinds {
			if k == e.Kind {
				ok = true
				break
			}
		}
		if !ok {
			return false
		}
	}
	if len(f.ObjectTypes) > 0 && e.ObjectType != "" {
		ok := false
		for _, n := range f.ObjectTypes {
			if n == e.ObjectType {
				ok = true
				break
			}
		}
		if !ok {
			return false
		}
	}
	return true
}

// Handler is the consumer signature. It MUST NOT block; callers expecting
// long-running work should hand off to a goroutine internally. The bus
// will not retry on error — wrap your handler with retry/dead-letter logic
// if the contract requires it.
type Handler func(ctx context.Context, e Event) error

// Subscription is an opaque handle returned from Bus.Subscribe; cancel by
// calling Close.
type Subscription interface {
	Close() error
}

// Bus is the central fan-out abstraction. Implementations must be safe for
// concurrent use.
type Bus interface {
	Publish(ctx context.Context, e Event) error
	Subscribe(filter Filter, h Handler) (Subscription, error)
	Close() error
}
