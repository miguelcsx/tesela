// Cache holds the live schema per workspace, swapping atomically when the
// ontology subscriber emits a Change event.

package graphql

import (
	"context"
	"sync"
	"sync/atomic"

	"github.com/graphql-go/graphql"

	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// SchemaCache is the per-workspace schema holder.
type SchemaCache struct {
	mu       sync.Mutex
	schemas  sync.Map // map[types.WorkspaceID]*atomic.Pointer[graphql.Schema]
	registry *ontology.Registry
	builder  *Builder
}

// NewSchemaCache constructs a SchemaCache.
func NewSchemaCache(reg *ontology.Registry, b *Builder) *SchemaCache {
	return &SchemaCache{registry: reg, builder: b}
}

// For returns the schema for ws, building it on first access and rebuilding
// after a Subscribe event (rebuild is deferred to the watcher goroutine).
func (c *SchemaCache) For(ctx context.Context, ws types.WorkspaceID) (*graphql.Schema, error) {
	if v, ok := c.schemas.Load(ws); ok {
		if s := v.(*atomic.Pointer[graphql.Schema]).Load(); s != nil {
			return s, nil
		}
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	if v, ok := c.schemas.Load(ws); ok {
		if s := v.(*atomic.Pointer[graphql.Schema]).Load(); s != nil {
			return s, nil
		}
	}
	snap, err := c.registry.Snapshot(ctx, ws)
	if err != nil {
		return nil, err
	}
	schema, err := c.builder.Build(snap)
	if err != nil {
		return nil, err
	}
	ptr := &atomic.Pointer[graphql.Schema]{}
	ptr.Store(&schema)
	c.schemas.Store(ws, ptr)
	go c.watch(ws)
	return &schema, nil
}

// watch listens to Subscribe and rebuilds the schema on every Change.
func (c *SchemaCache) watch(ws types.WorkspaceID) {
	ch := c.registry.Cache().Subscribe(ws)
	for range ch {
		ctx := context.Background()
		snap, err := c.registry.Snapshot(ctx, ws)
		if err != nil {
			continue
		}
		schema, err := c.builder.Build(snap)
		if err != nil {
			continue
		}
		v, _ := c.schemas.LoadOrStore(ws, &atomic.Pointer[graphql.Schema]{})
		v.(*atomic.Pointer[graphql.Schema]).Store(&schema)
	}
}
