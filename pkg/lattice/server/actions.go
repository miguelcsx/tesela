// Action handlers — execute, get run, list runs, cancel.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/actions"
	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type actionHandlers struct {
	store    *storage.Store
	pipeline *actions.Pipeline
}

func newActionHandlers(cfg ServerConfig) *actionHandlers {
	return &actionHandlers{store: cfg.Store, pipeline: cfg.ActionPipeline}
}

type executeRequest struct {
	Input          map[string]any `json:"input"`
	Subject        string         `json:"subject,omitempty"`
	IdempotencyKey string         `json:"-"`
}

func (h *actionHandlers) Execute(w http.ResponseWriter, r *http.Request) {
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	if h.pipeline == nil {
		writeError(w, r, errs.New(errs.CodeInternal, "action pipeline not configured"))
		return
	}
	var req executeRequest
	if err := decodeJSON(r, &req); err != nil {
		writeError(w, r, err)
		return
	}
	res, err := h.pipeline.Execute(r.Context(), actions.ExecuteRequest{
		Actor:          actor,
		WorkspaceID:    ws.ID,
		ActionTypeName: types.APIName(chi.URLParam(r, "type")),
		Input:          req.Input,
		IdempotencyKey: r.Header.Get("Idempotency-Key"),
		RequestID:      requestIDFromContext(r.Context()),
		SubjectKey:     req.Subject,
	})
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, res)
}

func (h *actionHandlers) GetRun(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	id := types.ActionRunID(chi.URLParam(r, "run_id"))
	out, err := h.store.ActionRuns().GetByID(r.Context(), ws.ID, id)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, out)
}

func (h *actionHandlers) ListRuns(w http.ResponseWriter, r *http.Request) {
	ws, _, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.store.ActionRuns().List(r.Context(), ws.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{"runs": out})
}

func (h *actionHandlers) context(r *http.Request) (types.Workspace, types.Actor, error) {
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
