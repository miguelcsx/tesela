// Callback handler for FFI-registered actions (Python/Node/Rust closures).

package actions

import (
	"context"
	"encoding/json"
	"fmt"
)

// CallbackHandler dispatches actions to Go closures registered via FFI.
type CallbackHandler struct {
	Callbacks map[string]func(context.Context, map[string]any) (map[string]any, error)
}

// NewCallbackHandler builds a handler backed by the provided callback map.
func NewCallbackHandler(callbacks map[string]func(context.Context, map[string]any) (map[string]any, error)) *CallbackHandler {
	return &CallbackHandler{Callbacks: callbacks}
}

// Dispatch implements Handler.
func (h *CallbackHandler) Dispatch(ctx context.Context, ev DispatchEvent) (DispatchResult, error) {
	fn, ok := h.Callbacks[string(ev.ActionType.APIName)]
	if !ok {
		return DispatchResult{}, fmt.Errorf("actions: no callback registered for %q", ev.ActionType.APIName)
	}
	out, err := fn(ctx, ev.Input)
	if err != nil {
		return DispatchResult{}, fmt.Errorf("callback action %q: %w", ev.ActionType.APIName, err)
	}
	raw, _ := json.Marshal(out)
	return DispatchResult{Output: raw, AffectedRows: 1}, nil
}
