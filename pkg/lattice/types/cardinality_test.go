package types_test

import (
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func TestCardinality_Validate(t *testing.T) {
	t.Parallel()

	for _, c := range []types.Cardinality{
		types.CardinalityOneToOne, types.CardinalityOneToMany, types.CardinalityManyToMany,
	} {
		if err := c.Validate(); err != nil {
			t.Fatalf("Validate(%q) = %v, want nil", c, err)
		}
	}
	for _, c := range []types.Cardinality{"", "many_to_one", "any"} {
		if err := c.Validate(); err == nil {
			t.Fatalf("Validate(%q) = nil, want error", c)
		}
	}
}

func TestCardinality_RequiresJunction(t *testing.T) {
	t.Parallel()

	if !types.CardinalityManyToMany.RequiresJunction() {
		t.Fatal("many_to_many must require a junction")
	}
	for _, c := range []types.Cardinality{types.CardinalityOneToOne, types.CardinalityOneToMany} {
		if c.RequiresJunction() {
			t.Fatalf("%q must not require a junction", c)
		}
	}
}
