// Audit handler — read-only listing of audit records for a workspace.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type auditHandlers struct {
	store *storage.Store
}

func newAuditHandlers(cfg ServerConfig) *auditHandlers {
	return &auditHandlers{store: cfg.Store}
}

func (h *auditHandlers) List(w http.ResponseWriter, r *http.Request) {
	name := chi.URLParam(r, "workspace")
	ws, err := h.store.Workspaces().GetByAPIName(r.Context(), types.APIName(name))
	if err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.store.AuditRecords().List(r.Context(), ws.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{"records": out})
}
