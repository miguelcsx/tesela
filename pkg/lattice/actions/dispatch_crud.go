// CRUD handler — dispatches insert/update/delete via the adapter Mutation
// surface. Mappings translate input fields into property values; the
// expression form is currently a literal field reference.

package actions

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/backend"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// CRUDHandler dispatches CRUDCreate/Update/Delete actions.
type CRUDHandler struct{}

// NewCRUDHandler returns the singleton CRUD handler.
func NewCRUDHandler() *CRUDHandler { return &CRUDHandler{} }

// Dispatch implements Handler.
func (h *CRUDHandler) Dispatch(ctx context.Context, ev DispatchEvent) (DispatchResult, error) {
	if ev.Connection == nil {
		return DispatchResult{}, fmt.Errorf("crud: adapter connection is required")
	}
	if ev.ActionType.Subject == "" {
		return DispatchResult{}, fmt.Errorf("crud: action %q has no subject", ev.ActionType.APIName)
	}
	mut, err := h.buildMutation(ev)
	if err != nil {
		return DispatchResult{}, err
	}
	mutator, err := backend.AsMutator(ev.Connection)
	if err != nil {
		return DispatchResult{}, fmt.Errorf("crud: %w", err)
	}
	res, err := mutator.Mutate(ctx, ev.SourceConfig, mut)
	if err != nil {
		return DispatchResult{}, fmt.Errorf("crud: execute: %w", err)
	}
	output, err := json.Marshal(res)
	if err != nil {
		return DispatchResult{}, fmt.Errorf("crud: marshal output: %w", err)
	}
	return DispatchResult{
		Output:       output,
		AffectedRows: res.AffectedRows,
		PrimaryKey:   res.PrimaryKey,
	}, nil
}

func (h *CRUDHandler) buildMutation(ev DispatchEvent) (types.Mutation, error) {
	cfg := ev.ActionType.Handler.CRUD
	if cfg == nil {
		return types.Mutation{}, fmt.Errorf("crud: handler.crud is nil")
	}
	values := make(map[types.APIName]any, len(cfg.Mappings))
	for _, m := range cfg.Mappings {
		v, err := resolveExpression(m.Expression, ev)
		if err != nil {
			return types.Mutation{}, fmt.Errorf("mapping %s: %w", m.TargetProperty, err)
		}
		values[m.TargetProperty] = v
	}
	mut := types.Mutation{Values: values}
	switch ev.ActionType.Handler.Kind {
	case types.HandlerKindCRUDCreate:
		mut.Kind = types.MutationKindInsert
	case types.HandlerKindCRUDUpdate:
		mut.Kind = types.MutationKindUpdate
		mut.PrimaryKey = subjectPrimaryKey(ev)
	case types.HandlerKindCRUDDelete:
		mut.Kind = types.MutationKindDelete
		mut.PrimaryKey = subjectPrimaryKey(ev)
	}
	return mut, nil
}

func subjectPrimaryKey(ev DispatchEvent) any {
	if ev.Subject == nil {
		return nil
	}
	if v, ok := ev.Input["primary_key"]; ok {
		return v
	}
	return nil
}

// resolveExpression supports two forms:
//   - "input.<field>" reads a field from the action input.
//   - any other string is treated as a literal value.
//
// Phase 1: deliberately narrow. CEL evaluation will replace this in Phase 2.5.
func resolveExpression(expr string, ev DispatchEvent) (any, error) {
	if expr == "" {
		return nil, fmt.Errorf("expression is empty")
	}
	if len(expr) > 6 && expr[:6] == "input." {
		key := expr[6:]
		v, ok := ev.Input[key]
		if !ok {
			return nil, fmt.Errorf("input.%s not provided", key)
		}
		return v, nil
	}
	return expr, nil
}
