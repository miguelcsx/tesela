// Dispatcher is the declarative router from HandlerKind to a Handler. Each
// handler dispatches a single action invocation and returns the typed result.

package actions

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/backend"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Handler dispatches a single action invocation. Implementations live in
// dispatch_crud.go, dispatch_webhook.go, dispatch_composite.go.
type Handler interface {
	Dispatch(ctx context.Context, ev DispatchEvent) (DispatchResult, error)
}

// DispatchEvent is the input to a Handler.
type DispatchEvent struct {
	Workspace      types.Workspace
	ActionType     types.ActionType
	SubjectType    *types.ObjectType  // type metadata of the subject (when present)
	Subject        *types.Record      // resolved subject row, when present
	SourceConfig   types.SourceConfig // datasource source config of the subject
	Input          map[string]any
	Actor          types.Actor
	IdempotencyKey string
	Datasource     types.Datasource   // only populated for crud handlers
	Connection     backend.Connection // only populated for crud handlers
}

// DispatchResult is the typed output the pipeline records on success.
type DispatchResult struct {
	Output       json.RawMessage `json:"output,omitempty"`
	AffectedRows int64           `json:"affected_rows,omitempty"`
	PrimaryKey   any             `json:"primary_key,omitempty"`
}

// Dispatcher selects the handler for a given action type.
type Dispatcher struct {
	handlers map[types.HandlerKind]Handler
}

// NewDispatcher composes handlers in a declarative table.
func NewDispatcher(crud, webhook, composite, callback Handler) *Dispatcher {
	return &Dispatcher{handlers: map[types.HandlerKind]Handler{
		types.HandlerKindCRUDCreate: crud,
		types.HandlerKindCRUDUpdate: crud,
		types.HandlerKindCRUDDelete: crud,
		types.HandlerKindWebhook:    webhook,
		types.HandlerKindComposite:  composite,
		types.HandlerKindCallback:   callback,
	}}
}

// Dispatch picks the handler for ev's action and invokes it.
func (d *Dispatcher) Dispatch(ctx context.Context, ev DispatchEvent) (DispatchResult, error) {
	h, ok := d.handlers[ev.ActionType.Handler.Kind]
	if !ok {
		return DispatchResult{}, fmt.Errorf("actions: no handler for kind %q", ev.ActionType.Handler.Kind)
	}
	return h.Dispatch(ctx, ev)
}
