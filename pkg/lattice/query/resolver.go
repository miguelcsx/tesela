// SnapshotResolver memoizes Evaluators by ontology snapshot pointer. The
// pointer changes on every ontology Apply, so the cache automatically invalidates.

package query

import (
	"sync"

	"github.com/miguelcsx/lattice/pkg/lattice/policy"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// SnapshotResolver caches *policy.Evaluator per *types.Ontology.
type SnapshotResolver struct {
	mu    sync.Mutex
	cache map[*types.Ontology]*policy.Evaluator
}

// NewSnapshotResolver builds an empty resolver.
func NewSnapshotResolver() *SnapshotResolver {
	return &SnapshotResolver{cache: make(map[*types.Ontology]*policy.Evaluator, 4)}
}

// For returns the evaluator for snap, building it on first use.
func (r *SnapshotResolver) For(snap *types.Ontology) (*policy.Evaluator, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if e, ok := r.cache[snap]; ok {
		return e, nil
	}
	e, err := policy.NewEvaluator(snap)
	if err != nil {
		return nil, err
	}
	r.cache[snap] = e
	return e, nil
}
