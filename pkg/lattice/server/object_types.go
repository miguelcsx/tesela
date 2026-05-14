// ObjectType handlers — read-only over the ontology snapshot.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type objectTypeHandlers struct {
	store    *storage.Store
	ontology *ontology.Registry
}

func newObjectTypeHandlers(cfg ServerConfig) *objectTypeHandlers {
	return &objectTypeHandlers{store: cfg.Store, ontology: cfg.Ontology}
}

func (h *objectTypeHandlers) List(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.store.ObjectTypes().List(r.Context(), ws.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{"object_types": out})
}

func (h *objectTypeHandlers) Get(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	name := types.APIName(chi.URLParam(r, "name"))
	if err := name.Validate(); err != nil {
		writeError(w, r, errs.Wrap(err, errs.CodeValidation, "invalid api name"))
		return
	}
	out, err := h.store.ObjectTypes().GetByAPIName(r.Context(), ws.ID, name)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, out)
}

func (h *objectTypeHandlers) workspace(r *http.Request) (types.Workspace, error) {
	name := chi.URLParam(r, "workspace")
	return h.store.Workspaces().GetByAPIName(r.Context(), types.APIName(name))
}
