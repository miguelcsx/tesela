// GraphQL handler — accepts a {"query":"...","variables":{...}} POST and
// runs it against the workspace's schema.

package server

import (
	"context"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/graphql-go/graphql"

	"github.com/miguelcsx/lattice/pkg/lattice/errs"
	gqlpkg "github.com/miguelcsx/lattice/pkg/lattice/graphql"
	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type graphqlHandlers struct {
	store *storage.Store
	cache *gqlpkg.SchemaCache
}

func newGraphQLHandlers(cfg ServerConfig) *graphqlHandlers {
	return &graphqlHandlers{store: cfg.Store, cache: cfg.GraphQL}
}

type graphqlReq struct {
	Query     string         `json:"query"`
	Variables map[string]any `json:"variables,omitempty"`
}

func (h *graphqlHandlers) Execute(w http.ResponseWriter, r *http.Request) {
	if h.cache == nil {
		writeError(w, r, errs.New(errs.CodeInternal, "graphql not configured"))
		return
	}
	ws, actor, err := h.context(r)
	if err != nil {
		writeError(w, r, err)
		return
	}
	var body graphqlReq
	if err := decodeJSON(r, &body); err != nil {
		writeError(w, r, err)
		return
	}
	schema, err := h.cache.For(r.Context(), ws.ID)
	if err != nil {
		writeError(w, r, err)
		return
	}
	ctx := context.WithValue(r.Context(), gqlpkg.CtxKeyActor, actor)
	ctx = context.WithValue(ctx, gqlpkg.CtxKeyWorkspaceID, ws.ID)
	res := graphql.Do(graphql.Params{
		Schema:         *schema,
		RequestString:  body.Query,
		VariableValues: body.Variables,
		Context:        ctx,
	})
	writeJSON(w, r, http.StatusOK, res)
}

func (h *graphqlHandlers) context(r *http.Request) (types.Workspace, types.Actor, error) {
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
