// Object handlers — the operational data API. Each handler resolves the
// workspace, parses the request, and delegates to the query pipeline.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"
	"go.opentelemetry.io/otel/trace"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	"github.com/miguelcsx/lattice/pkg/lattice/query"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type objectHandlers struct {
	store    *storage.Store
	pipeline *query.Pipeline
}

func newObjectHandlers(cfg ServerConfig) *objectHandlers {
	return &objectHandlers{store: cfg.Store, pipeline: cfg.QueryPipeline}
}

func (h *objectHandlers) Get(w http.ResponseWriter, r *http.Request) {
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	rec, err := h.pipeline.Get(r.Context(), query.GetRequest{
		Actor: actor, WorkspaceID: ws.ID,
		ObjectType: types.APIName(chi.URLParam(r, "type")),
		PrimaryKey: chi.URLParam(r, "pk"),
		RequestID:  requestIDFromContext(r.Context()),
		TraceID:    traceIDFromContext(r),
	})
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, rec)
}

func (h *objectHandlers) Search(w http.ResponseWriter, r *http.Request) {
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	var spec types.QuerySpec
	if err := decodeJSON(r, &spec); err != nil {
		writeError(w, r, err)
		return
	}
	if !spec.Filter.IsZero() {
		if err := spec.Filter.Validate(); err != nil {
			writeError(w, r, errs.Wrap(err, errs.CodeValidation, "filter"))
			return
		}
	}
	page, err := h.pipeline.Search(r.Context(), query.SearchRequest{
		Actor: actor, WorkspaceID: ws.ID,
		ObjectType: types.APIName(chi.URLParam(r, "type")),
		Spec:       spec,
		RequestID:  requestIDFromContext(r.Context()),
		TraceID:    traceIDFromContext(r),
	})
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, page)
}

func (h *objectHandlers) Aggregate(w http.ResponseWriter, r *http.Request) {
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	var spec types.AggregateSpec
	if err := decodeJSON(r, &spec); err != nil {
		writeError(w, r, err)
		return
	}
	res, err := h.pipeline.Aggregate(r.Context(), query.AggregateRequest{
		Actor: actor, WorkspaceID: ws.ID,
		ObjectType: types.APIName(chi.URLParam(r, "type")),
		Spec:       spec,
		RequestID:  requestIDFromContext(r.Context()),
		TraceID:    traceIDFromContext(r),
	})
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, res)
}

func (h *objectHandlers) Traverse(w http.ResponseWriter, r *http.Request) {
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	page, err := h.pipeline.Traverse(r.Context(), query.TraverseRequest{
		Actor: actor, WorkspaceID: ws.ID,
		From:      types.APIName(chi.URLParam(r, "type")),
		LinkType:  types.APIName(chi.URLParam(r, "link")),
		SourceKey: chi.URLParam(r, "pk"),
		RequestID: requestIDFromContext(r.Context()),
		TraceID:   traceIDFromContext(r),
	})
	if err != nil {
		writeError(w, r, err)
		return
	}
	writeJSON(w, r, http.StatusOK, page)
}

func (h *objectHandlers) context(r *http.Request) (types.Workspace, types.Actor, error) {
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

func traceIDFromContext(r *http.Request) string {
	span := trace.SpanContextFromContext(r.Context())
	if !span.IsValid() {
		return ""
	}
	return span.TraceID().String()
}
