// Datasource handlers — CRUD plus a :test endpoint that does a Connect+Ping.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type datasourceHandlers struct {
	store *storage.Store
}

func newDatasourceHandlers(cfg ServerConfig) *datasourceHandlers {
	return &datasourceHandlers{store: cfg.Store}
}

type datasourceCreateReq struct {
	APIName     string          `json:"api_name"`
	DisplayName string          `json:"display_name"`
	AdapterType string          `json:"adapter_type"`
	Config      types.ConfigMap `json:"config"`
}

func (h *datasourceHandlers) Create(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	var req datasourceCreateReq
	if err := decodeJSON(r, &req); err != nil {
		writeError(w, r, err)
		return
	}
	if req.APIName == "" || req.AdapterType == "" {
		writeError(w, r, errs.New(errs.CodeValidation, "api_name and adapter_type are required"))
		return
	}
	created, err := h.store.Datasources().Create(r.Context(), types.Datasource{
		ID:          types.DatasourceID(ids.NewULID()),
		WorkspaceID: ws.ID,
		APIName:     types.APIName(req.APIName),
		DisplayName: req.DisplayName,
		AdapterType: req.AdapterType,
		Config:      req.Config,
	})
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusCreated, created)
}

func (h *datasourceHandlers) List(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.store.Datasources().List(r.Context(), ws.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{"datasources": out})
}

func (h *datasourceHandlers) Get(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	name := chi.URLParam(r, "datasource")
	out, err := h.store.Datasources().GetByAPIName(r.Context(), ws.ID, types.APIName(name))
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, out)
}

func (h *datasourceHandlers) Delete(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	name := chi.URLParam(r, "datasource")
	if err := h.store.Datasources().Delete(r.Context(), ws.ID, types.APIName(name)); err != nil {
		writeError(w, r, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (h *datasourceHandlers) workspace(r *http.Request) (types.Workspace, error) {
	name := chi.URLParam(r, "workspace")
	return h.store.Workspaces().GetByAPIName(r.Context(), types.APIName(name))
}
