// Ontology is the in-memory snapshot the registry serves to readers, plus the
// versioning and diff value types used by the API.

package types

import "time"

// Kind enumerates the kinds of entities the ontology contains. Used for
// generic CRUD endpoints and diff output.
type Kind string

const (
	KindWorkspace  Kind = "workspace"
	KindDatasource Kind = "datasource"
	KindObjectType Kind = "object_type"
	KindLinkType   Kind = "link_type"
	KindActionType Kind = "action_type"
	KindRole       Kind = "role"
	KindPolicyRule Kind = "policy_rule"
	KindCustomTool Kind = "custom_tool"
	KindAgent      Kind = "agent"
	KindAsset      Kind = "asset"
)

// Ontology is the immutable snapshot of all configuration entities for a
// workspace at a point in time. The registry hands snapshots out under an
// atomic.Pointer; readers never mutate them.
type Ontology struct {
	Workspace   Workspace    `json:"workspace"`
	Version     int          `json:"version"`
	GeneratedAt time.Time    `json:"generated_at"`
	Datasources []Datasource `json:"datasources,omitempty"`
	ObjectTypes []ObjectType `json:"object_types,omitempty"`
	LinkTypes   []LinkType   `json:"link_types,omitempty"`
	ActionTypes []ActionType `json:"action_types,omitempty"`
	Roles       []Role       `json:"roles,omitempty"`
	PolicyRules []PolicyRule `json:"policy_rules,omitempty"`
	CustomTools []CustomTool `json:"custom_tools,omitempty"`
	Agents      []Agent      `json:"agents,omitempty"`
	Assets      []Asset      `json:"assets,omitempty"`
}

// ObjectTypeByName returns the object type with matching api_name.
func (o Ontology) ObjectTypeByName(name APIName) (ObjectType, bool) {
	for _, ot := range o.ObjectTypes {
		if ot.APIName == name {
			return ot, true
		}
	}
	return ObjectType{}, false
}

// LinkTypeByName returns the link type with matching api_name.
func (o Ontology) LinkTypeByName(name APIName) (LinkType, bool) {
	for _, lt := range o.LinkTypes {
		if lt.APIName == name {
			return lt, true
		}
	}
	return LinkType{}, false
}

// DatasourceByName returns the datasource with matching api_name.
func (o Ontology) DatasourceByName(name APIName) (Datasource, bool) {
	for _, ds := range o.Datasources {
		if ds.APIName == name {
			return ds, true
		}
	}
	return Datasource{}, false
}

// ActionTypeByName returns the action type with matching api_name.
func (o Ontology) ActionTypeByName(name APIName) (ActionType, bool) {
	for _, at := range o.ActionTypes {
		if at.APIName == name {
			return at, true
		}
	}
	return ActionType{}, false
}

// OntologyVersion is a published, named snapshot in the version history.
type OntologyVersion struct {
	ID          string      `json:"id"`
	WorkspaceID WorkspaceID `json:"workspace_id"`
	Name        string      `json:"name"`
	Snapshot    Ontology    `json:"snapshot"`
	CreatedBy   string      `json:"created_by"`
	CreatedAt   time.Time   `json:"created_at"`
	Notes       string      `json:"notes,omitempty"`
}

// Diff describes the structural difference between two ontologies. The same
// shape is produced by Apply (showing what would change) and by Diff
// (comparing two named versions).
type Diff struct {
	Created []DiffEntry `json:"created,omitempty"`
	Updated []DiffEntry `json:"updated,omitempty"`
	Deleted []DiffEntry `json:"deleted,omitempty"`
	Errors  []string    `json:"errors,omitempty"`
}

// DiffEntry describes a single delta in a Diff.
type DiffEntry struct {
	Kind    Kind    `json:"kind"`
	APIName APIName `json:"api_name"`
	Summary string  `json:"summary,omitempty"`
}

// IsEmpty reports whether the diff contains no changes.
func (d Diff) IsEmpty() bool {
	return len(d.Created) == 0 && len(d.Updated) == 0 && len(d.Deleted) == 0
}

// Change is a single update event published by the registry's Subscribe
// channel — consumed by the GraphQL builder, the SDK codegen, and any other
// component that needs to react to ontology hot-reload.
type Change struct {
	WorkspaceID WorkspaceID `json:"workspace_id"`
	NewVersion  int         `json:"new_version"`
	OccurredAt  time.Time   `json:"occurred_at"`
	Diff        Diff        `json:"diff"`
}
