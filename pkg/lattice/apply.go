// Spec-based registration. Bindings (Python/Node/Rust) introspect their
// class definitions, serialize a compact JSON spec, and apply it in one
// call. The spec format is documented in docs/06-extensibility/spec.md.

package lattice

import (
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/ids"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Spec is the JSON shape bindings emit. Unmarshal into this, walk it,
// replay against the App's fluent builders.
type Spec struct {
	Workspace   *specWorkspace   `json:"workspace,omitempty"`
	ObjectTypes []specObjectType `json:"object_types,omitempty"`
	LinkTypes   []specLinkType   `json:"link_types,omitempty"`
	ActionTypes []specActionType `json:"action_types,omitempty"`
	Policies    []specPolicy     `json:"policies,omitempty"`
	Roles       []specRole       `json:"roles,omitempty"`
	Datasources []specDatasource `json:"datasources,omitempty"`
	Agents      []specAgent      `json:"agents,omitempty"`
	CustomTools []specCustomTool `json:"custom_tools,omitempty"`
	Assets      []specAsset      `json:"assets,omitempty"`
}

type specWorkspace struct {
	APIName     string `json:"api_name"`
	DisplayName string `json:"display_name,omitempty"`
}

type specObjectType struct {
	APIName     string         `json:"api_name"`
	DisplayName string         `json:"display_name,omitempty"`
	Description string         `json:"description,omitempty"`
	Properties  []specProperty `json:"properties"`
	Source      *specSource    `json:"source,omitempty"`
}

type specProperty struct {
	APIName       string                    `json:"api_name"`
	DataType      string                    `json:"data_type"`
	PrimaryKey    bool                      `json:"primary_key,omitempty"`
	Indexed       bool                      `json:"indexed,omitempty"`
	Nullable      bool                      `json:"nullable,omitempty"`
	Tags          []string                  `json:"tags,omitempty"`
	Markings      []string                  `json:"markings,omitempty"`
	AllowedValues []string                  `json:"allowed_values,omitempty"`
	Metadata      map[string]any            `json:"metadata,omitempty"`
	DefaultValue  any                       `json:"default_value,omitempty"`
	Transforms    []types.PropertyTransform `json:"transforms,omitempty"`
	Computed      *types.ComputedProperty   `json:"computed,omitempty"`
}

type specSource struct {
	Datasource string `json:"datasource"`
	Schema     string `json:"schema,omitempty"`
	Table      string `json:"table"`
}

type specLinkType struct {
	APIName      string `json:"api_name"`
	From         string `json:"from"`
	To           string `json:"to"`
	Cardinality  string `json:"cardinality"`
	FromProperty string `json:"from_property"`
	ToProperty   string `json:"to_property"`
}

type specActionType struct {
	APIName                string                `json:"api_name"`
	DisplayName            string                `json:"display_name,omitempty"`
	Description            string                `json:"description,omitempty"`
	Subject                string                `json:"subject,omitempty"`
	PermissionKey          string                `json:"permission_key"`
	InputSchema            map[string]any        `json:"input_schema,omitempty"`
	OutputSchema           map[string]any        `json:"output_schema,omitempty"`
	IdempotencyKeyTemplate string                `json:"idempotency_key_template,omitempty"`
	HandlerKind            string                `json:"handler_kind,omitempty"`
	WebhookURL             string                `json:"webhook_url,omitempty"`
	WebhookSecret          string                `json:"webhook_secret,omitempty"`
	ExecutionMode          string                `json:"execution_mode,omitempty"`
	CRUDMappings           []types.CRUDMapping   `json:"crud_mappings,omitempty"`
	CompositeSteps         []types.CompositeStep `json:"composite_steps,omitempty"`
	WebhookOptions         map[string]any        `json:"webhook_options,omitempty"`
}

type specPolicy struct {
	Effect     string        `json:"effect"`
	Roles      []string      `json:"roles,omitempty"`
	On         string        `json:"on,omitempty"`
	OnAction   string        `json:"on_action,omitempty"`
	Operations []string      `json:"operations,omitempty"`
	Redactions []string      `json:"redactions,omitempty"`
	RowFilter  *types.Filter `json:"row_filter,omitempty"`
}

type specRole struct {
	APIName  string   `json:"api_name"`
	Inherits []string `json:"inherits,omitempty"`
}

type specDatasource struct {
	APIName     string         `json:"api_name"`
	AdapterType string         `json:"adapter_type"`
	Config      map[string]any `json:"config,omitempty"`
}

type specAgent struct {
	APIName                   string                         `json:"api_name"`
	DisplayName               string                         `json:"display_name,omitempty"`
	Description               string                         `json:"description,omitempty"`
	SystemPrompt              string                         `json:"system_prompt"`
	Model                     specModel                      `json:"model,omitempty"`
	FromObjectTypes           []string                       `json:"from_object_types,omitempty"`
	FromLinkTypes             []string                       `json:"from_link_types,omitempty"`
	FromActions               []string                       `json:"from_actions,omitempty"`
	CustomTools               []string                       `json:"custom_tools,omitempty"`
	ContextSources            []types.AgentContextSource     `json:"context_sources,omitempty"`
	Memory                    types.AgentMemoryConfig        `json:"memory,omitempty"`
	Planning                  types.AgentPlanningConfig      `json:"planning,omitempty"`
	Compaction                types.AgentCompactionConfig    `json:"compaction,omitempty"`
	Subagents                 types.AgentSubagentConfig      `json:"subagents,omitempty"`
	Communication             types.AgentCommunicationConfig `json:"communication,omitempty"`
	AllowedRoles              []string                       `json:"allowed_roles,omitempty"`
	Limits                    specAgentLimits                `json:"limits,omitempty"`
	RequireApprovalForActions bool                           `json:"require_approval_for_actions,omitempty"`
}

type specModel struct {
	Provider    string  `json:"provider"`
	Model       string  `json:"model"`
	Temperature float64 `json:"temperature,omitempty"`
	MaxTokens   int     `json:"max_tokens,omitempty"`
}

type specAgentLimits struct {
	MaxToolCalls   int     `json:"max_tool_calls,omitempty"`
	MaxTokens      int     `json:"max_tokens,omitempty"`
	MaxCostUSD     float64 `json:"max_cost_usd,omitempty"`
	TimeoutSeconds int     `json:"timeout_seconds,omitempty"`
}

type specCustomTool struct {
	APIName      string         `json:"api_name"`
	DisplayName  string         `json:"display_name,omitempty"`
	Description  string         `json:"description,omitempty"`
	Kind         string         `json:"kind"`
	InputSchema  map[string]any `json:"input_schema,omitempty"`
	OutputSchema map[string]any `json:"output_schema,omitempty"`
	SQL          *specSQLTool   `json:"sql,omitempty"`
	Webhook      *specWebhook   `json:"webhook,omitempty"`
	Composite    *specComposite `json:"composite,omitempty"`
}

type specSQLTool struct {
	Datasource string `json:"datasource"`
	Statement  string `json:"statement"`
}

type specWebhook struct {
	URL    string `json:"url"`
	Secret string `json:"secret,omitempty"`
}

type specComposite struct {
	Steps []types.CompositeStep `json:"steps,omitempty"`
}

type specAsset struct {
	APIName              string                  `json:"api_name"`
	DisplayName          string                  `json:"display_name,omitempty"`
	Description          string                  `json:"description,omitempty"`
	Metadata             map[string]any          `json:"metadata,omitempty"`
	Tags                 []string                `json:"tags,omitempty"`
	Properties           []specProperty          `json:"properties,omitempty"`
	QualityRules         []types.QualityRule     `json:"quality_rules,omitempty"`
	Dependencies         []types.AssetDependency `json:"dependencies,omitempty"`
	Sink                 *specAssetSink          `json:"sink,omitempty"`
	SavedColumnMapping   []types.ColumnMapping   `json:"saved_column_mapping,omitempty"`
	UnmappedColumnPolicy string                  `json:"unmapped_column_policy,omitempty"`
}

type specAssetSink struct {
	Datasource string `json:"datasource"`
	Schema     string `json:"schema,omitempty"`
	Table      string `json:"table"`
}

// ApplyJSON applies a JSON ontology spec by replaying it as builder calls.
// Designed for use by FFI bindings — Python/Node/Rust assemble the spec
// from their native class definitions and call this in a single FFI hop.
//
// Object-type callbacks (search/get/mutate) are NOT part of the spec; the
// binding registers them separately via the Find() lookup.
func (a *App) ApplyJSON(raw []byte) error {
	var spec Spec
	if err := json.Unmarshal(raw, &spec); err != nil {
		return fmt.Errorf("apply spec: %w", err)
	}
	return a.applySpec(spec)
}

func (a *App) applySpec(spec Spec) error {
	if spec.Workspace != nil {
		a.workspace.APIName = types.APIName(spec.Workspace.APIName)
		if spec.Workspace.DisplayName != "" {
			a.workspace.DisplayName = spec.Workspace.DisplayName
		}
	}
	for _, ot := range spec.ObjectTypes {
		a.applyObjectType(ot)
	}
	for _, lt := range spec.LinkTypes {
		a.applyLinkType(lt)
	}
	for _, at := range spec.ActionTypes {
		a.applyActionType(at)
	}
	for _, r := range spec.Roles {
		a.Role(r.APIName, r.Inherits...)
	}
	for _, p := range spec.Policies {
		a.applyPolicy(p)
	}
	for _, ds := range spec.Datasources {
		a.applyDatasource(ds)
	}
	for _, ag := range spec.Agents {
		a.applyAgent(ag)
	}
	for _, ct := range spec.CustomTools {
		a.applyCustomTool(ct)
	}
	for _, asset := range spec.Assets {
		a.applyAsset(asset)
	}
	return nil
}

func (a *App) applyObjectType(ot specObjectType) {
	b := a.ObjectType(ot.APIName)
	if ot.DisplayName != "" {
		b.DisplayName(ot.DisplayName)
	}
	if ot.Description != "" {
		b.Description(ot.Description)
	}
	if ot.Source != nil {
		b.Source(ot.Source.Datasource, ot.Source.Table)
	}
	for _, p := range ot.Properties {
		b.Property(p.APIName, types.DataType(p.DataType))
		if p.PrimaryKey {
			b.PrimaryKey()
		}
		if p.Indexed {
			b.Indexed()
		}
		if p.Nullable {
			b.Nullable()
		}
		for _, tag := range p.Tags {
			b.Tag(tag)
		}
		for key, value := range p.Metadata {
			b.Metadata(key, value)
		}
		if len(p.AllowedValues) > 0 {
			b.AllowedValues(p.AllowedValues...)
		}
		if p.DefaultValue != nil {
			b.DefaultValue(p.DefaultValue)
		}
		for _, transform := range p.Transforms {
			b.Transform(transform.Kind, transform.Config)
		}
		if p.Computed != nil {
			deps := make([]string, 0, len(p.Computed.DependsOn))
			for _, dep := range p.Computed.DependsOn {
				deps = append(deps, string(dep))
			}
			b.Computed(p.Computed.Expression, deps...)
		}
	}
}

func (a *App) applyLinkType(lt specLinkType) {
	b := a.LinkType(lt.APIName).From(lt.From).To(lt.To)
	switch lt.Cardinality {
	case "one_to_many":
		b.OneToMany(lt.FromProperty, lt.ToProperty)
	case "one_to_one":
		b.OneToOne(lt.FromProperty, lt.ToProperty)
	}
}

func (a *App) applyActionType(at specActionType) {
	b := a.Action(at.APIName)
	if at.DisplayName != "" {
		b.DisplayName(at.DisplayName)
	}
	if at.Description != "" {
		b.Description(at.Description)
	}
	if at.Subject != "" {
		b.Subject(at.Subject)
	}
	if at.PermissionKey != "" {
		b.PermissionKey(at.PermissionKey)
	}
	if at.InputSchema != nil {
		b.InputSchema(at.InputSchema)
	}
	if at.OutputSchema != nil {
		b.OutputSchema(at.OutputSchema)
	}
	if at.IdempotencyKeyTemplate != "" {
		b.IdempotencyKeyTemplate(at.IdempotencyKeyTemplate)
	}
	if at.ExecutionMode != "" {
		b.ExecutionMode(types.ExecutionMode(at.ExecutionMode))
	}
	if at.HandlerKind == "webhook" && at.WebhookURL != "" {
		b.Webhook(at.WebhookURL, at.WebhookSecret)
		if at.WebhookOptions != nil {
			b.WebhookOptions(
				intFromMap(at.WebhookOptions, "timeout_seconds"),
				intFromMap(at.WebhookOptions, "max_retries"),
				intSliceFromMap(at.WebhookOptions, "retry_on_status"),
				stringSliceFromMap(at.WebhookOptions, "header_forwards"),
				intFromMap(at.WebhookOptions, "backoff_initial_ms"),
				intFromMap(at.WebhookOptions, "backoff_max_ms"),
				floatFromMap(at.WebhookOptions, "backoff_jitter"),
			)
		}
	}
	if len(at.CRUDMappings) > 0 {
		b.CRUD(types.HandlerKind(at.HandlerKind), at.CRUDMappings...)
	}
	if len(at.CompositeSteps) > 0 {
		b.Composite(at.CompositeSteps...)
	}
	if at.HandlerKind == "callback" {
		b.handlerKind = string(types.HandlerKindCallback)
	}
}

func (a *App) applyPolicy(p specPolicy) {
	var pb *PolicyBuilder
	switch p.Effect {
	case "deny":
		pb = a.Deny(p.Roles...)
	default:
		pb = a.Allow(p.Roles...)
	}
	if p.On != "" {
		pb.On(p.On)
	}
	if p.OnAction != "" {
		pb.OnAction(p.OnAction)
	}
	if len(p.Operations) == 1 && p.Operations[0] == "*" {
		pb.All()
	} else {
		ops := make([]types.Operation, 0, len(p.Operations))
		for _, op := range p.Operations {
			ops = append(ops, types.Operation(op))
		}
		if len(ops) > 0 {
			pb.Operations(ops...)
		}
	}
	if len(p.Redactions) > 0 {
		pb.Redact(p.Redactions...)
	}
	if p.RowFilter != nil {
		pb.Filter(*p.RowFilter)
	}
}

func (a *App) applyDatasource(ds specDatasource) {
	// FFI bindings register backends via lattice_register_backend separately.
	// Here we only ensure the DatasourceBuilder exists in the App spec.
	if _, ok := a.datasources[ds.APIName]; !ok {
		// placeholder: backend must be registered via C ABI before serve.
		a.datasources[ds.APIName] = types.Datasource{
			ID:          types.DatasourceID(ids.NewULID()),
			WorkspaceID: a.workspace.ID,
			APIName:     types.APIName(ds.APIName),
			AdapterType: ds.AdapterType,
			Config:      types.ConfigMap(ds.Config),
		}
	}
}

func (a *App) applyAgent(ag specAgent) {
	b := a.Agent(ag.APIName)
	if ag.DisplayName != "" {
		b.DisplayName(ag.DisplayName)
	}
	if ag.Description != "" {
		b.Description(ag.Description)
	}
	if ag.SystemPrompt != "" {
		b.SystemPrompt(ag.SystemPrompt)
	}
	b.Model(ag.Model.Provider, ag.Model.Model, ag.Model.Temperature, ag.Model.MaxTokens)
	if len(ag.FromObjectTypes) > 0 {
		b.FromObjectTypes(ag.FromObjectTypes...)
	}
	if len(ag.FromLinkTypes) > 0 {
		b.FromLinkTypes(ag.FromLinkTypes...)
	}
	if len(ag.FromActions) > 0 {
		b.FromActions(ag.FromActions...)
	}
	if len(ag.CustomTools) > 0 {
		b.CustomTools(ag.CustomTools...)
	}
	for _, src := range ag.ContextSources {
		b.ContextSource(src)
	}
	if ag.Memory.Enabled || ag.Memory.Namespace != "" || ag.Memory.Scope != "" {
		b.Memory(ag.Memory)
	}
	if ag.Planning.Enabled || ag.Planning.Mode != "" || ag.Planning.GoalPrompt != "" {
		b.Planning(ag.Planning)
	}
	if ag.Compaction.Enabled || ag.Compaction.TriggerTokens > 0 {
		b.Compaction(ag.Compaction)
	}
	if ag.Subagents.Enabled || len(ag.Subagents.AgentRefs) > 0 {
		b.Subagents(ag.Subagents)
	}
	if len(ag.Communication.Channels) > 0 {
		b.Communication(ag.Communication)
	}
	if len(ag.AllowedRoles) > 0 {
		b.AllowedRoles(ag.AllowedRoles...)
	}
	b.Limits(types.AgentLimits{
		MaxToolCalls:   ag.Limits.MaxToolCalls,
		MaxTokens:      ag.Limits.MaxTokens,
		MaxCostUSD:     ag.Limits.MaxCostUSD,
		TimeoutSeconds: ag.Limits.TimeoutSeconds,
	})
	if ag.RequireApprovalForActions {
		b.RequireApprovalForActions()
	}
}

func (a *App) applyCustomTool(ct specCustomTool) {
	b := a.CustomTool(ct.APIName)
	if ct.DisplayName != "" {
		b.DisplayName(ct.DisplayName)
	}
	if ct.Description != "" {
		b.Description(ct.Description)
	}
	switch ct.Kind {
	case "sql":
		if ct.SQL != nil {
			b.SQL(ct.SQL.Datasource, ct.SQL.Statement)
		}
	case "webhook":
		if ct.Webhook != nil {
			b.Webhook(ct.Webhook.URL, ct.Webhook.Secret)
		}
	case "callback":
		b.Kind(types.CustomToolKindCallback)
	case "composite":
		if ct.Composite != nil {
			b.Composite(ct.Composite.Steps...)
		}
	}
	if ct.InputSchema != nil {
		b.InputSchema(ct.InputSchema)
	}
	if ct.OutputSchema != nil {
		b.OutputSchema(ct.OutputSchema)
	}
}

func (a *App) applyAsset(as specAsset) {
	b := a.Asset(as.APIName)
	if as.DisplayName != "" {
		b.DisplayName(as.DisplayName)
	}
	if as.Description != "" {
		b.Description(as.Description)
	}
	for key, value := range as.Metadata {
		b.Metadata(key, value)
	}
	for _, tag := range as.Tags {
		b.Tag(tag)
	}
	for _, prop := range as.Properties {
		b.Property(prop.APIName, types.DataType(prop.DataType))
		if prop.Nullable {
			b.Nullable()
		}
	}
	for _, rule := range as.QualityRules {
		b.QualityRule(rule)
	}
	for _, dep := range as.Dependencies {
		b.DependsOn(dep.Kind, dep.Target)
	}
	if as.Sink != nil {
		b.Sink(as.Sink.Datasource, as.Sink.Table)
		if as.Sink.Schema != "" {
			b.Schema(as.Sink.Schema)
		}
	}
	for _, mapping := range as.SavedColumnMapping {
		b.ColumnMapping(mapping)
	}
	if as.UnmappedColumnPolicy != "" {
		b.UnmappedColumnPolicy(as.UnmappedColumnPolicy)
	}
}

// FindObjectType returns the named ObjectTypeBuilder so callers (typically
// FFI bindings) can attach callbacks after applying a spec.
func (a *App) FindObjectType(apiName string) *ObjectTypeBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	for _, b := range a.objectTypes {
		if b.apiName == apiName {
			return b
		}
	}
	return nil
}

// FindAgent returns the named AgentBuilder.
func (a *App) FindAgent(apiName string) *AgentBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	for _, b := range a.agentBuilders {
		if string(b.apiName) == apiName {
			return b
		}
	}
	return nil
}

// FindCustomTool returns the named CustomToolBuilder.
func (a *App) FindCustomTool(apiName string) *CustomToolBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	for _, b := range a.customTools {
		if string(b.apiName) == apiName {
			return b
		}
	}
	return nil
}

// FindActionType returns the named ActionBuilder.
func (a *App) FindActionType(apiName string) *ActionBuilder {
	a.mu.Lock()
	defer a.mu.Unlock()
	for _, b := range a.actions {
		if b.apiName == apiName {
			return b
		}
	}
	return nil
}

func intFromMap(m map[string]any, key string) int {
	v, ok := m[key]
	if !ok {
		return 0
	}
	switch n := v.(type) {
	case int:
		return n
	case float64:
		return int(n)
	default:
		return 0
	}
}

func floatFromMap(m map[string]any, key string) float64 {
	v, ok := m[key]
	if !ok {
		return 0
	}
	switch n := v.(type) {
	case float64:
		return n
	case int:
		return float64(n)
	default:
		return 0
	}
}

func intSliceFromMap(m map[string]any, key string) []int {
	raw, ok := m[key]
	if !ok {
		return nil
	}
	items, ok := raw.([]any)
	if !ok {
		if typed, ok := raw.([]int); ok {
			return typed
		}
		return nil
	}
	out := make([]int, 0, len(items))
	for _, item := range items {
		switch n := item.(type) {
		case int:
			out = append(out, n)
		case float64:
			out = append(out, int(n))
		}
	}
	return out
}

func stringSliceFromMap(m map[string]any, key string) []string {
	raw, ok := m[key]
	if !ok {
		return nil
	}
	items, ok := raw.([]any)
	if !ok {
		if typed, ok := raw.([]string); ok {
			return typed
		}
		return nil
	}
	out := make([]string, 0, len(items))
	for _, item := range items {
		if s, ok := item.(string); ok {
			out = append(out, s)
		}
	}
	return out
}
