// installRoutes wires every endpoint onto r. Routes are grouped by resource;
// authenticated endpoints live behind the auth middleware in their own
// chi.Group.

package server

import (
	"net/http"

	"github.com/go-chi/chi/v5"
)

func installRoutes(r chi.Router, cfg ServerConfig) {
	// Public infrastructure endpoints.
	r.Get("/healthz", handleHealth)
	r.Get("/readyz", handleReady(cfg))
	r.Get("/v1/version", handleVersion(cfg))

	// Authenticated endpoints.
	r.Group(func(g chi.Router) {
		g.Use(authMiddleware(cfg.Authenticator))

		// Workspaces.
		ws := newWorkspaceHandlers(cfg)
		g.Post("/v1/workspaces", ws.Create)
		g.Get("/v1/workspaces", ws.List)
		g.Get("/v1/workspaces/{workspace}", ws.Get)
		g.Patch("/v1/workspaces/{workspace}", ws.Update)
		g.Delete("/v1/workspaces/{workspace}", ws.Delete)

		// Datasources.
		ds := newDatasourceHandlers(cfg)
		g.Post("/v1/workspaces/{workspace}/datasources", ds.Create)
		g.Get("/v1/workspaces/{workspace}/datasources", ds.List)
		g.Get("/v1/workspaces/{workspace}/datasources/{datasource}", ds.Get)
		g.Delete("/v1/workspaces/{workspace}/datasources/{datasource}", ds.Delete)

		// Ontology.
		on := newOntologyHandlers(cfg)
		g.Post("/v1/workspaces/{workspace}/ontology:apply", on.Apply)
		g.Get("/v1/workspaces/{workspace}/ontology:export", on.Export)
		g.Post("/v1/workspaces/{workspace}/ontology:publish", on.Publish)
		g.Get("/v1/workspaces/{workspace}/ontology/versions", on.ListVersions)
		g.Get("/v1/workspaces/{workspace}/ontology/versions/{name}", on.GetVersion)
		g.Get("/v1/workspaces/{workspace}/ontology:diff", on.Diff)

		// Object types CRUD individual.
		ot := newObjectTypeHandlers(cfg)
		g.Get("/v1/workspaces/{workspace}/object-types", ot.List)
		g.Get("/v1/workspaces/{workspace}/object-types/{name}", ot.Get)

		// Operational object API.
		obj := newObjectHandlers(cfg)
		g.Get("/v1/workspaces/{workspace}/objects/{type}/{pk}", obj.Get)
		g.Post("/v1/workspaces/{workspace}/objects/{type}:search", obj.Search)
		g.Post("/v1/workspaces/{workspace}/objects/{type}:aggregate", obj.Aggregate)
		g.Get("/v1/workspaces/{workspace}/objects/{type}/{pk}/links/{link}", obj.Traverse)

		// Actions.
		ac := newActionHandlers(cfg)
		g.Post("/v1/workspaces/{workspace}/actions/{type}:execute", ac.Execute)
		g.Get("/v1/workspaces/{workspace}/action-runs", ac.ListRuns)
		g.Get("/v1/workspaces/{workspace}/action-runs/{run_id}", ac.GetRun)

		// Uploads.
		up := newUploadHandlers(cfg)
		g.Post("/v1/workspaces/{workspace}/assets/{asset}/uploads", up.Create)
		g.Get("/v1/workspaces/{workspace}/assets/{asset}/uploads", up.List)
		g.Post("/v1/workspaces/{workspace}/uploads/{upload_id}:notify-uploaded", up.Notify)
		g.Post("/v1/workspaces/{workspace}/uploads/{upload_id}/mapping", up.SetMapping)
		g.Post("/v1/workspaces/{workspace}/uploads/{upload_id}:approve", up.Approve)
		g.Delete("/v1/workspaces/{workspace}/uploads/{upload_id}", up.Cancel)
		g.Get("/v1/workspaces/{workspace}/uploads/{upload_id}", up.Get)
		g.Get("/v1/workspaces/{workspace}/uploads/{upload_id}/proposed-mapping", up.ProposedMapping)
		g.Post("/v1/workspaces/{workspace}/uploads/{upload_id}:retry", up.Retry)
		g.Get("/v1/workspaces/{workspace}/uploads/{upload_id}/errors", up.Errors)
		g.Get("/v1/workspaces/{workspace}/assets/{asset}/versions", up.ListVersions)
		g.Get("/v1/workspaces/{workspace}/assets/{asset}/versions/latest", up.LatestVersion)

		// GraphQL.
		gql := newGraphQLHandlers(cfg)
		g.Post("/v1/workspaces/{workspace}/graphql", gql.Execute)

		// SDK codegen.
		sdk := newSDKHandlers(cfg)
		g.Get("/v1/workspaces/{workspace}/sdk/{lang}.zip", sdk.Generate)

		// Agents.
		ag := newAgentHandlers(cfg)
		g.Post("/v1/workspaces/{workspace}/agents/{agent}/runs", ag.Start)
		g.Get("/v1/workspaces/{workspace}/agent-runs", ag.ListRuns)
		g.Get("/v1/workspaces/{workspace}/agent-runs/{run_id}", ag.GetRun)

		// Audit.
		au := newAuditHandlers(cfg)
		g.Get("/v1/workspaces/{workspace}/audit", au.List)
	})
}

func handleHealth(w http.ResponseWriter, _ *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write([]byte(`{"status":"ok"}`))
}

func handleReady(cfg ServerConfig) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if err := cfg.Store.Ping(r.Context()); err != nil {
			writeError(w, r, err)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"status":"ready"}`))
	}
}

func handleVersion(cfg ServerConfig) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		writeJSON(w, r, http.StatusOK, cfg.BuildInfo)
	}
}
