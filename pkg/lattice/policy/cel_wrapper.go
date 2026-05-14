// celWrapper adapts cel.Program to the celProgram interface used by loader.

package policy

import (
	"fmt"

	"github.com/google/cel-go/cel"
)

type celWrapper struct{ prg cel.Program }

func (w celWrapper) eval(actor, resource, input map[string]any) (any, error) {
	out, _, err := w.prg.Eval(map[string]any{
		"actor":    actor,
		"resource": resource,
		"input":    input,
		"subject":  resource, // alias for ergonomic CEL expressions
		"now":      "",
	})
	if err != nil {
		return nil, fmt.Errorf("cel eval: %w", err)
	}
	return out.Value(), nil
}
