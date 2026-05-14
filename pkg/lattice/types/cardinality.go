// Cardinality is the multiplicity of a link type relationship.

package types

import "fmt"

// Cardinality is the multiplicity of a relationship between two object types.
type Cardinality string

const (
	CardinalityOneToOne   Cardinality = "one_to_one"
	CardinalityOneToMany  Cardinality = "one_to_many"
	CardinalityManyToMany Cardinality = "many_to_many"
)

// validCardinalities is the closed set used by Validate. Stored as a map for
// O(1) lookup and to keep the code declarative.
var validCardinalities = map[Cardinality]struct{}{
	CardinalityOneToOne:   {},
	CardinalityOneToMany:  {},
	CardinalityManyToMany: {},
}

// Validate reports whether c is one of the recognized cardinalities.
func (c Cardinality) Validate() error {
	if _, ok := validCardinalities[c]; !ok {
		return fmt.Errorf("unknown cardinality %q", c)
	}
	return nil
}

// RequiresJunction reports whether links of this cardinality must declare a
// junction table to resolve the relationship.
func (c Cardinality) RequiresJunction() bool { return c == CardinalityManyToMany }

// String implements fmt.Stringer.
func (c Cardinality) String() string { return string(c) }
