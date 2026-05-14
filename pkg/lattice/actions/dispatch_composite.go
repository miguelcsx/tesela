// Composite handler — runs a sequence of sub-actions, propagating outputs to
// later steps via a simple stepX.<field> reference.

package actions

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// CompositeHandler dispatches composite actions.
type CompositeHandler struct {
	resolver SubActionResolver
}

// SubActionResolver finds an ActionType + Handler pair for a referenced step.
// The pipeline implements this by looking up actions in the live ontology
// snapshot.
type SubActionResolver interface {
	Resolve(ctx context.Context, actionRef types.APIName) (types.ActionType, error)
	Dispatcher() *Dispatcher
}

// NewCompositeHandler returns a composite handler.
func NewCompositeHandler(r SubActionResolver) *CompositeHandler {
	return &CompositeHandler{resolver: r}
}

// Dispatch implements Handler.
func (h *CompositeHandler) Dispatch(ctx context.Context, ev DispatchEvent) (DispatchResult, error) {
	cfg := ev.ActionType.Handler.Composite
	if cfg == nil {
		return DispatchResult{}, fmt.Errorf("composite: handler.composite is nil")
	}
	results := make(map[string]any, len(cfg.Steps))
	for _, step := range cfg.Steps {
		stepInput := make(map[string]any, len(step.InputExpr))
		for k, expr := range step.InputExpr {
			v, err := evaluateCompositeExpression(expr, ev.Input, results)
			if err != nil {
				return DispatchResult{}, fmt.Errorf("step %s: %w", step.Name, err)
			}
			stepInput[k] = v
		}
		sub, err := h.resolver.Resolve(ctx, step.ActionRef)
		if err != nil {
			if step.OnFailure == types.CompositeOnFailureSkip {
				continue
			}
			return DispatchResult{}, fmt.Errorf("step %s: resolve: %w", step.Name, err)
		}
		subEv := ev
		subEv.ActionType = sub
		subEv.Input = stepInput
		res, err := h.resolver.Dispatcher().Dispatch(ctx, subEv)
		if err != nil {
			if step.OnFailure == types.CompositeOnFailureSkip {
				continue
			}
			return DispatchResult{}, fmt.Errorf("step %s: %w", step.Name, err)
		}
		results[step.Name] = decodeStepOutput(res)
	}
	out, err := json.Marshal(results)
	if err != nil {
		return DispatchResult{}, fmt.Errorf("composite: marshal: %w", err)
	}
	return DispatchResult{Output: out}, nil
}

// evaluateCompositeExpression supports forms input.<field>, steps.<name>.<field>,
// or a literal string.
func evaluateCompositeExpression(expr string, input, results map[string]any) (any, error) {
	switch {
	case strings.HasPrefix(expr, "input."):
		v, ok := input[expr[6:]]
		if !ok {
			return nil, fmt.Errorf("input.%s not provided", expr[6:])
		}
		return v, nil
	case strings.HasPrefix(expr, "steps."):
		rest := expr[6:]
		dot := strings.IndexByte(rest, '.')
		if dot < 0 {
			return nil, fmt.Errorf("invalid steps reference %q", expr)
		}
		stepName, field := rest[:dot], rest[dot+1:]
		stepRes, ok := results[stepName].(map[string]any)
		if !ok {
			return nil, fmt.Errorf("step %q has no output", stepName)
		}
		v, ok := stepRes[field]
		if !ok {
			return nil, fmt.Errorf("step %s.%s missing", stepName, field)
		}
		return v, nil
	default:
		return expr, nil
	}
}

func decodeStepOutput(res DispatchResult) any {
	if len(res.Output) == 0 {
		return map[string]any{}
	}
	var v any
	if err := json.Unmarshal(res.Output, &v); err != nil {
		return string(res.Output)
	}
	return v
}
