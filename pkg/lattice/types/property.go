// Property is a typed attribute of an ObjectType.

package types

// Property is the typed attribute of an object type. Properties are versioned
// in lockstep with their parent object type — adding a property bumps the
// type version.
type Property struct {
	APIName       APIName  `json:"api_name"`
	DisplayName   string   `json:"display_name,omitempty"`
	Description   string   `json:"description,omitempty"`
	DataType      DataType `json:"data_type"`
	SourceColumn  string   `json:"source_column,omitempty"`
	Nullable      bool     `json:"nullable"`
	Indexed       bool     `json:"indexed,omitempty"`
	AllowedValues []string `json:"allowed_values,omitempty"`
	Tags          []string `json:"tags,omitempty"`
	// Metadata carries arbitrary user-defined annotations. Lattice stores and
	// transports these values but does not interpret them.
	Metadata map[string]any `json:"metadata,omitempty"`
	// Markings restrict who can read this property. An actor missing any
	// marking listed here gets the property redacted. Empty = unrestricted.
	Markings  []string `json:"markings,omitempty"`
	SortOrder int      `json:"sort_order,omitempty"`
	// DefaultValue is explicit, never implicit. Lattice only applies it when a
	// caller or integration layer chooses to do so.
	DefaultValue any `json:"default_value,omitempty"`
	// Transforms are declarative, ordered transforms supplied by the user or an
	// external integration. Lattice preserves and validates them structurally.
	Transforms []PropertyTransform `json:"transforms,omitempty"`
	// Computed, when set, marks the property as derived. Its value is
	// evaluated during result hydration from the expression in Computed.
	Computed *ComputedProperty `json:"computed,omitempty"`
}

// IsComputed reports whether the property is derived rather than sourced from
// a column.
func (p Property) IsComputed() bool { return p.Computed != nil }

// ResolvedSourceColumn returns the source column name for this property,
// defaulting to the api_name when SourceColumn is empty.
func (p Property) ResolvedSourceColumn() string {
	if p.SourceColumn != "" {
		return p.SourceColumn
	}
	return string(p.APIName)
}

// ComputedProperty describes a derived property. The expression is the same
// CEL dialect used by policy custom_expression conditions and is evaluated at
// hydration time against the row's other property values.
type ComputedProperty struct {
	Expression string `json:"expression"`
	// DependsOn makes lineage explicit. Lattice does not try to infer semantic
	// dependencies from the expression text.
	DependsOn []APIName `json:"depends_on,omitempty"`
}

// PropertyTransform is a declarative transformation step. The semantics are
// owned by the integration that executes it, not by Lattice itself.
type PropertyTransform struct {
	Kind        string         `json:"kind"`
	Description string         `json:"description,omitempty"`
	Config      map[string]any `json:"config,omitempty"`
}
