// LinkType is a directed relationship between two object types.

package types

import "time"

// LinkTypeID is the canonical handle for a link type.
type LinkTypeID string

// LinkType describes a directed relationship between two object types.
// PropertyMappings define the join condition; Junction is required for
// many_to_many relationships.
type LinkType struct {
	ID               LinkTypeID        `json:"id"`
	WorkspaceID      WorkspaceID       `json:"workspace_id"`
	APIName          APIName           `json:"api_name"`
	DisplayName      string            `json:"display_name,omitempty"`
	FromObjectType   APIName           `json:"from_object_type"`
	ToObjectType     APIName           `json:"to_object_type"`
	Cardinality      Cardinality       `json:"cardinality"`
	PropertyMappings []PropertyMapping `json:"property_mappings"`
	Junction         *JunctionConfig   `json:"junction,omitempty"`
	Version          int               `json:"version"`
	DeprecatedAt     *time.Time        `json:"deprecated_at,omitempty"`
	CreatedAt        time.Time         `json:"created_at"`
	UpdatedAt        time.Time         `json:"updated_at"`
}

// PropertyMapping is a single join condition: from.<FromProperty> = to.<ToProperty>.
// Multiple mappings within the same link type are combined with AND.
type PropertyMapping struct {
	FromProperty APIName `json:"from_property"`
	ToProperty   APIName `json:"to_property"`
}

// JunctionConfig describes the junction table for a many_to_many link.
// Additional properties exposed during traversal can be declared in Properties.
type JunctionConfig struct {
	DatasourceAPIName APIName  `json:"datasource"`
	Schema            string   `json:"schema,omitempty"`
	Table             string   `json:"table"`
	FromColumn        string   `json:"from_column"`
	ToColumn          string   `json:"to_column"`
	Properties        []string `json:"properties,omitempty"`
}

// IsManyToMany reports whether the link type has many-to-many cardinality.
func (lt LinkType) IsManyToMany() bool { return lt.Cardinality == CardinalityManyToMany }
