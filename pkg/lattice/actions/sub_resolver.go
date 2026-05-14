// snapshotResolver implements composite handler's SubActionResolver against a
// running ontology snapshot.

package actions

import (
	"context"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// snapshotResolver finds sub-actions in the live ontology snapshot.
type snapshotResolver struct {
	registry   *ontology.Registry
	dispatcher *Dispatcher
	wsID       types.WorkspaceID
}

// NewSnapshotResolver constructs a SubActionResolver bound to a workspace.
func NewSnapshotResolver(reg *ontology.Registry, d *Dispatcher, ws types.WorkspaceID) SubActionResolver {
	return &snapshotResolver{registry: reg, dispatcher: d, wsID: ws}
}

func (r *snapshotResolver) Resolve(ctx context.Context, name types.APIName) (types.ActionType, error) {
	snap, err := r.registry.Snapshot(ctx, r.wsID)
	if err != nil {
		return types.ActionType{}, err
	}
	at, ok := snap.ActionTypeByName(name)
	if !ok {
		return types.ActionType{}, fmt.Errorf("action %q not found", name)
	}
	return at, nil
}

func (r *snapshotResolver) Dispatcher() *Dispatcher { return r.dispatcher }
