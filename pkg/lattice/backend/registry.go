// Registry holds the set of registered Backend factories and the cached
// Connections opened against each Datasource. Connections are keyed by
// (workspace_id, datasource_api_name) and are evicted explicitly when the
// datasource changes.

package backend

import (
	"context"
	"errors"
	"fmt"
	"sync"

	"github.com/miguelcsx/lattice/pkg/lattice/crypto"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// ErrNotFound is returned by Get/Search when the requested row
// or page does not exist. Adapters should map their native "no rows" errors
// to this sentinel.
var ErrNotFound = errors.New("adapter: not found")

// Registry is the in-memory composition of registered adapter drivers and
// open connections. All methods are safe for concurrent use.
type Registry struct {
	mu      sync.RWMutex
	drivers map[string]Backend
	open    map[connKey]Connection
	sealer  crypto.Sealer
}

type connKey struct {
	workspaceID types.WorkspaceID
	apiName     types.APIName
}

// NewRegistry constructs a Registry. Sealer is used to decrypt sealed
// credentials before invoking Connect. Sealer may be nil when no datasource
// has sealed credentials.
func NewRegistry(sealer crypto.Sealer) *Registry {
	return &Registry{
		drivers: make(map[string]Backend),
		open:    make(map[connKey]Connection),
		sealer:  sealer,
	}
}

// Register adds a driver. Subsequent calls with the same Type override the
// previous registration.
func (r *Registry) Register(d Backend) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.drivers[d.Type()] = d
}

// Driver returns the registered driver for adapter type t.
func (r *Registry) Driver(t string) (Backend, bool) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	d, ok := r.drivers[t]
	return d, ok
}

// Drivers returns the list of registered adapter types (sorted is the
// caller's responsibility).
func (r *Registry) Drivers() []string {
	r.mu.RLock()
	defer r.mu.RUnlock()
	out := make([]string, 0, len(r.drivers))
	for t := range r.drivers {
		out = append(out, t)
	}
	return out
}

// Acquire returns a live Connection for ds. Connections are cached; repeated
// calls for the same datasource return the same handle until Evict is called
// or the datasource is updated through ApplyDatasource.
func (r *Registry) Acquire(ctx context.Context, ds types.Datasource) (Connection, error) {
	k := connKey{ds.WorkspaceID, ds.APIName}
	r.mu.RLock()
	if c, ok := r.open[k]; ok {
		r.mu.RUnlock()
		return c, nil
	}
	r.mu.RUnlock()

	r.mu.Lock()
	defer r.mu.Unlock()
	if c, ok := r.open[k]; ok {
		return c, nil
	}

	driver, ok := r.drivers[ds.AdapterType]
	if !ok {
		return nil, fmt.Errorf("adapter %q not registered", ds.AdapterType)
	}
	cfg, err := r.resolveConfig(ds)
	if err != nil {
		return nil, fmt.Errorf("resolve config for %q: %w", ds.APIName, err)
	}
	c, err := driver.Connect(ctx, cfg)
	if err != nil {
		return nil, fmt.Errorf("connect %q: %w", ds.APIName, err)
	}
	r.open[k] = c
	return c, nil
}

// Evict closes and removes the cached connection for the given datasource.
// Safe to call when no connection is open.
func (r *Registry) Evict(ctx context.Context, ws types.WorkspaceID, name types.APIName) error {
	r.mu.Lock()
	c, ok := r.open[connKey{ws, name}]
	if !ok {
		r.mu.Unlock()
		return nil
	}
	delete(r.open, connKey{ws, name})
	r.mu.Unlock()
	return c.Close(ctx)
}

// CloseAll closes every cached connection. Returns the first error.
func (r *Registry) CloseAll(ctx context.Context) error {
	r.mu.Lock()
	conns := r.open
	r.open = make(map[connKey]Connection)
	r.mu.Unlock()
	var first error
	for _, c := range conns {
		if err := c.Close(ctx); err != nil && first == nil {
			first = err
		}
	}
	return first
}

// resolveConfig produces the adapter ConfigMap by merging ds.Config with the
// decrypted credentials blob, when present.
func (r *Registry) resolveConfig(ds types.Datasource) (types.ConfigMap, error) {
	cfg := make(types.ConfigMap, len(ds.Config)+4)
	for k, v := range ds.Config {
		cfg[k] = v
	}
	if len(ds.SealedCredentials) == 0 {
		return cfg, nil
	}
	if r.sealer == nil {
		return nil, errors.New("sealed credentials present but no Sealer configured")
	}
	plaintext, err := r.sealer.Open(ds.SealedCredentials)
	if err != nil {
		return nil, fmt.Errorf("unseal credentials: %w", err)
	}
	creds, err := decodeCredentials(plaintext)
	if err != nil {
		return nil, fmt.Errorf("decode credentials: %w", err)
	}
	for k, v := range creds {
		cfg[k] = v
	}
	return cfg, nil
}
