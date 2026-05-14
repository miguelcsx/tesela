// Agent handlers — start a run, get a run, list runs, cancel.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/agents"
	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type agentHandlers struct {
	store   *storage.Store
	runtime *agents.Runtime
}

func newAgentHandlers(cfg ServerConfig) *agentHandlers {
	return &agentHandlers{store: cfg.Store, runtime: cfg.AgentRuntime}
}

type agentRunReq struct {
	Input string `json:"input"`
}

func (h *agentHandlers) Start(w http.ResponseWriter, r *http.Request) {
	if h.runtime == nil {
		writeError(w, r, errs.New(errs.CodeInternal, "agent runtime not configured"))
		return
	}
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	var body agentRunReq
	if err := decodeJSON(r, &body); err != nil {
		writeError(w, r, err)
		return
	}
	res, err := h.runtime.Start(r.Context(), agents.StartRequest{
		Actor: actor, WorkspaceID: ws.ID, Agent: types.APIName(chi.URLParam(r, "agent")),
		Input: body.Input, RequestID: requestIDFromContext(r.Context()),
	})
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusAccepted, res)
}

func (h *agentHandlers) GetRun(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	id := types.AgentRunID(chi.URLParam(r, "run_id"))
	out, err := h.store.AgentRuns().GetByID(r.Context(), ws.ID, id)
	if err != nil {
		writeError(w, r, err)
		return
	}
	toolCalls, err := h.store.AgentRuns().ListToolCalls(r.Context(), id)
	if err != nil {
		writeError(w, r, err)
		return
	}
	messages, err := h.store.AgentRuns().ListMessages(r.Context(), id)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{"run": out, "tool_calls": toolCalls, "messages": messages})
}

func (h *agentHandlers) ListRuns(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.store.AgentRuns().List(r.Context(), ws.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{"runs": out})
}

func (h *agentHandlers) context(r *http.Request) (types.Workspace, types.Actor, error) {
	name := chi.URLParam(r, "workspace")
	ws, err := h.store.Workspaces().GetByAPIName(r.Context(), types.APIName(name))
	if err != nil {
		return types.Workspace{}, types.Actor{}, err
	}
	actor, err := actorFromContext(r.Context())
	if err != nil {
		return types.Workspace{}, types.Actor{}, errs.Wrap(err, errs.CodeUnauthenticated, "actor")
	}
	return ws, actor, nil
}
