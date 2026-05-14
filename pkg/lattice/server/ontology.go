// Ontology handlers — apply/export declarative specs, publish, diff, list versions.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type ontologyHandlers struct {
	store    *storage.Store
	ontology *ontology.Registry
}

func newOntologyHandlers(cfg ServerConfig) *ontologyHandlers {
	return &ontologyHandlers{store: cfg.Store, ontology: cfg.Ontology}
}

func (h *ontologyHandlers) Apply(w http.ResponseWriter, r *http.Request) {
	body, err := readBody(r)
	if err != nil {
		writeError(w, r, errs.Wrap(err, errs.CodeValidation, "read body"))
		return
	}
	diff, err := h.ontology.Apply(r.Context(), body)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, diff)
}

func (h *ontologyHandlers) Export(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.ontology.ExportDocument(r.Context(), ws.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	_, _ = w.Write(out)
}

type publishRequest struct {
	Name  string `json:"name"`
	Notes string `json:"notes,omitempty"`
}

func (h *ontologyHandlers) Publish(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	var req publishRequest
	if err := decodeJSON(r, &req); err != nil {
		writeError(w, r, err)
		return
	}
	if req.Name == "" {
		writeError(w, r, errs.New(errs.CodeValidation, "name is required"))
		return
	}
	actor, _ := actorFromContext(r.Context())
	v, err := h.ontology.Publish(r.Context(), ws.ID, req.Name, actor.UserID, req.Notes)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusCreated, v)
}

func (h *ontologyHandlers) ListVersions(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	out, err := h.store.OntologyVersions().List(r.Context(), ws.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, map[string]any{"versions": out})
}

func (h *ontologyHandlers) GetVersion(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	name := chi.URLParam(r, "name")
	out, err := h.store.OntologyVersions().GetByName(r.Context(), ws.ID, name)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, out)
}

func (h *ontologyHandlers) Diff(w http.ResponseWriter, r *http.Request) {
	ws, err := h.workspace(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	from := r.URL.Query().Get("from")
	to := r.URL.Query().Get("to")
	if from == "" || to == "" {
		writeError(w, r, errs.New(errs.CodeValidation, "from and to query params required"))
		return
	}
	d, err := h.ontology.Diff(r.Context(), ws.ID, from, to)
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, d)
}

func (h *ontologyHandlers) workspace(r *http.Request) (types.Workspace, error) {
	name := chi.URLParam(r, "workspace")
	return h.store.Workspaces().GetByAPIName(r.Context(), types.APIName(name))
}
