// Materialized is the in-memory bundle a declarative ontology document expands into. It is
// the input to the validator and to the registry's persist step.

package ontology

import (
	"bytes"
	"io"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Materialized is the in-memory bundle a declarative ontology document expands into.
type Materialized struct {
	Workspace   types.Workspace
	Datasources []types.Datasource
	ObjectTypes []types.ObjectType
	LinkTypes   []types.LinkType
	ActionTypes []types.ActionType
	Roles       []types.Role
	PolicyRules []types.PolicyRule
	CustomTools []types.CustomTool
	Agents      []types.Agent
	Assets      []types.Asset
}

// Snapshot converts a Materialized bundle into a *types.Ontology snapshot
// (read-only consumers only).
func (m Materialized) Snapshot(version int) *types.Ontology {
	return &types.Ontology{
		Workspace:   m.Workspace,
		Version:     version,
		Datasources: m.Datasources,
		ObjectTypes: m.ObjectTypes,
		LinkTypes:   m.LinkTypes,
		ActionTypes: m.ActionTypes,
		Roles:       m.Roles,
		PolicyRules: m.PolicyRules,
		CustomTools: m.CustomTools,
		Agents:      m.Agents,
		Assets:      m.Assets,
	}
}

// bytesReader returns an io.Reader over raw without copying.
func bytesReader(raw []byte) io.Reader { return bytes.NewReader(raw) }

func documentFromOntology(o *types.Ontology) *Document {
	doc := &Document{
		APIVersion: "lattice/v1",
		Workspace: workspaceDoc{
			APIName:     string(o.Workspace.APIName),
			DisplayName: o.Workspace.DisplayName,
			Description: o.Workspace.Description,
			Settings:    o.Workspace.Settings,
		},
	}
	for _, ds := range o.Datasources {
		doc.Datasources = append(doc.Datasources, datasourceDoc{
			APIName: string(ds.APIName), DisplayName: ds.DisplayName,
			AdapterType: ds.AdapterType, Config: ds.Config,
		})
	}
	for _, ot := range o.ObjectTypes {
		props := make([]propertyDoc, 0, len(ot.Properties))
		for _, p := range ot.Properties {
			props = append(props, propertyDoc{
				APIName: string(p.APIName), DisplayName: p.DisplayName, Description: p.Description,
				DataType: string(p.DataType), SourceColumn: p.SourceColumn, Nullable: p.Nullable,
				Indexed: p.Indexed, AllowedValues: p.AllowedValues, Tags: p.Tags, Markings: p.Markings,
				Metadata: p.Metadata, DefaultValue: p.DefaultValue, Transforms: p.Transforms, Computed: p.Computed,
			})
		}
		doc.ObjectTypes = append(doc.ObjectTypes, objectTypeDoc{
			APIName: string(ot.APIName), DisplayName: ot.DisplayName, Description: ot.Description,
			PrimaryKey: string(ot.PrimaryKey),
			Source: sourceDoc{
				Datasource: string(ot.Source.DatasourceAPIName),
				Schema:     ot.Source.Schema,
				Table:      ot.Source.Table,
			},
			Properties:   props,
			Environments: ot.Environments,
		})
	}
	for _, lt := range o.LinkTypes {
		mappings := make([]propertyMappingDoc, len(lt.PropertyMappings))
		for i, m := range lt.PropertyMappings {
			mappings[i] = propertyMappingDoc{
				FromProperty: string(m.FromProperty), ToProperty: string(m.ToProperty),
			}
		}
		var junction *junctionDoc
		if lt.Junction != nil {
			junction = &junctionDoc{
				Datasource: string(lt.Junction.DatasourceAPIName),
				Schema:     lt.Junction.Schema, Table: lt.Junction.Table,
				FromColumn: lt.Junction.FromColumn, ToColumn: lt.Junction.ToColumn,
				Properties: lt.Junction.Properties,
			}
		}
		doc.LinkTypes = append(doc.LinkTypes, linkTypeDoc{
			APIName: string(lt.APIName), DisplayName: lt.DisplayName,
			FromObjectType: string(lt.FromObjectType), ToObjectType: string(lt.ToObjectType),
			Cardinality: string(lt.Cardinality), PropertyMappings: mappings, Junction: junction,
		})
	}
	for _, at := range o.ActionTypes {
		doc.ActionTypes = append(doc.ActionTypes, actionTypeFromCore(at))
	}
	for _, r := range o.Roles {
		doc.Roles = append(doc.Roles, roleDoc{
			APIName: string(r.APIName), DisplayName: r.DisplayName, Description: r.Description,
			Inherits: stringSlice(r.Inherits),
		})
	}
	for _, pr := range o.PolicyRules {
		doc.PolicyRules = append(doc.PolicyRules, policyRuleFromCore(pr))
	}
	for _, ct := range o.CustomTools {
		doc.CustomTools = append(doc.CustomTools, customToolFromCore(ct))
	}
	for _, a := range o.Agents {
		doc.Agents = append(doc.Agents, agentDoc{
			APIName: string(a.APIName), DisplayName: a.DisplayName, Description: a.Description,
			SystemPrompt: a.SystemPrompt, Model: a.Model,
			FromObjectTypes: stringSlice(a.FromObjectTypes), FromLinkTypes: stringSlice(a.FromLinkTypes),
			FromActions: stringSlice(a.FromActions), CustomTools: stringSlice(a.CustomTools),
			ContextSources: a.ContextSources, Memory: a.Memory, Planning: a.Planning,
			Compaction: a.Compaction, Subagents: a.Subagents, Communication: a.Communication,
			AllowedRoles: stringSlice(a.AllowedRoles), Limits: a.Limits,
			RequireApprovalForActions: a.RequireApprovalForActions,
		})
	}
	for _, as := range o.Assets {
		props := make([]propertyDoc, 0, len(as.Properties))
		for _, p := range as.Properties {
			props = append(props, propertyDoc{
				APIName: string(p.APIName), DisplayName: p.DisplayName, Description: p.Description,
				DataType: string(p.DataType), SourceColumn: p.SourceColumn, Nullable: p.Nullable,
			})
		}
		doc.Assets = append(doc.Assets, assetDoc{
			APIName: string(as.APIName), DisplayName: as.DisplayName, Description: as.Description,
			Metadata: as.Metadata, Tags: as.Tags, Properties: props, QualityRules: as.QualityRules,
			Dependencies: as.Dependencies, Sink: as.Sink,
			SavedColumnMapping: as.SavedColumnMapping, UnmappedColumnPolicy: as.UnmappedColumnPolicy,
		})
	}
	return doc
}

func stringSlice(in []types.APIName) []string {
	out := make([]string, 0, len(in))
	for _, n := range in {
		out = append(out, string(n))
	}
	return out
}

func actionTypeFromCore(at types.ActionType) actionTypeDoc {
	return actionTypeDoc{
		APIName: string(at.APIName), DisplayName: at.DisplayName, Description: at.Description,
		Subject: string(at.Subject),
		// InputSchema/OutputSchema are kept as opaque maps in the document; the
		// round-trip preserves shape but not whitespace.
		InputSchema: jsonToMap(at.InputSchema), OutputSchema: jsonToMap(at.OutputSchema),
		PermissionKey:          at.PermissionKey,
		IdempotencyKeyTemplate: at.IdempotencyKeyTemplate,
		ExecutionMode:          string(at.ExecutionMode),
		Handler:                handlerDocFromCore(at.Handler),
	}
}

func handlerDocFromCore(h types.HandlerConfig) handlerDoc {
	out := handlerDoc{Kind: string(h.Kind)}
	if h.CRUD != nil {
		mappings := make([]crudMappingDoc, len(h.CRUD.Mappings))
		for i, m := range h.CRUD.Mappings {
			mappings[i] = crudMappingDoc{
				TargetProperty: string(m.TargetProperty), Expression: m.Expression,
			}
		}
		out.CRUD = &crudDoc{Mappings: mappings}
	}
	if h.Webhook != nil {
		out.Webhook = &webhookDoc{
			URL: h.Webhook.URL, TimeoutSeconds: h.Webhook.TimeoutSeconds,
			MaxRetries: h.Webhook.MaxRetries, SigningSecretRef: h.Webhook.SigningSecretRef,
			RetryOnStatus: h.Webhook.RetryOnStatus, HeaderForwards: h.Webhook.HeaderForwards,
			BackoffInitialMS: h.Webhook.BackoffInitialMS, BackoffMaxMS: h.Webhook.BackoffMaxMS,
			BackoffJitter: h.Webhook.BackoffJitter,
		}
	}
	if h.Composite != nil {
		steps := make([]compositeStepDoc, len(h.Composite.Steps))
		for i, s := range h.Composite.Steps {
			steps[i] = compositeStepDoc{
				Name: s.Name, ActionRef: string(s.ActionRef),
				InputExpr: s.InputExpr, OnFailure: string(s.OnFailure),
			}
		}
		out.Composite = &compositeDoc{Steps: steps}
	}
	return out
}

func policyRuleFromCore(pr types.PolicyRule) policyRuleDoc {
	ops := make([]string, 0, len(pr.Operations))
	for _, o := range pr.Operations {
		ops = append(ops, string(o))
	}
	var rf *types.Filter
	if !pr.RowFilter.IsZero() {
		f := pr.RowFilter
		rf = &f
	}
	return policyRuleDoc{
		APIName: string(pr.APIName), DisplayName: pr.DisplayName, Description: pr.Description,
		Effect: string(pr.Effect), Roles: stringSlice(pr.Roles), Operations: ops,
		ObjectType: string(pr.ObjectType), ActionType: string(pr.ActionType),
		RowFilter: rf, Conditions: pr.Conditions, Redactions: stringSlice(pr.Redactions),
		Priority: pr.Priority,
	}
}

func customToolFromCore(ct types.CustomTool) customToolDoc {
	out := customToolDoc{
		APIName: string(ct.APIName), DisplayName: ct.DisplayName, Description: ct.Description,
		Kind:         string(ct.Kind),
		InputSchema:  jsonToMap(ct.InputSchema),
		OutputSchema: jsonToMap(ct.OutputSchema),
	}
	if ct.SQL != nil {
		out.SQL = &sqlToolDoc{Datasource: string(ct.SQL.DatasourceAPIName), Statement: ct.SQL.Statement}
	}
	if ct.Webhook != nil {
		out.Webhook = &webhookDoc{
			URL: ct.Webhook.URL, TimeoutSeconds: ct.Webhook.TimeoutSeconds,
			MaxRetries: ct.Webhook.MaxRetries, SigningSecretRef: ct.Webhook.SigningSecretRef,
			RetryOnStatus: ct.Webhook.RetryOnStatus, HeaderForwards: ct.Webhook.HeaderForwards,
			BackoffInitialMS: ct.Webhook.BackoffInitialMS, BackoffMaxMS: ct.Webhook.BackoffMaxMS,
			BackoffJitter: ct.Webhook.BackoffJitter,
		}
	}
	if ct.Composite != nil {
		steps := make([]compositeStepDoc, len(ct.Composite.Steps))
		for i, s := range ct.Composite.Steps {
			steps[i] = compositeStepDoc{
				Name: s.Name, ActionRef: string(s.ActionRef),
				InputExpr: s.InputExpr, OnFailure: string(s.OnFailure),
			}
		}
		out.Composite = &compositeDoc{Steps: steps}
	}
	return out
}

func jsonToMap(raw []byte) map[string]any {
	if len(raw) == 0 {
		return nil
	}
	var m map[string]any
	if err := jsonUnmarshal(raw, &m); err != nil {
		return nil
	}
	return m
}
