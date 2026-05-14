package backend

import (
	"context"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/events"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// InlineBackend wraps inline closures into a Backend + Connection.
type InlineBackend struct {
	typeName string
	search   SearchFunc
	get      GetFunc
	mutate   MutateFunc
	publish  func(ctx context.Context, kind events.Kind, mut types.Mutation, res types.MutationResult)
}

// NewInlineBackend constructs a Backend from user-supplied closures.
// nil closures are acceptable; the corresponding capability will return a
// CapabilityError at runtime.
func NewInlineBackend(name string, search SearchFunc, get GetFunc, mutate MutateFunc, publish func(ctx context.Context, kind events.Kind, mut types.Mutation, res types.MutationResult)) Backend {
	return &InlineBackend{typeName: name, search: search, get: get, mutate: mutate, publish: publish}
}

// SetPublisher configures the post-mutation event publisher. Safe to call
// after construction before the backend is registered.
func (b *InlineBackend) SetPublisher(fn func(ctx context.Context, kind events.Kind, mut types.Mutation, res types.MutationResult)) {
	b.publish = fn
}

// Type implements backend.Backend.
func (b *InlineBackend) Type() string { return "inline:" + b.typeName }

// Connect returns a Connection that satisfies whichever capability
// interfaces the user populated.
func (b *InlineBackend) Connect(_ context.Context, _ types.ConfigMap) (Connection, error) {
	return &inlineConn{parent: b}, nil
}

type inlineConn struct{ parent *InlineBackend }

func (c *inlineConn) Ping(_ context.Context) error  { return nil }
func (c *inlineConn) Close(_ context.Context) error { return nil }
func (c *inlineConn) Name() string                 { return "inline:" + c.parent.typeName }

// Search is exposed only when the parent has a search closure.
func (c *inlineConn) Search(ctx context.Context, _ types.SourceConfig, _ types.ObjectType, q types.QuerySpec, extra types.Filter) (types.Page, error) {
	if c.parent.search == nil {
		return types.Page{}, &CapabilityError{Backend: c.Name(), Capability: "Searcher"}
	}
	q.Filter = types.AndFilters(q.Filter, extra)
	page, err := c.parent.search(ctx, q)
	if err != nil {
		return types.Page{}, fmt.Errorf("inline search: %w", err)
	}
	return page, nil
}

// Get is the Getter implementation, parallel to Search.
func (c *inlineConn) Get(ctx context.Context, _ types.SourceConfig, _ types.ObjectType, pk any, _ types.Filter) (types.Record, error) {
	if c.parent.get == nil {
		return types.Record{}, &CapabilityError{Backend: c.Name(), Capability: "Getter"}
	}
	rec, err := c.parent.get(ctx, pk)
	if err != nil {
		return types.Record{}, fmt.Errorf("inline get: %w", err)
	}
	return rec, nil
}

// Mutate is the Mutator implementation.
func (c *inlineConn) Mutate(ctx context.Context, _ types.SourceConfig, mut types.Mutation) (types.MutationResult, error) {
	if c.parent.mutate == nil {
		return types.MutationResult{}, &CapabilityError{Backend: c.Name(), Capability: "Mutator"}
	}
	res, err := c.parent.mutate(ctx, mut)
	if err != nil {
		return types.MutationResult{}, fmt.Errorf("inline mutate: %w", err)
	}
	if c.parent.publish != nil {
		c.parent.publish(ctx, mutationKindToEvent(mut.Kind), mut, res)
	}
	return res, nil
}

func mutationKindToEvent(k types.MutationKind) events.Kind {
	switch k {
	case types.MutationKindInsert, types.MutationKindUpsert:
		return events.KindObjectCreated
	case types.MutationKindUpdate:
		return events.KindObjectUpdated
	case types.MutationKindDelete:
		return events.KindObjectDeleted
	default:
		return events.KindObjectUpdated
	}
}
