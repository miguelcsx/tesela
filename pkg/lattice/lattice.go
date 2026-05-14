// App is the central composition: ontology + backends + policies + audit
// wired into an http.Handler that exposes REST + GraphQL.

package lattice

import (
	"context"
	"errors"
	"log/slog"
	"net/http"
	"sync"

	"github.com/miguelcsx/lattice/pkg/lattice/audit"
	"github.com/miguelcsx/lattice/pkg/lattice/backend"
	"github.com/miguelcsx/lattice/pkg/lattice/branch"
	"github.com/miguelcsx/lattice/pkg/lattice/events"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/schedule"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
	"github.com/miguelcsx/lattice/pkg/lattice/workflow"
)

// App is the user-facing facade. Construct with New, accumulate ontology
// via builders, then call Handler() to mount on any http.ServeMux.
type App struct {
	mu sync.Mutex

	workspace types.Workspace

	objectTypes     []*ObjectTypeBuilder
	linkTypes       []*LinkTypeBuilder
	actions         []*ActionBuilder
	assets          []*AssetBuilder
	policies        []types.PolicyRule
	roles           []types.Role
	agents          []types.Agent
	agentBuilders   []*AgentBuilder
	customTools     []*CustomToolBuilder
	callbackActions map[string]func(context.Context, map[string]any) (map[string]any, error)
	callbackTools   map[string]func(context.Context, map[string]any) (map[string]any, error)
	authCallback    func(map[string]any) (types.Actor, error)
	auditCallback   func([]types.AuditRecord) error
	eventCallback   func(events.Event) error
	datasources     map[string]types.Datasource

	backends        *backend.Registry
	auditWriter     *audit.Writer
	auditSink       audit.Sink
	bus             events.Bus
	branches        *branch.Manager
	scheduler       *schedule.Scheduler
	schedulerCancel context.CancelFunc
	workflows       *workflow.Engine

	authenticator AuthFunc
	logger        *slog.Logger
	errs          []error

	handler http.Handler // built lazily on first Handler() call
}

// New constructs an App with sensible dev defaults. Defaults:
//
//   - In-memory ontology (no metadata DB).
//   - Audit goes to stdout via slog.
//   - Authentication is no-op (every request is the "dev" actor with role "admin").
//
// Override via options.
func New(opts ...Option) *App {
	a := &App{
		workspace: types.Workspace{
			ID:          types.WorkspaceID(ids.NewULID()),
			APIName:     "default",
			DisplayName: "Default Workspace",
		},
		datasources: make(map[string]types.Datasource),
		backends:    backend.NewRegistry(nil),
		logger:      slog.Default(),
		scheduler:   schedule.NewScheduler(),
		workflows:   workflow.NewEngine(workflow.NewMemoryStore()),
	}
	for _, o := range opts {
		o(a)
	}
	if a.bus == nil {
		a.bus = events.NewMemoryBus()
	}
	if a.branches == nil {
		a.branches = branch.NewManager(branch.NewMemoryStore())
	}
	if a.auditSink == nil {
		a.auditSink = newSlogSink(a.logger)
	}
	if a.authenticator == nil {
		a.authenticator = devAuth
	}
	// Wrap the user-configured audit sink so every audit emission is also
	// fanned out as an event. If the user replaces the sink later via
	// WithAuditSink (only honored at construction), the wrap is preserved.
	a.auditSink = audit.NewEventBridge(a.auditSink, a.bus)
	a.auditWriter = audit.NewWriter(a.auditSink, audit.Config{Logger: a.logger})
	schedulerCtx, schedulerCancel := context.WithCancel(context.Background())
	a.schedulerCancel = schedulerCancel
	go func() {
		_ = a.scheduler.Run(schedulerCtx)
	}()
	return a
}

// Events returns the configured event bus. Useful for advanced wiring
// (multiple subscribers, custom fan-out). Most users prefer OnChange /
// OnAudit which are typed wrappers.
func (a *App) Events() events.Bus { return a.bus }

// Branches returns the branch.Manager. Use for create/promote/diff/merge
// of ontology versions.
func (a *App) Branches() *branch.Manager { return a.branches }

// Scheduler returns the configured in-process scheduler.
func (a *App) Scheduler() *schedule.Scheduler { return a.scheduler }

// Workflows returns the configured workflow engine.
func (a *App) Workflows() *workflow.Engine { return a.workflows }

// Errors returns every accumulated configuration error from the builders.
// Callers should check this before calling Handler().
func (a *App) Errors() []error { return a.errs }

// addError records a builder-time configuration error.
func (a *App) addError(err error) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.errs = append(a.errs, err)
}

// addAgent appends an agent to the App.
func (a *App) addAgent(agent types.Agent) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.agents = append(a.agents, agent)
}

// addCustomTool appends a custom tool builder.
func (a *App) addCustomTool(b *CustomToolBuilder) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.customTools = append(a.customTools, b)
}

// addAgentBuilder appends an agent builder.
func (a *App) addAgentBuilder(b *AgentBuilder) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.agentBuilders = append(a.agentBuilders, b)
}

// RegisterActionCallback stores a Python/FFI callback for an action.
func (a *App) RegisterActionCallback(apiName string, fn func(context.Context, map[string]any) (map[string]any, error)) {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.callbackActions == nil {
		a.callbackActions = make(map[string]func(context.Context, map[string]any) (map[string]any, error))
	}
	a.callbackActions[apiName] = fn
}

// RegisterCustomToolCallback stores a Python/FFI callback for a custom tool.
func (a *App) RegisterCustomToolCallback(apiName string, fn func(context.Context, map[string]any) (map[string]any, error)) {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.callbackTools == nil {
		a.callbackTools = make(map[string]func(context.Context, map[string]any) (map[string]any, error))
	}
	a.callbackTools[apiName] = fn
}

// RegisterDatasource stores a datasource config for FFI bindings.
func (a *App) RegisterDatasource(apiName, adapterType string, config types.ConfigMap) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.datasources[apiName] = types.Datasource{
		ID:          types.DatasourceID(ids.NewULID()),
		WorkspaceID: a.workspace.ID,
		APIName:     types.APIName(apiName),
		AdapterType: adapterType,
		Config:      config,
	}
}

// SetAuthCallback registers a Python/FFI authentication callback.
func (a *App) SetAuthCallback(fn func(reqMeta map[string]any) (types.Actor, error)) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.authCallback = fn
}

// SetAuditCallback registers a Python/FFI audit sink callback.
func (a *App) SetAuditCallback(fn func(records []types.AuditRecord) error) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.auditCallback = fn
}

// SetEventCallback registers a Python/FFI event subscriber.
func (a *App) SetEventCallback(fn func(event events.Event) error) {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.eventCallback = fn
}

// RegisterWorkflow adds or replaces a workflow definition on the in-process engine.
func (a *App) RegisterWorkflow(def workflow.Definition) {
	a.mu.Lock()
	engine := a.workflows
	a.mu.Unlock()
	engine.Register(def)
}

// StartWorkflow launches a named workflow with the provided initial state.
func (a *App) StartWorkflow(ctx context.Context, name string, initial workflow.State) (workflow.Run, error) {
	a.mu.Lock()
	engine := a.workflows
	a.mu.Unlock()
	return engine.Start(ctx, name, cloneWorkflowState(initial))
}

// ScheduleWorkflow registers a cron job that starts the named workflow.
func (a *App) ScheduleWorkflow(id, expr, workflowName string, initial workflow.State) error {
	a.mu.Lock()
	s := a.scheduler
	a.mu.Unlock()
	return s.Add(id, expr, func(ctx context.Context) error {
		_, err := a.StartWorkflow(ctx, workflowName, initial)
		return err
	})
}

// SetLogLevel changes the runtime log level.
func (a *App) SetLogLevel(level string) {
	a.mu.Lock()
	defer a.mu.Unlock()
	var lvl slog.Level
	switch level {
	case "debug":
		lvl = slog.LevelDebug
	case "info":
		lvl = slog.LevelInfo
	case "warn", "warning":
		lvl = slog.LevelWarn
	case "error":
		lvl = slog.LevelError
	default:
		lvl = slog.LevelInfo
	}
	_ = lvl
	a.logger = slog.New(slog.Default().Handler()).WithGroup("lattice")
	// Re-wrap audit writer with new logger.
	a.auditWriter = audit.NewWriter(a.auditSink, audit.Config{Logger: a.logger})
}

// snapshot materializes the in-memory ontology into a *types.Ontology that
// the pipelines can consume. Called from Handler() after all builders ran.
func (a *App) snapshot() *types.Ontology {
	a.mu.Lock()
	defer a.mu.Unlock()
	ots := make([]types.ObjectType, 0, len(a.objectTypes))
	for _, b := range a.objectTypes {
		ots = append(ots, b.materialize(a.workspace.ID))
	}
	lts := make([]types.LinkType, 0, len(a.linkTypes))
	for _, b := range a.linkTypes {
		lts = append(lts, b.materialize(a.workspace.ID))
	}
	ats := make([]types.ActionType, 0, len(a.actions))
	for _, b := range a.actions {
		ats = append(ats, b.materialize(a.workspace.ID))
	}
	dss := make([]types.Datasource, 0, len(a.datasources))
	for _, ds := range a.datasources {
		dss = append(dss, ds)
	}
	ag := make([]types.Agent, 0, len(a.agents)+len(a.agentBuilders))
	ag = append(ag, a.agents...)
	for _, b := range a.agentBuilders {
		ag = append(ag, b.materialize(a.workspace.ID))
	}
	cts := make([]types.CustomTool, 0, len(a.customTools))
	for _, b := range a.customTools {
		cts = append(cts, b.materialize(a.workspace.ID))
	}
	assets := make([]types.Asset, 0, len(a.assets))
	for _, b := range a.assets {
		assets = append(assets, b.materialize(a.workspace.ID))
	}
	return &types.Ontology{
		Workspace:   a.workspace,
		Datasources: dss,
		ObjectTypes: ots,
		LinkTypes:   lts,
		ActionTypes: ats,
		Roles:       append([]types.Role(nil), a.roles...),
		PolicyRules: append([]types.PolicyRule(nil), a.policies...),
		Agents:      ag,
		CustomTools: cts,
		Assets:      assets,
	}
}

// Snapshot materializes the current in-memory ontology. This is primarily
// for bindings (FFI) and advanced workflows that need a snapshot without
// constructing the full HTTP handler.
func (a *App) Snapshot() *types.Ontology {
	return a.snapshot()
}

// AuthFunc resolves an http.Request into a logged-in Actor. Return an error
// to reject the request.
type AuthFunc func(r *http.Request) (types.Actor, error)

// devAuth grants every request an "admin" actor. Default in dev mode.
func devAuth(_ *http.Request) (types.Actor, error) {
	return types.Actor{UserID: "dev", WorkspaceID: "default", Roles: []string{"admin"}}, nil
}

// Option configures an App at construction time.
type Option func(*App)

// WithLogger sets the slog logger used by audit and middleware.
func WithLogger(l *slog.Logger) Option { return func(a *App) { a.logger = l } }

// WithWorkspace overrides the default workspace metadata.
func WithWorkspace(apiName, displayName string) Option {
	return func(a *App) {
		a.workspace.APIName = types.APIName(apiName)
		a.workspace.DisplayName = displayName
	}
}

// WithAuth registers an authentication function. Replaces the dev no-op.
func WithAuth(fn AuthFunc) Option { return func(a *App) { a.authenticator = fn } }

// WithAuditSink replaces the default stdout-via-slog sink.
func WithAuditSink(s audit.Sink) Option { return func(a *App) { a.auditSink = s } }

// WithEventBus overrides the default in-memory event bus. Use to plug in
// a Kafka / NATS / Redis Streams implementation.
func WithEventBus(b events.Bus) Option { return func(a *App) { a.bus = b } }

// WithBranchStore overrides the default in-memory branch store.
func WithBranchStore(s branch.Store) Option {
	return func(a *App) { a.branches = branch.NewManager(s) }
}

// Agent registers a new agent and returns a fluent builder.
func (a *App) Agent(apiName string) *AgentBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	b := &AgentBuilder{app: a, apiName: types.APIName(apiName)}
	a.agentBuilders = append(a.agentBuilders, b)
	return b
}

// CustomTool registers a new custom tool and returns a fluent builder.
func (a *App) CustomTool(apiName string) *CustomToolBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	b := &CustomToolBuilder{app: a, apiName: types.APIName(apiName)}
	a.customTools = append(a.customTools, b)
	return b
}

// Asset registers a new asset and returns a fluent builder.
func (a *App) Asset(apiName string) *AssetBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	b := &AssetBuilder{app: a, apiName: types.APIName(apiName)}
	a.assets = append(a.assets, b)
	return b
}

// Datasource registers a named datasource backed by b. The same datasource
// may host multiple object types via DatasourceBuilder.Bind.
func (a *App) Datasource(apiName string, b backend.Backend) *DatasourceBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.backends.Register(b)
	ds := types.Datasource{
		ID:          types.DatasourceID(ids.NewULID()),
		WorkspaceID: a.workspace.ID,
		APIName:     types.APIName(apiName),
		AdapterType: b.Type(),
	}
	a.datasources[apiName] = ds
	return &DatasourceBuilder{app: a, name: apiName}
}

// Allow / Deny start a policy rule for the given roles.
func (a *App) Allow(roles ...string) *PolicyBuilder {
	return &PolicyBuilder{app: a, effect: types.PolicyEffectAllow, roles: roles}
}

func (a *App) Deny(roles ...string) *PolicyBuilder {
	return &PolicyBuilder{app: a, effect: types.PolicyEffectDeny, roles: roles}
}

// Role declares a role. Optional; policies reference role api_names directly.
func (a *App) Role(apiName string, inherits ...string) {
	a.mu.Lock()
	defer a.mu.Unlock()
	in := make([]types.APIName, 0, len(inherits))
	for _, i := range inherits {
		in = append(in, types.APIName(i))
	}
	a.roles = append(a.roles, types.Role{
		ID: types.RoleID(ids.NewULID()), WorkspaceID: a.workspace.ID,
		APIName: types.APIName(apiName), Inherits: in,
	})
}

// errMissingFeature is returned when the user calls a method that requires
// a feature that wasn't enabled at construction time.
var errMissingFeature = errors.New("lattice: feature not enabled")

// slogSink is the default Sink that emits audit records as slog Info entries.
type slogSink struct{ logger *slog.Logger }

func newSlogSink(l *slog.Logger) *slogSink { return &slogSink{logger: l} }

func cloneWorkflowState(in workflow.State) workflow.State {
	if in == nil {
		return workflow.State{}
	}
	out := make(workflow.State, len(in))
	for k, v := range in {
		out[k] = v
	}
	return out
}

func (s *slogSink) Write(_ context.Context, batch []types.AuditRecord) error {
	for _, r := range batch {
		s.logger.Info("audit",
			"workspace_id", r.WorkspaceID,
			"actor", r.ActorUserID,
			"operation", r.Operation,
			"resource_kind", r.ResourceKind,
			"resource", r.ResourceAPIName,
			"decision", r.PolicyDecision,
			"duration_ms", r.DurationMS,
		)
	}
	return nil
}
