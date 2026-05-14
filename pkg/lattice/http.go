// Lean built-in HTTP handler. Pipelines are constructed lazily so the builder
// phase stays cheap; once Handler() is called the App is effectively frozen.
// The full production server lives in pkg/lattice/server/.

package lattice

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/miguelcsx/lattice/pkg/lattice/backend"
	"github.com/miguelcsx/lattice/pkg/lattice/events"
	"github.com/miguelcsx/lattice/pkg/lattice/mcp"
	"github.com/miguelcsx/lattice/pkg/lattice/policy"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// changeBody is the JSON wire format for object change events.
type changeBody struct {
	Before json.RawMessage `json:"before,omitempty"`
	After  json.RawMessage `json:"after,omitempty"`
}

// ---------------------------------------------------------------------
// Handler entry point
// ---------------------------------------------------------------------

// Handler returns the http.Handler. Idempotent — caches on first build.
func (a *App) Handler() http.Handler {
	a.mu.Lock()
	if a.handler != nil {
		h := a.handler
		a.mu.Unlock()
		return h
	}
	a.mu.Unlock()

	snap := a.snapshot()
	a.registerInlineBackends(snap)

	resolver := newStaticResolver(snap)
	policyResolver := newPolicyResolver()

	mux := http.NewServeMux()
	mux.HandleFunc("/healthz", healthHandler)
	mux.HandleFunc("/readyz", healthHandler)
	mux.HandleFunc("/v1/version", versionHandler)
	mux.Handle("/v1/workspaces/", a.workspaceRouter(resolver, policyResolver))

	wrapped := a.applyMiddleware(mux)

	a.mu.Lock()
	a.handler = wrapped
	a.mu.Unlock()
	return wrapped
}

// Serve starts the HTTP server on addr and blocks until shutdown.
func (a *App) Serve(addr string) error {
	return http.ListenAndServe(addr, a.Handler())
}

// ServeGraceful starts the HTTP server and blocks until ctx is cancelled.
func (a *App) ServeGraceful(ctx context.Context, addr string) error {
	srv := &http.Server{Addr: addr, Handler: a.Handler()}
	go func() {
		<-ctx.Done()
		shutCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		_ = srv.Shutdown(shutCtx)
	}()
	return srv.ListenAndServe()
}

// Shutdown gracefully stops the server if one is running.
func (a *App) Shutdown(ctx context.Context) error {
	if a.schedulerCancel != nil {
		a.schedulerCancel()
	}
	// This is still a no-op for the simple embedded server; the caller should
	// cancel the context passed to ServeGraceful for HTTP shutdown.
	return nil
}

// ---------------------------------------------------------------------
// Inline backend wiring
// ---------------------------------------------------------------------

func (a *App) registerInlineBackends(snap *types.Ontology) {
	a.mu.Lock()
	defer a.mu.Unlock()
	for _, b := range a.objectTypes {
		search, get, mutate, ok := b.InlineClosures()
		if !ok {
			continue
		}
		typeName := types.APIName(b.APIName())
		bus := a.bus
		var publish func(ctx context.Context, kind events.Kind, mut types.Mutation, res types.MutationResult)
		if bus != nil {
			publish = func(ctx context.Context, kind events.Kind, mut types.Mutation, res types.MutationResult) {
				body, _ := json.Marshal(changeBody{After: marshalRaw(mut.Values)})
				pk := ""
				if mut.PrimaryKey != nil {
					pk = primaryKeyString(mut.PrimaryKey)
				}
				_ = bus.Publish(ctx, events.Event{
					Kind:        kind,
					WorkspaceID: a.workspace.ID,
					ObjectType:  typeName,
					PrimaryKey:  pk,
					Actor:       actorIDFromContext(ctx),
					Body:        body,
				})
			}
		}
		bk := backend.NewInlineBackend(b.APIName(), search, get, mutate, publish)
		a.backends.Register(bk)
		dsName := types.APIName(b.APIName() + "_inline")
		ds := types.Datasource{
			ID:          types.DatasourceID(b.APIName() + "-ds"),
			WorkspaceID: a.workspace.ID,
			APIName:     dsName,
			AdapterType: bk.Type(),
		}
		snap.Datasources = append(snap.Datasources, ds)
	}
}

// ---------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------

func (a *App) applyMiddleware(h http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		var actor types.Actor
		var err error
		a.mu.Lock()
		cb := a.authCallback
		a.mu.Unlock()
		if cb != nil {
			headers := make(map[string]string)
			for k, v := range r.Header {
				if len(v) > 0 {
					headers[k] = v[0]
				}
			}
			actor, err = cb(map[string]any{
				"method":  r.Method,
				"path":    r.URL.Path,
				"headers": headers,
			})
		} else {
			actor, err = a.authenticator(r)
		}
		if err != nil {
			http.Error(w, `{"error":{"code":"unauthenticated","message":"`+err.Error()+`"}}`, http.StatusUnauthorized)
			return
		}
		ctx := withActor(r.Context(), actor)
		h.ServeHTTP(w, r.WithContext(ctx))
	})
}

func healthHandler(w http.ResponseWriter, _ *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	_, _ = w.Write([]byte(`{"status":"ok"}`))
}

func versionHandler(w http.ResponseWriter, _ *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	_, _ = w.Write([]byte(`{"version":"dev"}`))
}

// ---------------------------------------------------------------------
// Resolvers
// ---------------------------------------------------------------------

type staticResolver struct {
	mu   sync.RWMutex
	snap *types.Ontology
}

func newStaticResolver(snap *types.Ontology) *staticResolver {
	return &staticResolver{snap: snap}
}

func (r *staticResolver) Snapshot(_ context.Context, _ types.WorkspaceID) (*types.Ontology, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.snap, nil
}

func (r *staticResolver) ObjectTypeByName(name types.APIName) (types.ObjectType, bool) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.snap.ObjectTypeByName(name)
}

type policyResolverImpl struct {
	mu    sync.Mutex
	cache map[*types.Ontology]*policy.Evaluator
}

func newPolicyResolver() *policyResolverImpl {
	return &policyResolverImpl{cache: make(map[*types.Ontology]*policy.Evaluator)}
}

func (r *policyResolverImpl) For(snap *types.Ontology) (*policy.Evaluator, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if e, ok := r.cache[snap]; ok {
		return e, nil
	}
	e, err := policy.NewEvaluator(snap)
	if err != nil {
		return nil, err
	}
	r.cache[snap] = e
	return e, nil
}

// ---------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------

func (a *App) workspaceRouter(resolver *staticResolver, polr *policyResolverImpl) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		actor, ok := actorFromContext(r.Context())
		if !ok {
			http.Error(w, `{"error":{"code":"unauthenticated"}}`, http.StatusUnauthorized)
			return
		}
		switch op := classifyOp(r); op {
		case opSearch:
			a.handleSearch(w, r, resolver, polr, actor)
		case opGet:
			a.handleGet(w, r, resolver, polr, actor)
		case opSubscribe:
			a.handleSubscribe(w, r, actor)
		case opMCP:
			a.handleMCP(w, r, actor)
		default:
			http.NotFound(w, r)
		}
	})
}

// ---------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------

func (a *App) handleSearch(w http.ResponseWriter, r *http.Request, resolver *staticResolver, polr *policyResolverImpl, actor types.Actor) {
	wsName, otName := parseObjectPath(r.URL.Path)
	_ = wsName
	snap, _ := resolver.Snapshot(r.Context(), a.workspace.ID)
	ot, ok := snap.ObjectTypeByName(types.APIName(otName))
	if !ok {
		http.Error(w, `{"error":{"code":"not_found","message":"object type"}}`, http.StatusNotFound)
		return
	}
	eval, err := polr.For(snap)
	if err != nil {
		http.Error(w, `{"error":{"code":"internal"}}`, http.StatusInternalServerError)
		return
	}
	dec := eval.Evaluate(policy.Request{
		Actor: actor, Operation: types.OperationSearch,
		ResourceKind: types.KindObjectType, ResourceName: ot.APIName,
	})
	if !dec.Allow {
		http.Error(w, `{"error":{"code":"policy_denied","message":"`+dec.Reason+`"}}`, http.StatusForbidden)
		return
	}
	var spec types.QuerySpec
	if r.ContentLength > 0 {
		_ = json.NewDecoder(r.Body).Decode(&spec)
	}
	page, err := a.runSearch(r.Context(), ot, spec, dec.Filter)
	if err != nil {
		http.Error(w, `{"error":{"code":"adapter","message":"`+err.Error()+`"}}`, http.StatusBadGateway)
		return
	}
	red := append(append([]types.APIName(nil), dec.Redactions...), policy.MarkingRedactions(actor, ot)...)
	page = policy.ApplyToPage(page, red)
	a.writeAudit(types.OperationSearch, ot.APIName, actor, dec, int64(len(page.Records)))
	writeJSON(w, http.StatusOK, page)
}

func (a *App) handleGet(w http.ResponseWriter, r *http.Request, resolver *staticResolver, polr *policyResolverImpl, actor types.Actor) {
	wsName, otName, pk := parseGetPath(r.URL.Path)
	_ = wsName
	snap, _ := resolver.Snapshot(r.Context(), a.workspace.ID)
	ot, ok := snap.ObjectTypeByName(types.APIName(otName))
	if !ok {
		http.Error(w, `{"error":{"code":"not_found"}}`, http.StatusNotFound)
		return
	}
	eval, _ := polr.For(snap)
	dec := eval.Evaluate(policy.Request{
		Actor: actor, Operation: types.OperationRead,
		ResourceKind: types.KindObjectType, ResourceName: ot.APIName,
	})
	if !dec.Allow {
		http.Error(w, `{"error":{"code":"policy_denied"}}`, http.StatusForbidden)
		return
	}
	rec, err := a.runGet(r.Context(), ot, pk, dec.Filter)
	if err != nil {
		http.Error(w, `{"error":{"code":"adapter","message":"`+err.Error()+`"}}`, http.StatusBadGateway)
		return
	}
	red := append(append([]types.APIName(nil), dec.Redactions...), policy.MarkingRedactions(actor, ot)...)
	rec = policy.ApplyToRecord(rec, red)
	a.writeAudit(types.OperationRead, ot.APIName, actor, dec, 1)
	writeJSON(w, http.StatusOK, rec)
}

func (a *App) handleMCP(w http.ResponseWriter, r *http.Request, actor types.Actor) {
	snap := a.snapshot()
	srv := mcp.NewServer(mcp.ServerConfig{
		ServerName: string(a.workspace.APIName),
		Snapshot:   snap,
		Actor:      actor,
		Search: func(ctx context.Context, ot types.ObjectType, spec types.QuerySpec) (types.Page, error) {
			return a.runSearch(ctx, ot, spec, types.Filter{})
		},
		Get: func(ctx context.Context, ot types.ObjectType, pk any) (types.Record, error) {
			return a.runGet(ctx, ot, pk, types.Filter{})
		},
	})
	srv.ServeHTTP(w, r)
}

func (a *App) handleSubscribe(w http.ResponseWriter, r *http.Request, _ types.Actor) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		http.Error(w, "streaming unsupported", http.StatusInternalServerError)
		return
	}
	if a.bus == nil {
		http.Error(w, "events bus not configured", http.StatusServiceUnavailable)
		return
	}

	filter := events.Filter{}
	for _, k := range r.URL.Query()["kind"] {
		filter.Kinds = append(filter.Kinds, events.Kind(k))
	}
	for _, n := range r.URL.Query()["object_type"] {
		filter.ObjectTypes = append(filter.ObjectTypes, types.APIName(n))
	}

	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.WriteHeader(http.StatusOK)
	flusher.Flush()

	out := make(chan events.Event, 64)
	sub, err := a.bus.Subscribe(filter, func(_ context.Context, e events.Event) error {
		select {
		case out <- e:
		default:
		}
		return nil
	})
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	defer func() { _ = sub.Close() }()

	keepalive := time.NewTicker(20 * time.Second)
	defer keepalive.Stop()

	notify := r.Context().Done()
	for {
		select {
		case <-notify:
			return
		case e := <-out:
			payload, _ := json.Marshal(e)
			fmt.Fprintf(w, "event: %s\ndata: %s\n\n", e.Kind, payload)
			flusher.Flush()
		case <-keepalive.C:
			fmt.Fprintf(w, ": keepalive\n\n")
			flusher.Flush()
		}
	}
}

// ---------------------------------------------------------------------
// Backend dispatch
// ---------------------------------------------------------------------

func (a *App) runSearch(ctx context.Context, ot types.ObjectType, spec types.QuerySpec, extra types.Filter) (types.Page, error) {
	conn, err := a.acquireConn(ctx, ot)
	if err != nil {
		return types.Page{}, err
	}
	s, err := backend.AsSearcher(conn)
	if err != nil {
		return types.Page{}, err
	}
	return s.Search(ctx, ot.Source, ot, spec, extra)
}

func (a *App) runGet(ctx context.Context, ot types.ObjectType, pk any, extra types.Filter) (types.Record, error) {
	conn, err := a.acquireConn(ctx, ot)
	if err != nil {
		return types.Record{}, err
	}
	g, err := backend.AsGetter(conn)
	if err != nil {
		return types.Record{}, err
	}
	return g.Get(ctx, ot.Source, ot, pk, extra)
}

func (a *App) acquireConn(ctx context.Context, ot types.ObjectType) (backend.Connection, error) {
	a.mu.Lock()
	ds := types.Datasource{
		WorkspaceID: a.workspace.ID,
		APIName:     ot.Source.DatasourceAPIName,
		AdapterType: backendType(a.backends, string(ot.Source.DatasourceAPIName)),
	}
	a.mu.Unlock()
	return a.backends.Acquire(ctx, ds)
}

func backendType(reg *backend.Registry, dsName string) string {
	if len(dsName) > 7 && dsName[len(dsName)-7:] == "_inline" {
		return "inline:" + dsName[:len(dsName)-7]
	}
	return ""
}

// ---------------------------------------------------------------------
// Audit helpers
// ---------------------------------------------------------------------

func (a *App) writeAudit(op types.Operation, name types.APIName, actor types.Actor, dec policy.Decision, count int64) {
	rec := types.AuditRecord{
		WorkspaceID:        a.workspace.ID,
		OccurredAt:         time.Now().UTC(),
		Operation:          op,
		ResourceKind:       string(types.KindObjectType),
		ResourceAPIName:    name,
		ActorUserID:        actor.UserID,
		ActorRoles:         append([]string(nil), actor.Roles...),
		PolicyDecision:     decisionToAudit(dec),
		MatchedRules:       dec.MatchedRules,
		RedactedProperties: dec.Redactions,
		ResultCount:        count,
	}
	_ = a.auditWriter.Write(context.Background(), rec)
}

func decisionToAudit(d policy.Decision) types.AuditDecision {
	if d.Allow {
		return types.AuditDecisionAllow
	}
	return types.AuditDecisionDeny
}

func writeJSON(w http.ResponseWriter, status int, body any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(body)
}

func marshalRaw(v any) json.RawMessage {
	if v == nil {
		return json.RawMessage("null")
	}
	b, err := json.Marshal(v)
	if err != nil {
		return json.RawMessage("null")
	}
	return b
}

// ---------------------------------------------------------------------
// Routing helpers
// ---------------------------------------------------------------------

type opKind int

const (
	opUnknown opKind = iota
	opSearch
	opGet
	opSubscribe
	opMCP
)

func classifyOp(r *http.Request) opKind {
	switch r.Method {
	case "POST":
		if strings.HasSuffix(r.URL.Path, ":search") {
			return opSearch
		}
		if strings.HasSuffix(r.URL.Path, "/mcp") {
			return opMCP
		}
	case "GET":
		if strings.HasSuffix(r.URL.Path, "/subscribe") {
			return opSubscribe
		}
		parts := splitPath(r.URL.Path)
		if len(parts) == 6 && parts[0] == "v1" && parts[1] == "workspaces" && parts[3] == "objects" {
			return opGet
		}
	}
	return opUnknown
}

func actorIDFromContext(ctx context.Context) string {
	if a, ok := actorFromContext(ctx); ok {
		return a.UserID
	}
	return ""
}

func primaryKeyString(v any) string {
	switch x := v.(type) {
	case string:
		return x
	case []byte:
		return string(x)
	default:
		return ""
	}
}

func parseObjectPath(p string) (workspace, objectType string) {
	parts := splitPath(p)
	if len(parts) < 5 {
		return "", ""
	}
	objectType = parts[4]
	if i := strings.Index(objectType, ":"); i >= 0 {
		objectType = objectType[:i]
	}
	return parts[2], objectType
}

func parseGetPath(p string) (workspace, objectType, pk string) {
	parts := splitPath(p)
	if len(parts) < 6 {
		return "", "", ""
	}
	return parts[2], parts[4], parts[5]
}

func splitPath(p string) []string {
	p = strings.Trim(p, "/")
	if p == "" {
		return nil
	}
	return strings.Split(p, "/")
}

// actor context plumbing
type actorCtxKey struct{}

func withActor(ctx context.Context, a types.Actor) context.Context {
	return context.WithValue(ctx, actorCtxKey{}, a)
}

func actorFromContext(ctx context.Context) (types.Actor, bool) {
	a, ok := ctx.Value(actorCtxKey{}).(types.Actor)
	return a, ok
}
