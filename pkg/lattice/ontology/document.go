// Declarative document parsing and serialization. The Document is the
// JSON user-visible shape, and Materialize converts it to a slice of core
// entities ready for the validator.

package ontology

import (
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Document is the declarative JSON ontology form.
type Document struct {
	APIVersion  string          `json:"api_version"`
	Workspace   workspaceDoc    `json:"workspace"`
	Datasources []datasourceDoc `json:"datasources,omitempty"`
	ObjectTypes []objectTypeDoc `json:"object_types,omitempty"`
	LinkTypes   []linkTypeDoc   `json:"link_types,omitempty"`
	ActionTypes []actionTypeDoc `json:"action_types,omitempty"`
	Roles       []roleDoc       `json:"roles,omitempty"`
	PolicyRules []policyRuleDoc `json:"policy_rules,omitempty"`
	CustomTools []customToolDoc `json:"custom_tools,omitempty"`
	Agents      []agentDoc      `json:"agents,omitempty"`
	Assets      []assetDoc      `json:"assets,omitempty"`
}

type workspaceDoc struct {
	APIName     string                  `json:"api_name"`
	DisplayName string                  `json:"display_name"`
	Description string                  `json:"description,omitempty"`
	Settings    types.WorkspaceSettings `json:"settings,omitempty"`
}

type datasourceDoc struct {
	APIName     string         `json:"api_name"`
	DisplayName string         `json:"display_name,omitempty"`
	AdapterType string         `json:"adapter_type"`
	Config      map[string]any `json:"config,omitempty"`
	Credentials map[string]any `json:"credentials,omitempty"` // sealed at apply time
}

type objectTypeDoc struct {
	APIName      string        `json:"api_name"`
	DisplayName  string        `json:"display_name,omitempty"`
	Description  string        `json:"description,omitempty"`
	PrimaryKey   string        `json:"primary_key"`
	Source       sourceDoc     `json:"source"`
	Properties   []propertyDoc `json:"properties"`
	Environments []string      `json:"environments,omitempty"`
}

type sourceDoc struct {
	Datasource string `json:"datasource"`
	Schema     string `json:"schema,omitempty"`
	Table      string `json:"table"`
}

type propertyDoc struct {
	APIName       string                    `json:"api_name"`
	DisplayName   string                    `json:"display_name,omitempty"`
	Description   string                    `json:"description,omitempty"`
	DataType      string                    `json:"data_type"`
	SourceColumn  string                    `json:"source_column,omitempty"`
	Nullable      bool                      `json:"nullable,omitempty"`
	Indexed       bool                      `json:"indexed,omitempty"`
	AllowedValues []string                  `json:"allowed_values,omitempty"`
	Tags          []string                  `json:"tags,omitempty"`
	Markings      []string                  `json:"markings,omitempty"`
	Metadata      map[string]any            `json:"metadata,omitempty"`
	DefaultValue  any                       `json:"default_value,omitempty"`
	Transforms    []types.PropertyTransform `json:"transforms,omitempty"`
	Computed      *types.ComputedProperty   `json:"computed,omitempty"`
}

type linkTypeDoc struct {
	APIName          string               `json:"api_name"`
	DisplayName      string               `json:"display_name,omitempty"`
	FromObjectType   string               `json:"from_object_type"`
	ToObjectType     string               `json:"to_object_type"`
	Cardinality      string               `json:"cardinality"`
	PropertyMappings []propertyMappingDoc `json:"property_mappings,omitempty"`
	Junction         *junctionDoc         `json:"junction,omitempty"`
}

type propertyMappingDoc struct {
	FromProperty string `json:"from_property"`
	ToProperty   string `json:"to_property"`
}

type junctionDoc struct {
	Datasource string   `json:"datasource"`
	Schema     string   `json:"schema,omitempty"`
	Table      string   `json:"table"`
	FromColumn string   `json:"from_column"`
	ToColumn   string   `json:"to_column"`
	Properties []string `json:"properties,omitempty"`
}

type actionTypeDoc struct {
	APIName                string         `json:"api_name"`
	DisplayName            string         `json:"display_name,omitempty"`
	Description            string         `json:"description,omitempty"`
	Subject                string         `json:"subject,omitempty"`
	InputSchema            map[string]any `json:"input_schema"`
	OutputSchema           map[string]any `json:"output_schema,omitempty"`
	PermissionKey          string         `json:"permission_key"`
	IdempotencyKeyTemplate string         `json:"idempotency_key_template,omitempty"`
	ExecutionMode          string         `json:"execution_mode,omitempty"`
	Handler                handlerDoc     `json:"handler"`
}

type handlerDoc struct {
	Kind      string        `json:"kind"`
	CRUD      *crudDoc      `json:"crud,omitempty"`
	Webhook   *webhookDoc   `json:"webhook,omitempty"`
	Composite *compositeDoc `json:"composite,omitempty"`
}

type crudDoc struct {
	Mappings []crudMappingDoc `json:"mappings"`
}

type crudMappingDoc struct {
	TargetProperty string `json:"target_property"`
	Expression     string `json:"expression"`
}

type webhookDoc struct {
	URL              string   `json:"url"`
	TimeoutSeconds   int      `json:"timeout_seconds,omitempty"`
	MaxRetries       int      `json:"max_retries,omitempty"`
	SigningSecretRef string   `json:"signing_secret_ref,omitempty"`
	RetryOnStatus    []int    `json:"retry_on_status,omitempty"`
	HeaderForwards   []string `json:"header_forwards,omitempty"`
	BackoffInitialMS int      `json:"backoff_initial_ms,omitempty"`
	BackoffMaxMS     int      `json:"backoff_max_ms,omitempty"`
	BackoffJitter    float64  `json:"backoff_jitter,omitempty"`
}

type compositeDoc struct {
	Steps []compositeStepDoc `json:"steps"`
}

type compositeStepDoc struct {
	Name      string            `json:"name"`
	ActionRef string            `json:"action_ref"`
	InputExpr map[string]string `json:"input_expr,omitempty"`
	OnFailure string            `json:"on_failure"`
}

type roleDoc struct {
	APIName     string   `json:"api_name"`
	DisplayName string   `json:"display_name,omitempty"`
	Description string   `json:"description,omitempty"`
	Inherits    []string `json:"inherits,omitempty"`
}

type policyRuleDoc struct {
	APIName     string            `json:"api_name"`
	DisplayName string            `json:"display_name,omitempty"`
	Description string            `json:"description,omitempty"`
	Effect      string            `json:"effect"`
	Roles       []string          `json:"roles,omitempty"`
	Operations  []string          `json:"operations"`
	ObjectType  string            `json:"object_type,omitempty"`
	ActionType  string            `json:"action_type,omitempty"`
	RowFilter   *types.Filter     `json:"row_filter,omitempty"`
	Conditions  []types.Condition `json:"conditions,omitempty"`
	Redactions  []string          `json:"redactions,omitempty"`
	Priority    int               `json:"priority,omitempty"`
}

type customToolDoc struct {
	APIName      string         `json:"api_name"`
	DisplayName  string         `json:"display_name,omitempty"`
	Description  string         `json:"description,omitempty"`
	Kind         string         `json:"kind"`
	InputSchema  map[string]any `json:"input_schema"`
	OutputSchema map[string]any `json:"output_schema,omitempty"`
	SQL          *sqlToolDoc    `json:"sql,omitempty"`
	Webhook      *webhookDoc    `json:"webhook,omitempty"`
	Composite    *compositeDoc  `json:"composite,omitempty"`
}

type sqlToolDoc struct {
	Datasource string `json:"datasource"`
	Statement  string `json:"statement"`
}

type agentDoc struct {
	APIName                   string                         `json:"api_name"`
	DisplayName               string                         `json:"display_name,omitempty"`
	Description               string                         `json:"description,omitempty"`
	SystemPrompt              string                         `json:"system_prompt"`
	Model                     types.ModelConfig              `json:"model"`
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
	Limits                    types.AgentLimits              `json:"limits"`
	RequireApprovalForActions bool                           `json:"require_approval_for_actions,omitempty"`
}

type assetDoc struct {
	APIName              string                  `json:"api_name"`
	DisplayName          string                  `json:"display_name,omitempty"`
	Description          string                  `json:"description,omitempty"`
	Metadata             map[string]any          `json:"metadata,omitempty"`
	Tags                 []string                `json:"tags,omitempty"`
	Properties           []propertyDoc           `json:"properties"`
	QualityRules         []types.QualityRule     `json:"quality_rules,omitempty"`
	Dependencies         []types.AssetDependency `json:"dependencies,omitempty"`
	Sink                 types.AssetSink         `json:"sink"`
	SavedColumnMapping   []types.ColumnMapping   `json:"saved_column_mapping,omitempty"`
	UnmappedColumnPolicy string                  `json:"unmapped_column_policy,omitempty"`
}

// ParseDocument decodes a JSON declarative document into a Document. Unknown
// fields are rejected.
func ParseDocument(raw []byte) (*Document, error) {
	var d Document
	dec := json.NewDecoder(bytesReader(raw))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&d); err != nil {
		return nil, fmt.Errorf("parse document: %w", err)
	}
	if d.Workspace.APIName == "" {
		return nil, fmt.Errorf("workspace.api_name is required")
	}
	return &d, nil
}

// Materialize converts a Document to a slice of core entities scoped under
// workspaceID. Timestamps and IDs are populated by the registry on persist.
func (d *Document) Materialize(workspaceID types.WorkspaceID) (Materialized, error) {
	out := Materialized{Workspace: workspaceFromDoc(d.Workspace, workspaceID)}
	out.Datasources = datasourcesFromDoc(d.Datasources, workspaceID)

	for _, ot := range d.ObjectTypes {
		out.ObjectTypes = append(out.ObjectTypes, objectTypeFromDoc(ot, workspaceID))
	}
	for _, lt := range d.LinkTypes {
		out.LinkTypes = append(out.LinkTypes, linkTypeFromDoc(lt, workspaceID))
	}
	for _, at := range d.ActionTypes {
		v, err := actionTypeFromDoc(at, workspaceID)
		if err != nil {
			return Materialized{}, err
		}
		out.ActionTypes = append(out.ActionTypes, v)
	}
	for _, r := range d.Roles {
		out.Roles = append(out.Roles, roleFromDoc(r, workspaceID))
	}
	for _, pr := range d.PolicyRules {
		out.PolicyRules = append(out.PolicyRules, policyRuleFromDoc(pr, workspaceID))
	}
	for _, ct := range d.CustomTools {
		v, err := customToolFromDoc(ct, workspaceID)
		if err != nil {
			return Materialized{}, err
		}
		out.CustomTools = append(out.CustomTools, v)
	}
	for _, a := range d.Agents {
		out.Agents = append(out.Agents, agentFromDoc(a, workspaceID))
	}
	for _, as := range d.Assets {
		out.Assets = append(out.Assets, assetFromDoc(as, workspaceID))
	}
	return out, nil
}

// SerializeDocument renders an *types.Ontology back to a declarative JSON document.
func SerializeDocument(o *types.Ontology) ([]byte, error) {
	doc := documentFromOntology(o)
	return json.MarshalIndent(doc, "", "  ")
}

func workspaceFromDoc(w workspaceDoc, ws types.WorkspaceID) types.Workspace {
	return types.Workspace{
		ID:          ws,
		APIName:     types.APIName(w.APIName),
		DisplayName: w.DisplayName,
		Description: w.Description,
		Settings:    w.Settings,
	}
}

func datasourcesFromDoc(in []datasourceDoc, ws types.WorkspaceID) []types.Datasource {
	out := make([]types.Datasource, 0, len(in))
	for _, ds := range in {
		out = append(out, types.Datasource{
			WorkspaceID: ws,
			APIName:     types.APIName(ds.APIName),
			DisplayName: ds.DisplayName,
			AdapterType: ds.AdapterType,
			Config:      ds.Config,
		})
	}
	return out
}

func objectTypeFromDoc(ot objectTypeDoc, ws types.WorkspaceID) types.ObjectType {
	props := make([]types.Property, 0, len(ot.Properties))
	for _, p := range ot.Properties {
		props = append(props, propertyFromDoc(p))
	}
	return types.ObjectType{
		WorkspaceID:  ws,
		APIName:      types.APIName(ot.APIName),
		DisplayName:  ot.DisplayName,
		Description:  ot.Description,
		PrimaryKey:   types.APIName(ot.PrimaryKey),
		Source:       types.SourceConfig{DatasourceAPIName: types.APIName(ot.Source.Datasource), Schema: ot.Source.Schema, Table: ot.Source.Table},
		Properties:   props,
		Environments: ot.Environments,
	}
}

func propertyFromDoc(p propertyDoc) types.Property {
	return types.Property{
		APIName:       types.APIName(p.APIName),
		DisplayName:   p.DisplayName,
		Description:   p.Description,
		DataType:      types.DataType(p.DataType),
		SourceColumn:  p.SourceColumn,
		Nullable:      p.Nullable,
		Indexed:       p.Indexed,
		AllowedValues: p.AllowedValues,
		Tags:          p.Tags,
		Markings:      p.Markings,
		Metadata:      p.Metadata,
		DefaultValue:  p.DefaultValue,
		Transforms:    p.Transforms,
		Computed:      p.Computed,
	}
}

func linkTypeFromDoc(lt linkTypeDoc, ws types.WorkspaceID) types.LinkType {
	mappings := make([]types.PropertyMapping, len(lt.PropertyMappings))
	for i, m := range lt.PropertyMappings {
		mappings[i] = types.PropertyMapping{
			FromProperty: types.APIName(m.FromProperty),
			ToProperty:   types.APIName(m.ToProperty),
		}
	}
	var junction *types.JunctionConfig
	if lt.Junction != nil {
		junction = &types.JunctionConfig{
			DatasourceAPIName: types.APIName(lt.Junction.Datasource),
			Schema:            lt.Junction.Schema,
			Table:             lt.Junction.Table,
			FromColumn:        lt.Junction.FromColumn,
			ToColumn:          lt.Junction.ToColumn,
			Properties:        lt.Junction.Properties,
		}
	}
	return types.LinkType{
		WorkspaceID:      ws,
		APIName:          types.APIName(lt.APIName),
		DisplayName:      lt.DisplayName,
		FromObjectType:   types.APIName(lt.FromObjectType),
		ToObjectType:     types.APIName(lt.ToObjectType),
		Cardinality:      types.Cardinality(lt.Cardinality),
		PropertyMappings: mappings,
		Junction:         junction,
	}
}

func handlerFromDoc(h handlerDoc) types.HandlerConfig {
	out := types.HandlerConfig{Kind: types.HandlerKind(h.Kind)}
	if h.CRUD != nil {
		mappings := make([]types.CRUDMapping, len(h.CRUD.Mappings))
		for i, m := range h.CRUD.Mappings {
			mappings[i] = types.CRUDMapping{
				TargetProperty: types.APIName(m.TargetProperty),
				Expression:     m.Expression,
			}
		}
		out.CRUD = &types.CRUDHandler{Mappings: mappings}
	}
	if h.Webhook != nil {
		out.Webhook = &types.WebhookHandler{
			URL:              h.Webhook.URL,
			TimeoutSeconds:   h.Webhook.TimeoutSeconds,
			MaxRetries:       h.Webhook.MaxRetries,
			SigningSecretRef: h.Webhook.SigningSecretRef,
			RetryOnStatus:    h.Webhook.RetryOnStatus,
			HeaderForwards:   h.Webhook.HeaderForwards,
			BackoffInitialMS: h.Webhook.BackoffInitialMS,
			BackoffMaxMS:     h.Webhook.BackoffMaxMS,
			BackoffJitter:    h.Webhook.BackoffJitter,
		}
	}
	if h.Composite != nil {
		steps := make([]types.CompositeStep, len(h.Composite.Steps))
		for i, s := range h.Composite.Steps {
			steps[i] = types.CompositeStep{
				Name: s.Name, ActionRef: types.APIName(s.ActionRef),
				InputExpr: s.InputExpr, OnFailure: types.CompositeOnFailure(s.OnFailure),
			}
		}
		out.Composite = &types.CompositeHandler{Steps: steps}
	}
	return out
}

func actionTypeFromDoc(at actionTypeDoc, ws types.WorkspaceID) (types.ActionType, error) {
	input, err := json.Marshal(at.InputSchema)
	if err != nil {
		return types.ActionType{}, fmt.Errorf("action %s input_schema: %w", at.APIName, err)
	}
	var output []byte
	if at.OutputSchema != nil {
		output, err = json.Marshal(at.OutputSchema)
		if err != nil {
			return types.ActionType{}, fmt.Errorf("action %s output_schema: %w", at.APIName, err)
		}
	}
	mode := types.ExecutionMode(at.ExecutionMode)
	if mode == "" {
		mode = types.ExecutionModeSync
	}
	return types.ActionType{
		WorkspaceID:            ws,
		APIName:                types.APIName(at.APIName),
		DisplayName:            at.DisplayName,
		Description:            at.Description,
		Subject:                types.APIName(at.Subject),
		InputSchema:            input,
		OutputSchema:           output,
		PermissionKey:          at.PermissionKey,
		IdempotencyKeyTemplate: at.IdempotencyKeyTemplate,
		ExecutionMode:          mode,
		Handler:                handlerFromDoc(at.Handler),
	}, nil
}

func roleFromDoc(r roleDoc, ws types.WorkspaceID) types.Role {
	return types.Role{
		WorkspaceID: ws,
		APIName:     types.APIName(r.APIName),
		DisplayName: r.DisplayName,
		Description: r.Description,
		Inherits:    apiNameSlice(r.Inherits),
	}
}

func policyRuleFromDoc(pr policyRuleDoc, ws types.WorkspaceID) types.PolicyRule {
	ops := make([]types.Operation, 0, len(pr.Operations))
	for _, o := range pr.Operations {
		ops = append(ops, types.Operation(o))
	}
	var rowFilter types.Filter
	if pr.RowFilter != nil {
		rowFilter = *pr.RowFilter
	}
	return types.PolicyRule{
		WorkspaceID: ws,
		APIName:     types.APIName(pr.APIName),
		DisplayName: pr.DisplayName,
		Description: pr.Description,
		Effect:      types.PolicyEffect(pr.Effect),
		Roles:       apiNameSlice(pr.Roles),
		Operations:  ops,
		ObjectType:  types.APIName(pr.ObjectType),
		ActionType:  types.APIName(pr.ActionType),
		RowFilter:   rowFilter,
		Conditions:  pr.Conditions,
		Redactions:  apiNameSlice(pr.Redactions),
		Priority:    pr.Priority,
	}
}

func customToolFromDoc(ct customToolDoc, ws types.WorkspaceID) (types.CustomTool, error) {
	input, err := json.Marshal(ct.InputSchema)
	if err != nil {
		return types.CustomTool{}, fmt.Errorf("custom_tool %s input_schema: %w", ct.APIName, err)
	}
	var output []byte
	if ct.OutputSchema != nil {
		output, err = json.Marshal(ct.OutputSchema)
		if err != nil {
			return types.CustomTool{}, fmt.Errorf("custom_tool %s output_schema: %w", ct.APIName, err)
		}
	}
	out := types.CustomTool{
		WorkspaceID: ws, APIName: types.APIName(ct.APIName),
		DisplayName: ct.DisplayName, Description: ct.Description,
		Kind:        types.CustomToolKind(ct.Kind),
		InputSchema: input, OutputSchema: output,
	}
	if ct.SQL != nil {
		out.SQL = &types.SQLToolSpec{
			DatasourceAPIName: types.APIName(ct.SQL.Datasource),
			Statement:         ct.SQL.Statement,
		}
	}
	if ct.Webhook != nil {
		out.Webhook = &types.WebhookHandler{
			URL: ct.Webhook.URL, TimeoutSeconds: ct.Webhook.TimeoutSeconds,
			MaxRetries: ct.Webhook.MaxRetries, SigningSecretRef: ct.Webhook.SigningSecretRef,
			RetryOnStatus: ct.Webhook.RetryOnStatus, HeaderForwards: ct.Webhook.HeaderForwards,
			BackoffInitialMS: ct.Webhook.BackoffInitialMS, BackoffMaxMS: ct.Webhook.BackoffMaxMS,
			BackoffJitter: ct.Webhook.BackoffJitter,
		}
	}
	if ct.Composite != nil {
		steps := make([]types.CompositeStep, len(ct.Composite.Steps))
		for i, s := range ct.Composite.Steps {
			steps[i] = types.CompositeStep{
				Name: s.Name, ActionRef: types.APIName(s.ActionRef),
				InputExpr: s.InputExpr, OnFailure: types.CompositeOnFailure(s.OnFailure),
			}
		}
		out.Composite = &types.CompositeHandler{Steps: steps}
	}
	return out, nil
}

func agentFromDoc(a agentDoc, ws types.WorkspaceID) types.Agent {
	return types.Agent{
		WorkspaceID:               ws,
		APIName:                   types.APIName(a.APIName),
		DisplayName:               a.DisplayName,
		Description:               a.Description,
		SystemPrompt:              a.SystemPrompt,
		Model:                     a.Model,
		FromObjectTypes:           apiNameSlice(a.FromObjectTypes),
		FromLinkTypes:             apiNameSlice(a.FromLinkTypes),
		FromActions:               apiNameSlice(a.FromActions),
		CustomTools:               apiNameSlice(a.CustomTools),
		ContextSources:            a.ContextSources,
		Memory:                    a.Memory,
		Planning:                  a.Planning,
		Compaction:                a.Compaction,
		Subagents:                 a.Subagents,
		Communication:             a.Communication,
		AllowedRoles:              apiNameSlice(a.AllowedRoles),
		Limits:                    a.Limits,
		RequireApprovalForActions: a.RequireApprovalForActions,
	}
}

func assetFromDoc(as assetDoc, ws types.WorkspaceID) types.Asset {
	props := make([]types.Property, 0, len(as.Properties))
	for _, p := range as.Properties {
		props = append(props, propertyFromDoc(p))
	}
	return types.Asset{
		WorkspaceID:          ws,
		APIName:              types.APIName(as.APIName),
		DisplayName:          as.DisplayName,
		Description:          as.Description,
		Metadata:             as.Metadata,
		Tags:                 as.Tags,
		Properties:           props,
		QualityRules:         as.QualityRules,
		Dependencies:         as.Dependencies,
		Sink:                 as.Sink,
		SavedColumnMapping:   as.SavedColumnMapping,
		UnmappedColumnPolicy: as.UnmappedColumnPolicy,
	}
}

func apiNameSlice(in []string) []types.APIName {
	out := make([]types.APIName, 0, len(in))
	for _, s := range in {
		out = append(out, types.APIName(s))
	}
	return out
}
