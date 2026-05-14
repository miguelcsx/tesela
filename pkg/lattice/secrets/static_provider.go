// StaticProvider is an in-memory SecretProvider, useful for tests and for
// bootstrap-time secrets that ship in configuration.

package secrets

import (
	"context"
	"fmt"
)

// StaticProvider is a SecretProvider backed by an immutable map.
type StaticProvider struct {
	values map[string]string
}

// NewStaticProvider returns a provider that resolves references via the
// supplied map. Nil is treated as an empty map.
func NewStaticProvider(values map[string]string) *StaticProvider {
	if values == nil {
		values = map[string]string{}
	}
	// Copy so callers cannot mutate the provider state after construction.
	cp := make(map[string]string, len(values))
	for k, v := range values {
		cp[k] = v
	}
	return &StaticProvider{values: cp}
}

// Name implements SecretProvider.
func (*StaticProvider) Name() string { return "static" }

// Lookup implements SecretProvider.
func (p *StaticProvider) Lookup(_ context.Context, reference string) (string, error) {
	v, ok := p.values[reference]
	if !ok {
		return "", fmt.Errorf("%w: static %q", ErrNotFound, reference)
	}
	return v, nil
}
