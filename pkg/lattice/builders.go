// Fluent builders for ObjectType, LinkType, ActionType, Policy, Role, and Datasource.
// Types are returned by App.ObjectType, App.LinkType, App.Action, etc.

package lattice

import (
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/backend"
	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// SearchFunc is the closure shape registered via ObjectTypeBuilder.Search.
type SearchFunc = backend.SearchFunc

// GetFunc is the closure shape registered via ObjectTypeBuilder.Get.
type GetFunc = backend.GetFunc

// MutateFunc is the closure shape registered via ObjectTypeBuilder.Mutate.
type MutateFunc = backend.MutateFunc

// ---------------------------------------------------------------------
// ObjectType builder
// ---------------------------------------------------------------------

// ObjectType registers a new object type and returns a fluent builder.
func (a *App) ObjectType(apiName string) *ObjectTypeBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	b := &ObjectTypeBuilder{
		app:     a,
		apiName: apiName,
	}
	a.objectTypes = append(a.objectTypes, b)
	return b
}

// ObjectTypeBuilder accumulates the configuration of a single object type.
type ObjectTypeBuilder struct {
	app *App

	apiName     string
	displayName string
	description string
	primaryKey  string
	properties  []propertyState

	// Source binding — either a closure (inline backend) or a registered datasource.
	dsName       string
	tableName    string
	schemaName   string
	inlineSearch SearchFunc
	inlineGet    GetFunc
	inlineMutate MutateFunc

	currentProp int // index of the most-recently-added property
}

type propertyState struct {
	prop       types.Property
	primaryKey bool
}

// DisplayName sets the human-readable name.
func (b *ObjectTypeBuilder) DisplayName(s string) *ObjectTypeBuilder {
	b.displayName = s
	return b
}

// Description sets the documentation string.
func (b *ObjectTypeBuilder) Description(s string) *ObjectTypeBuilder {
	b.description = s
	return b
}

// Property declares a typed property.
func (b *ObjectTypeBuilder) Property(name string, dt types.DataType) *ObjectTypeBuilder {
	b.properties = append(b.properties, propertyState{
		prop: types.Property{APIName: types.APIName(name), DataType: dt},
	})
	b.currentProp = len(b.properties) - 1
	return b
}

// PrimaryKey marks the most-recently-declared property as the primary key.
func (b *ObjectTypeBuilder) PrimaryKey() *ObjectTypeBuilder {
	if b.currentProp < 0 || b.currentProp >= len(b.properties) {
		b.app.addError(fmt.Errorf("ObjectType %q: PrimaryKey() called with no Property", b.apiName))
		return b
	}
	b.properties[b.currentProp].primaryKey = true
	b.primaryKey = string(b.properties[b.currentProp].prop.APIName)
	return b
}

// Tag attaches a tag to the most-recently-declared property.
func (b *ObjectTypeBuilder) Tag(tag string) *ObjectTypeBuilder {
	if b.currentProp < 0 {
		return b
	}
	p := &b.properties[b.currentProp].prop
	p.Tags = append(p.Tags, tag)
	return b
}

// Metadata attaches arbitrary metadata to the most-recently-declared property.
func (b *ObjectTypeBuilder) Metadata(key string, value any) *ObjectTypeBuilder {
	if b.currentProp < 0 {
		return b
	}
	p := &b.properties[b.currentProp].prop
	if p.Metadata == nil {
		p.Metadata = make(map[string]any)
	}
	p.Metadata[key] = value
	return b
}

// Indexed marks the most-recently-declared property as indexable.
func (b *ObjectTypeBuilder) Indexed() *ObjectTypeBuilder {
	if b.currentProp < 0 {
		return b
	}
	b.properties[b.currentProp].prop.Indexed = true
	return b
}

// Marking attaches a security marking to the most-recently-declared property.
func (b *ObjectTypeBuilder) Marking(markings ...string) *ObjectTypeBuilder {
	if b.currentProp < 0 {
		return b
	}
	b.properties[b.currentProp].prop.Markings = append(
		b.properties[b.currentProp].prop.Markings, markings...,
	)
	return b
}

// Nullable marks the most-recently-declared property as nullable.
func (b *ObjectTypeBuilder) Nullable() *ObjectTypeBuilder {
	if b.currentProp < 0 {
		return b
	}
	b.properties[b.currentProp].prop.Nullable = true
	return b
}

// AllowedValues constrains the property's value space.
func (b *ObjectTypeBuilder) AllowedValues(values ...string) *ObjectTypeBuilder {
	if b.currentProp < 0 {
		return b
	}
	b.properties[b.currentProp].prop.AllowedValues = values
	return b
}

// DefaultValue sets an explicit default for the current property.
func (b *ObjectTypeBuilder) DefaultValue(v any) *ObjectTypeBuilder {
	if b.currentProp < 0 {
		return b
	}
	b.properties[b.currentProp].prop.DefaultValue = v
	return b
}

// Transform appends a declarative transform to the current property.
func (b *ObjectTypeBuilder) Transform(kind string, config map[string]any) *ObjectTypeBuilder {
	if b.currentProp < 0 {
		return b
	}
	b.properties[b.currentProp].prop.Transforms = append(b.properties[b.currentProp].prop.Transforms, types.PropertyTransform{
		Kind:   kind,
		Config: config,
	})
	return b
}

// Computed marks the current property as derived from an explicit expression.
func (b *ObjectTypeBuilder) Computed(expression string, dependsOn ...string) *ObjectTypeBuilder {
	if b.currentProp < 0 {
		return b
	}
	deps := make([]types.APIName, 0, len(dependsOn))
	for _, dep := range dependsOn {
		deps = append(deps, types.APIName(dep))
	}
	b.properties[b.currentProp].prop.Computed = &types.ComputedProperty{
		Expression: expression,
		DependsOn:  deps,
	}
	return b
}

// Source binds this object type to a registered datasource and table.
func (b *ObjectTypeBuilder) Source(datasource, table string) *ObjectTypeBuilder {
	b.dsName = datasource
	b.tableName = table
	return b
}

// Search registers an inline closure as the Search backend for this object type.
func (b *ObjectTypeBuilder) Search(fn SearchFunc) *ObjectTypeBuilder {
	b.inlineSearch = fn
	return b
}

// Get registers an inline closure as the Get backend for this object type.
func (b *ObjectTypeBuilder) Get(fn GetFunc) *ObjectTypeBuilder {
	b.inlineGet = fn
	return b
}

// Mutate registers an inline closure as the Mutator for this object type.
func (b *ObjectTypeBuilder) Mutate(fn MutateFunc) *ObjectTypeBuilder {
	b.inlineMutate = fn
	return b
}

// APIName returns the api_name of the object type being built.
func (b *ObjectTypeBuilder) APIName() string { return b.apiName }

// InlineClosures returns the registered closures and true if any were set.
func (b *ObjectTypeBuilder) InlineClosures() (search backend.SearchFunc, get backend.GetFunc, mutate backend.MutateFunc, ok bool) {
	ok = b.inlineSearch != nil || b.inlineGet != nil || b.inlineMutate != nil
	return b.inlineSearch, b.inlineGet, b.inlineMutate, ok
}

// materialize converts the builder into a types.ObjectType.
func (b *ObjectTypeBuilder) materialize(ws types.WorkspaceID) types.ObjectType {
	props := make([]types.Property, 0, len(b.properties))
	pk := b.primaryKey
	for _, ps := range b.properties {
		props = append(props, ps.prop)
		if ps.primaryKey && pk == "" {
			pk = string(ps.prop.APIName)
		}
	}
	src := types.SourceConfig{
		DatasourceAPIName: types.APIName(b.dsName),
		Schema:            b.schemaName,
		Table:             b.tableName,
	}
	if src.Table == "" && b.dsName == "" {
		src.DatasourceAPIName = types.APIName(b.apiName + "_inline")
		src.Table = b.apiName
	}
	return types.ObjectType{
		ID:          types.ObjectTypeID(ids.NewULID()),
		WorkspaceID: ws,
		APIName:     types.APIName(b.apiName),
		DisplayName: b.displayName,
		Description: b.description,
		PrimaryKey:  types.APIName(pk),
		Source:      src,
		Properties:  props,
	}
}

// ---------------------------------------------------------------------
// LinkType builder
// ---------------------------------------------------------------------

// LinkType registers a directed relationship between two object types.
func (a *App) LinkType(apiName string) *LinkTypeBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	b := &LinkTypeBuilder{app: a, apiName: apiName}
	a.linkTypes = append(a.linkTypes, b)
	return b
}

// LinkTypeBuilder accumulates one link type's configuration.
type LinkTypeBuilder struct {
	app                      *App
	apiName, from, to        string
	cardinality              string
	fromProperty, toProperty string
}

// From sets the source object type api_name.
func (b *LinkTypeBuilder) From(name string) *LinkTypeBuilder { b.from = name; return b }

// To sets the target object type api_name.
func (b *LinkTypeBuilder) To(name string) *LinkTypeBuilder { b.to = name; return b }

// OneToMany sets cardinality and the join columns.
func (b *LinkTypeBuilder) OneToMany(fromProp, toProp string) *LinkTypeBuilder {
	b.cardinality = string(types.CardinalityOneToMany)
	b.fromProperty, b.toProperty = fromProp, toProp
	return b
}

// OneToOne is the cardinality variant.
func (b *LinkTypeBuilder) OneToOne(fromProp, toProp string) *LinkTypeBuilder {
	b.cardinality = string(types.CardinalityOneToOne)
	b.fromProperty, b.toProperty = fromProp, toProp
	return b
}

func (b *LinkTypeBuilder) materialize(ws types.WorkspaceID) types.LinkType {
	return types.LinkType{
		ID:             types.LinkTypeID(ids.NewULID()),
		WorkspaceID:    ws,
		APIName:        types.APIName(b.apiName),
		FromObjectType: types.APIName(b.from),
		ToObjectType:   types.APIName(b.to),
		Cardinality:    types.Cardinality(b.cardinality),
		PropertyMappings: []types.PropertyMapping{{
			FromProperty: types.APIName(b.fromProperty),
			ToProperty:   types.APIName(b.toProperty),
		}},
	}
}

// ---------------------------------------------------------------------
// ActionType builder
// ---------------------------------------------------------------------

// Action declares a typed mutation. Slim builder — full action surface comes
// when users need it.
func (a *App) Action(apiName string) *ActionBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	b := &ActionBuilder{app: a, apiName: apiName, executionMode: string(types.ExecutionModeSync)}
	a.actions = append(a.actions, b)
	return b
}

// ActionBuilder accumulates an action type's configuration.
type ActionBuilder struct {
	app                                                  *App
	apiName, displayName, subject                        string
	description                                          string
	permissionKey, idempotencyKeyTemplate, executionMode string
	inputSchema, outputSchema                            []byte
	handlerKind                                          string
	webhookURL, webhookSigningSecretRef                  string
	webhookTimeoutSeconds, webhookMaxRetries             int
	webhookRetryOnStatus                                 []int
	webhookHeaderForwards                                []string
	webhookBackoffInitialMS, webhookBackoffMaxMS         int
	webhookBackoffJitter                                 float64
	crudMappings                                         []types.CRUDMapping
	compositeSteps                                       []types.CompositeStep
}

// DisplayName sets the human-readable name.
func (b *ActionBuilder) DisplayName(s string) *ActionBuilder { b.displayName = s; return b }

// Description sets the docs string.
func (b *ActionBuilder) Description(s string) *ActionBuilder { b.description = s; return b }

// Subject names the object type the action operates on.
func (b *ActionBuilder) Subject(name string) *ActionBuilder { b.subject = name; return b }

// PermissionKey is required for policy evaluation.
func (b *ActionBuilder) PermissionKey(k string) *ActionBuilder { b.permissionKey = k; return b }

// InputSchema attaches a JSON Schema as a raw map.
func (b *ActionBuilder) InputSchema(schema map[string]any) *ActionBuilder {
	raw, _ := json.Marshal(schema)
	b.inputSchema = raw
	return b
}

// OutputSchema attaches a JSON Schema as a raw map.
func (b *ActionBuilder) OutputSchema(schema map[string]any) *ActionBuilder {
	raw, _ := json.Marshal(schema)
	b.outputSchema = raw
	return b
}

// IdempotencyKeyTemplate configures idempotency semantics.
func (b *ActionBuilder) IdempotencyKeyTemplate(tpl string) *ActionBuilder {
	b.idempotencyKeyTemplate = tpl
	return b
}

// Callback marks the action as dispatched via an FFI callback.
func (b *ActionBuilder) Callback() *ActionBuilder {
	b.handlerKind = string(types.HandlerKindCallback)
	return b
}

// ExecutionMode sets the action execution mode.
func (b *ActionBuilder) ExecutionMode(mode types.ExecutionMode) *ActionBuilder {
	b.executionMode = string(mode)
	return b
}

// Webhook configures an HTTP webhook handler.
func (b *ActionBuilder) Webhook(url string, signingSecretRef string) *ActionBuilder {
	b.handlerKind = string(types.HandlerKindWebhook)
	b.webhookURL = url
	b.webhookSigningSecretRef = signingSecretRef
	return b
}

// WebhookOptions extends webhook dispatch configuration.
func (b *ActionBuilder) WebhookOptions(timeoutSeconds, maxRetries int, retryOnStatus []int, headerForwards []string, backoffInitialMS, backoffMaxMS int, backoffJitter float64) *ActionBuilder {
	b.webhookTimeoutSeconds = timeoutSeconds
	b.webhookMaxRetries = maxRetries
	b.webhookRetryOnStatus = append([]int(nil), retryOnStatus...)
	b.webhookHeaderForwards = append([]string(nil), headerForwards...)
	b.webhookBackoffInitialMS = backoffInitialMS
	b.webhookBackoffMaxMS = backoffMaxMS
	b.webhookBackoffJitter = backoffJitter
	return b
}

// CRUD configures a declarative CRUD handler.
func (b *ActionBuilder) CRUD(kind types.HandlerKind, mappings ...types.CRUDMapping) *ActionBuilder {
	b.handlerKind = string(kind)
	b.crudMappings = append([]types.CRUDMapping(nil), mappings...)
	return b
}

// Composite configures an action as a sequence of steps.
func (b *ActionBuilder) Composite(steps ...types.CompositeStep) *ActionBuilder {
	b.handlerKind = string(types.HandlerKindComposite)
	b.compositeSteps = append([]types.CompositeStep(nil), steps...)
	return b
}

func (b *ActionBuilder) materialize(ws types.WorkspaceID) types.ActionType {
	at := types.ActionType{
		ID:                     types.ActionTypeID(ids.NewULID()),
		WorkspaceID:            ws,
		APIName:                types.APIName(b.apiName),
		DisplayName:            b.displayName,
		Description:            b.description,
		Subject:                types.APIName(b.subject),
		InputSchema:            b.inputSchema,
		OutputSchema:           b.outputSchema,
		PermissionKey:          b.permissionKey,
		IdempotencyKeyTemplate: b.idempotencyKeyTemplate,
		ExecutionMode:          types.ExecutionMode(b.executionMode),
		Handler:                types.HandlerConfig{Kind: types.HandlerKind(b.handlerKind)},
	}
	if b.handlerKind == string(types.HandlerKindWebhook) {
		at.Handler.Webhook = &types.WebhookHandler{
			URL:              b.webhookURL,
			TimeoutSeconds:   b.webhookTimeoutSeconds,
			MaxRetries:       b.webhookMaxRetries,
			SigningSecretRef: b.webhookSigningSecretRef,
			RetryOnStatus:    b.webhookRetryOnStatus,
			HeaderForwards:   b.webhookHeaderForwards,
			BackoffInitialMS: b.webhookBackoffInitialMS,
			BackoffMaxMS:     b.webhookBackoffMaxMS,
			BackoffJitter:    b.webhookBackoffJitter,
		}
	} else if len(b.crudMappings) > 0 {
		at.Handler.CRUD = &types.CRUDHandler{Mappings: b.crudMappings}
	} else if len(b.compositeSteps) > 0 {
		at.Handler.Composite = &types.CompositeHandler{Steps: b.compositeSteps}
	}
	return at
}

// ---------------------------------------------------------------------
// Policy builder
// ---------------------------------------------------------------------

// PolicyBuilder accumulates one policy rule. Terminal call is implicit:
// the rule is registered when the receiving methods complete.
type PolicyBuilder struct {
	app        *App
	effect     types.PolicyEffect
	roles      []string
	objectType string
	actionType string
	operations []types.Operation
	redactions []string
	rowFilter  types.Filter
}

// On scopes the rule to a single object type.
func (b *PolicyBuilder) On(objectType string) *PolicyBuilder {
	b.objectType = objectType
	return b
}

// OnAction scopes the rule to a single action type.
func (b *PolicyBuilder) OnAction(actionType string) *PolicyBuilder {
	b.actionType = actionType
	return b
}

// Operations sets the covered operations and registers the rule.
func (b *PolicyBuilder) Operations(ops ...types.Operation) *PolicyBuilder {
	b.operations = ops
	b.flush()
	return b
}

// All grants every operation. Terminal: registers the rule.
func (b *PolicyBuilder) All() *PolicyBuilder {
	return b.Operations(types.OperationRead, types.OperationSearch, types.OperationAggregate, types.OperationTraverse,
		types.OperationCreate, types.OperationUpdate, types.OperationDelete, types.OperationExecute)
}

// Read grants the read-only operation set (Read, Search, Aggregate, Traverse).
func (b *PolicyBuilder) Read() *PolicyBuilder {
	return b.Operations(types.OperationRead, types.OperationSearch, types.OperationAggregate, types.OperationTraverse)
}

// Write grants the write operation set (Create, Update, Delete, Execute).
func (b *PolicyBuilder) Write() *PolicyBuilder {
	return b.Operations(types.OperationCreate, types.OperationUpdate, types.OperationDelete, types.OperationExecute)
}

// Redact adds property names to redact in responses.
func (b *PolicyBuilder) Redact(properties ...string) *PolicyBuilder {
	b.redactions = append(b.redactions, properties...)
	b.flush()
	return b
}

// Filter sets an extra row filter the rule appends to every query.
func (b *PolicyBuilder) Filter(f types.Filter) *PolicyBuilder {
	b.rowFilter = f
	b.flush()
	return b
}

// flush materializes the in-progress rule into the App.
func (b *PolicyBuilder) flush() {
	b.app.mu.Lock()
	defer b.app.mu.Unlock()
	roles := make([]types.APIName, 0, len(b.roles))
	for _, r := range b.roles {
		roles = append(roles, types.APIName(r))
	}
	reds := make([]types.APIName, 0, len(b.redactions))
	for _, r := range b.redactions {
		reds = append(reds, types.APIName(r))
	}
	rule := types.PolicyRule{
		ID:          types.PolicyRuleID(ids.NewULID()),
		WorkspaceID: b.app.workspace.ID,
		APIName:     types.APIName(b.synthName()),
		Effect:      b.effect,
		Roles:       roles,
		Operations:  append([]types.Operation(nil), b.operations...),
		ObjectType:  types.APIName(b.objectType),
		ActionType:  types.APIName(b.actionType),
		Redactions:  reds,
		RowFilter:   b.rowFilter,
	}
	for i, existing := range b.app.policies {
		if existing.APIName == rule.APIName {
			b.app.policies[i] = rule
			return
		}
	}
	b.app.policies = append(b.app.policies, rule)
}

func (b *PolicyBuilder) synthName() string {
	out := string(b.effect)
	for _, r := range b.roles {
		out += "_" + r
	}
	if b.objectType != "" {
		out += "_on_" + b.objectType
	}
	if b.actionType != "" {
		out += "_action_" + b.actionType
	}
	return out
}

// ---------------------------------------------------------------------
// Datasource builder
// ---------------------------------------------------------------------

// DatasourceBuilder is returned by App.Datasource. Reserved for future
// methods (Configure, OverrideTable, etc.).
type DatasourceBuilder struct {
	app  *App
	name string
}

// Name returns the api_name of the datasource.
func (b *DatasourceBuilder) Name() string { return b.name }

// Config merges adapter-specific config into the datasource declaration.
func (b *DatasourceBuilder) Config(values map[string]any) *DatasourceBuilder {
	b.app.mu.Lock()
	defer b.app.mu.Unlock()
	ds := b.app.datasources[b.name]
	if ds.Config == nil {
		ds.Config = make(types.ConfigMap)
	}
	for k, v := range values {
		ds.Config[k] = v
	}
	b.app.datasources[b.name] = ds
	return b
}

// ---------------------------------------------------------------------
// Agent builder
// ---------------------------------------------------------------------

// AgentBuilder accumulates the configuration of a single agent.
type AgentBuilder struct {
	app             *App
	apiName         types.APIName
	displayName     string
	description     string
	systemPrompt    string
	model           types.ModelConfig
	fromObjectTypes []types.APIName
	fromLinkTypes   []types.APIName
	fromActions     []types.APIName
	customTools     []types.APIName
	contextSources  []types.AgentContextSource
	memory          types.AgentMemoryConfig
	planning        types.AgentPlanningConfig
	compaction      types.AgentCompactionConfig
	subagents       types.AgentSubagentConfig
	communication   types.AgentCommunicationConfig
	allowedRoles    []types.APIName
	limits          types.AgentLimits
	requireApproval bool
}

// DisplayName sets the human-readable name.
func (b *AgentBuilder) DisplayName(s string) *AgentBuilder { b.displayName = s; return b }

// Description sets the docs string.
func (b *AgentBuilder) Description(s string) *AgentBuilder { b.description = s; return b }

// SystemPrompt sets the system prompt.
func (b *AgentBuilder) SystemPrompt(s string) *AgentBuilder {
	b.systemPrompt = s
	return b
}

// Model sets the model configuration.
func (b *AgentBuilder) Model(provider, model string, temperature float64, maxTokens int) *AgentBuilder {
	b.model = types.ModelConfig{
		Provider:    provider,
		Model:       model,
		Temperature: temperature,
		MaxTokens:   maxTokens,
	}
	return b
}

// FromObjectTypes sets the object types the agent can access.
func (b *AgentBuilder) FromObjectTypes(names ...string) *AgentBuilder {
	for _, n := range names {
		b.fromObjectTypes = append(b.fromObjectTypes, types.APIName(n))
	}
	return b
}

// FromLinkTypes sets the link types the agent can traverse.
func (b *AgentBuilder) FromLinkTypes(names ...string) *AgentBuilder {
	for _, n := range names {
		b.fromLinkTypes = append(b.fromLinkTypes, types.APIName(n))
	}
	return b
}

// FromActions sets the action types the agent can execute.
func (b *AgentBuilder) FromActions(names ...string) *AgentBuilder {
	for _, n := range names {
		b.fromActions = append(b.fromActions, types.APIName(n))
	}
	return b
}

// CustomTools sets the custom tools available to the agent.
func (b *AgentBuilder) CustomTools(names ...string) *AgentBuilder {
	for _, n := range names {
		b.customTools = append(b.customTools, types.APIName(n))
	}
	return b
}

// ContextSource appends one runtime context source.
func (b *AgentBuilder) ContextSource(src types.AgentContextSource) *AgentBuilder {
	b.contextSources = append(b.contextSources, src)
	return b
}

// Memory sets the memory retrieval and persistence policy.
func (b *AgentBuilder) Memory(cfg types.AgentMemoryConfig) *AgentBuilder {
	b.memory = cfg
	return b
}

// Planning sets the plan generation/update policy.
func (b *AgentBuilder) Planning(cfg types.AgentPlanningConfig) *AgentBuilder {
	b.planning = cfg
	return b
}

// Compaction sets the long-horizon compaction policy.
func (b *AgentBuilder) Compaction(cfg types.AgentCompactionConfig) *AgentBuilder {
	b.compaction = cfg
	return b
}

// Subagents enables delegation to other agents.
func (b *AgentBuilder) Subagents(cfg types.AgentSubagentConfig) *AgentBuilder {
	b.subagents = cfg
	return b
}

// Communication sets the agent communication channels metadata.
func (b *AgentBuilder) Communication(cfg types.AgentCommunicationConfig) *AgentBuilder {
	b.communication = cfg
	return b
}

// AllowedRoles restricts who may invoke the agent.
func (b *AgentBuilder) AllowedRoles(names ...string) *AgentBuilder {
	for _, n := range names {
		b.allowedRoles = append(b.allowedRoles, types.APIName(n))
	}
	return b
}

// Limits sets the agent resource limits.
func (b *AgentBuilder) Limits(l types.AgentLimits) *AgentBuilder {
	b.limits = l
	return b
}

// RequireApprovalForActions forces action calls through an approval gate.
func (b *AgentBuilder) RequireApprovalForActions() *AgentBuilder {
	b.requireApproval = true
	return b
}

func (b *AgentBuilder) materialize(ws types.WorkspaceID) types.Agent {
	return types.Agent{
		ID:                        types.AgentID(ids.NewULID()),
		WorkspaceID:               ws,
		APIName:                   b.apiName,
		DisplayName:               b.displayName,
		Description:               b.description,
		SystemPrompt:              b.systemPrompt,
		Model:                     b.model,
		FromObjectTypes:           b.fromObjectTypes,
		FromLinkTypes:             b.fromLinkTypes,
		FromActions:               b.fromActions,
		CustomTools:               b.customTools,
		ContextSources:            b.contextSources,
		Memory:                    b.memory,
		Planning:                  b.planning,
		Compaction:                b.compaction,
		Subagents:                 b.subagents,
		Communication:             b.communication,
		AllowedRoles:              b.allowedRoles,
		Limits:                    b.limits,
		RequireApprovalForActions: b.requireApproval,
	}
}

// ---------------------------------------------------------------------
// CustomTool builder
// ---------------------------------------------------------------------

// CustomToolBuilder accumulates the configuration of a single custom tool.
type CustomToolBuilder struct {
	app          *App
	apiName      types.APIName
	displayName  string
	description  string
	kind         types.CustomToolKind
	inputSchema  json.RawMessage
	outputSchema json.RawMessage
	sql          *types.SQLToolSpec
	webhook      *types.WebhookHandler
	composite    *types.CompositeHandler
}

// DisplayName sets the human-readable name.
func (b *CustomToolBuilder) DisplayName(s string) *CustomToolBuilder { b.displayName = s; return b }

// Description sets the docs string.
func (b *CustomToolBuilder) Description(s string) *CustomToolBuilder { b.description = s; return b }

// Kind sets the custom tool kind.
func (b *CustomToolBuilder) Kind(k types.CustomToolKind) *CustomToolBuilder {
	b.kind = k
	return b
}

// InputSchema sets the JSON input schema.
func (b *CustomToolBuilder) InputSchema(raw map[string]any) *CustomToolBuilder {
	b.inputSchema, _ = json.Marshal(raw)
	return b
}

// OutputSchema sets the JSON output schema.
func (b *CustomToolBuilder) OutputSchema(raw map[string]any) *CustomToolBuilder {
	b.outputSchema, _ = json.Marshal(raw)
	return b
}

// SQL configures a SQL-based custom tool.
func (b *CustomToolBuilder) SQL(datasource, statement string) *CustomToolBuilder {
	b.kind = types.CustomToolKindSQL
	b.sql = &types.SQLToolSpec{
		DatasourceAPIName: types.APIName(datasource),
		Statement:         statement,
	}
	return b
}

// Webhook configures a webhook-based custom tool.
func (b *CustomToolBuilder) Webhook(url, secret string) *CustomToolBuilder {
	b.kind = types.CustomToolKindWebhook
	b.webhook = &types.WebhookHandler{
		URL:              url,
		SigningSecretRef: secret,
	}
	return b
}

// Callback marks the tool as an FFI callback (Python/Node/Rust handler).
func (b *CustomToolBuilder) Callback() *CustomToolBuilder {
	b.kind = types.CustomToolKindCallback
	return b
}

// Composite configures the tool as a composite of other actions/tools.
func (b *CustomToolBuilder) Composite(steps ...types.CompositeStep) *CustomToolBuilder {
	b.kind = types.CustomToolKindComposite
	b.composite = &types.CompositeHandler{Steps: steps}
	return b
}

func (b *CustomToolBuilder) materialize(ws types.WorkspaceID) types.CustomTool {
	ct := types.CustomTool{
		ID:           types.CustomToolID(ids.NewULID()),
		WorkspaceID:  ws,
		APIName:      b.apiName,
		DisplayName:  b.displayName,
		Description:  b.description,
		Kind:         b.kind,
		InputSchema:  b.inputSchema,
		OutputSchema: b.outputSchema,
	}
	if b.kind == types.CustomToolKindSQL && b.sql != nil {
		ct.SQL = b.sql
	}
	if b.kind == types.CustomToolKindWebhook && b.webhook != nil {
		ct.Webhook = b.webhook
	}
	if b.kind == types.CustomToolKindComposite && b.composite != nil {
		ct.Composite = b.composite
	}
	return ct
}

// ---------------------------------------------------------------------
// Asset builder
// ---------------------------------------------------------------------

type AssetBuilder struct {
	app                  *App
	apiName              types.APIName
	displayName          string
	description          string
	metadata             map[string]any
	tags                 []string
	properties           []types.Property
	qualityRules         []types.QualityRule
	dependencies         []types.AssetDependency
	sink                 types.AssetSink
	savedColumnMapping   []types.ColumnMapping
	unmappedColumnPolicy string
	currentProp          int
}

// DisplayName sets the human-readable name.
func (b *AssetBuilder) DisplayName(s string) *AssetBuilder { b.displayName = s; return b }

// Description sets the docs string.
func (b *AssetBuilder) Description(s string) *AssetBuilder { b.description = s; return b }

// Metadata sets arbitrary metadata on the asset.
func (b *AssetBuilder) Metadata(key string, value any) *AssetBuilder {
	if b.metadata == nil {
		b.metadata = make(map[string]any)
	}
	b.metadata[key] = value
	return b
}

// Tag appends a tag to the asset.
func (b *AssetBuilder) Tag(tag string) *AssetBuilder {
	b.tags = append(b.tags, tag)
	return b
}

// Property declares an asset property.
func (b *AssetBuilder) Property(name string, dt types.DataType) *AssetBuilder {
	b.properties = append(b.properties, types.Property{APIName: types.APIName(name), DataType: dt})
	b.currentProp = len(b.properties) - 1
	return b
}

// Nullable marks the current property as nullable.
func (b *AssetBuilder) Nullable() *AssetBuilder {
	if b.currentProp >= 0 && b.currentProp < len(b.properties) {
		b.properties[b.currentProp].Nullable = true
	}
	return b
}

// SourceColumn binds the current property to a source column.
func (b *AssetBuilder) SourceColumn(name string) *AssetBuilder {
	if b.currentProp >= 0 && b.currentProp < len(b.properties) {
		b.properties[b.currentProp].SourceColumn = name
	}
	return b
}

// QualityRule appends a declarative quality rule.
func (b *AssetBuilder) QualityRule(rule types.QualityRule) *AssetBuilder {
	b.qualityRules = append(b.qualityRules, rule)
	return b
}

// DependsOn declares an asset dependency edge.
func (b *AssetBuilder) DependsOn(kind, target string) *AssetBuilder {
	b.dependencies = append(b.dependencies, types.AssetDependency{Kind: kind, Target: target})
	return b
}

// Sink declares where ingested data lands.
func (b *AssetBuilder) Sink(datasource, table string) *AssetBuilder {
	b.sink = types.AssetSink{DatasourceAPIName: types.APIName(datasource), Table: table}
	return b
}

// Schema sets the sink schema.
func (b *AssetBuilder) Schema(name string) *AssetBuilder {
	b.sink.Schema = name
	return b
}

// ColumnMapping appends one saved source->target mapping.
func (b *AssetBuilder) ColumnMapping(mapping types.ColumnMapping) *AssetBuilder {
	b.savedColumnMapping = append(b.savedColumnMapping, mapping)
	return b
}

// UnmappedColumnPolicy configures warn|error|ignore behavior.
func (b *AssetBuilder) UnmappedColumnPolicy(policy string) *AssetBuilder {
	b.unmappedColumnPolicy = policy
	return b
}

func (b *AssetBuilder) materialize(ws types.WorkspaceID) types.Asset {
	return types.Asset{
		ID:                   types.AssetID(ids.NewULID()),
		WorkspaceID:          ws,
		APIName:              b.apiName,
		DisplayName:          b.displayName,
		Description:          b.description,
		Metadata:             b.metadata,
		Tags:                 b.tags,
		Properties:           b.properties,
		QualityRules:         b.qualityRules,
		Dependencies:         b.dependencies,
		Sink:                 b.sink,
		SavedColumnMapping:   b.savedColumnMapping,
		UnmappedColumnPolicy: b.unmappedColumnPolicy,
	}
}
