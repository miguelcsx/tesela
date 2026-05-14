// SDK download handler — generates a typed SDK on demand.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"

	"github.com/miguelcsx/lattice/pkg/lattice/codegen"
	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/ontology"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type sdkHandlers struct {
	store    *storage.Store
	ontology *ontology.Registry
}

func newSDKHandlers(cfg ServerConfig) *sdkHandlers {
	return &sdkHandlers{store: cfg.Store, ontology: cfg.Ontology}
}

func (h *sdkHandlers) Generate(w http.ResponseWriter, r *http.Request) {
	name := types.APIName(chi.URLParam(r, "workspace"))
	ws, err := h.store.Workspaces().GetByAPIName(r.Context(), name)
	if err != nil {
		writeError(w, r, err)
		return
	}
	snap, err := h.ontology.Snapshot(r.Context(), ws.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	lang := chi.URLParam(r, "lang")
	zip, err := codegen.Generate(lang, snap)
	if err != nil {
		writeError(w, r, errs.Wrap(err, errs.CodeValidation, "codegen"))
		return
	}
	w.Header().Set("Content-Type", "application/zip")
	w.Header().Set("Content-Disposition", `attachment; filename="lattice-sdk-`+lang+`.zip"`)
	_, _ = w.Write(zip)
}
