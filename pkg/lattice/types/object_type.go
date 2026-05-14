// ObjectType is a class of operational entities — the node-equivalent of the
// ontology graph.

package types

import "time"

// ObjectTypeID is the canonical handle for an object type.
type ObjectTypeID string

// ObjectType describes a class of operational entities.
type ObjectType struct {
	ID           ObjectTypeID `json:"id"`
	WorkspaceID  WorkspaceID  `json:"workspace_id"`
	APIName      APIName      `json:"api_name"`
	DisplayName  string       `json:"display_name"`
	Description  string       `json:"description,omitempty"`
	PrimaryKey   APIName      `json:"primary_key"`
	Source       SourceConfig `json:"source"`
	Properties   []Property   `json:"properties"`
	Environments []string     `json:"environments,omitempty"`
	Version      int          `json:"version"`
	DeprecatedAt *time.Time   `json:"deprecated_at,omitempty"`
	CreatedAt    time.Time    `json:"created_at"`
	UpdatedAt    time.Time    `json:"updated_at"`
}

// PropertyByName returns the property with matching api_name, or false.
func (ot ObjectType) PropertyByName(name APIName) (Property, bool) {
	for _, p := range ot.Properties {
		if p.APIName == name {
			return p, true
		}
	}
	return Property{}, false
}

// PropertyMap returns properties indexed by api_name. Useful when a caller
// needs many lookups against the same object type.
func (ot ObjectType) PropertyMap() map[APIName]Property {
	out := make(map[APIName]Property, len(ot.Properties))
	for _, p := range ot.Properties {
		out[p.APIName] = p
	}
	return out
}

// PrimaryKeyProperty returns the property entry corresponding to PrimaryKey.
// Callers SHOULD only invoke this on validated object types.
func (ot ObjectType) PrimaryKeyProperty() (Property, bool) {
	return ot.PropertyByName(ot.PrimaryKey)
}

// IsDeprecated reports whether the object type has been marked deprecated.
func (ot ObjectType) IsDeprecated() bool { return ot.DeprecatedAt != nil }
