// Workspace handlers — CRUD for the tenant boundary.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type workspaceHandlers struct {
	store *storage.Store
}

func newWorkspaceHandlers(cfg ServerConfig) *workspaceHandlers {
	return &workspaceHandlers{store: cfg.Store}
}

type workspaceCreateReq struct {
	APIName     string                  `json:"api_name"`
	DisplayName string                  `json:"display_name"`
	Description string                  `json:"description,omitempty"`
	Settings    types.WorkspaceSettings `json:"settings,omitempty"`
}

func (h *workspaceHandlers) Create(w http.ResponseWriter, r *http.Request) {
	var req workspaceCreateReq
	if err := decodeJSON(r, &req); err != nil {
		writeError(w, r, err)
		return
	}
	if req.APIName == "" || req.DisplayName == "" {
		writeError(w, r, errs.New(errs.CodeValidation, "api_name and display_name are required"))
		return
	}
	created, err := h.store.Workspaces().Create(r.Context(), types.Workspace{
		ID:          types.WorkspaceID(ids.NewULID()),
		APIName:     types.APIName(req.APIName),
		DisplayName: req.DisplayName,
		Description: req.Description,
		Settings:    req.Settings,
	})
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusCreated, created)
}

func (h *workspaceHandlers) List(w http.ResponseWriter, r *http.Request) {
	out, err := h.store.Workspaces().List(r.Context())
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{"workspaces": out})
}

func (h *workspaceHandlers) Get(w http.ResponseWriter, r *http.Request) {
	ws, err := h.resolveWorkspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, ws)
}

func (h *workspaceHandlers) Update(w http.ResponseWriter, r *http.Request) {
	ws, err := h.resolveWorkspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	var patch workspaceCreateReq
	if err := decodeJSON(r, &patch); err != nil {
		writeError(w, r, err)
		return
	}
	if patch.DisplayName != "" {
		ws.DisplayName = patch.DisplayName
	}
	ws.Description = patch.Description
	ws.Settings = patch.Settings
	updated, err := h.store.Workspaces().Update(r.Context(), ws)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, updated)
}

func (h *workspaceHandlers) Delete(w http.ResponseWriter, r *http.Request) {
	ws, err := h.resolveWorkspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	if err := h.store.Workspaces().Delete(r.Context(), ws.ID); err != nil {
		writeError(w, r, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (h *workspaceHandlers) resolveWorkspace(r *http.Request) (types.Workspace, error) {
	name := chi.URLParam(r, "workspace")
	return h.store.Workspaces().GetByAPIName(r.Context(), types.APIName(name))
}
