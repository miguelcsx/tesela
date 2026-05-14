package types_test

import (
	"sort"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// helper: leaf filter for tests.
func eq(prop string, val any) types.Filter {
	return types.Filter{Op: types.FilterOpEq, Property: prop, Value: val}
}

func and(children ...types.Filter) types.Filter {
	return types.Filter{Op: types.FilterOpAnd, Children: children}
}

func or(children ...types.Filter) types.Filter {
	return types.Filter{Op: types.FilterOpOr, Children: children}
}

func not(child types.Filter) types.Filter {
	return types.Filter{Op: types.FilterOpNot, Children: []types.Filter{child}}
}

func TestFilter_Validate_LeafRequiresProperty(t *testing.T) {
	t.Parallel()

	if err := eq("", 1).Validate(); err == nil {
		t.Fatal("eq with empty property must fail validation")
	}
	if err := eq("status", "open").Validate(); err != nil {
		t.Fatalf("valid eq returned %v", err)
	}
}

func TestFilter_Validate_AndOrRequireChildren(t *testing.T) {
	t.Parallel()

	if err := and().Validate(); err == nil {
		t.Fatal("and with no children must fail validation")
	}
	if err := or().Validate(); err == nil {
		t.Fatal("or with no children must fail validation")
	}
	if err := and(eq("a", 1), eq("b", 2)).Validate(); err != nil {
		t.Fatalf("valid and returned %v", err)
	}
}

func TestFilter_Validate_NotRequiresExactlyOneChild(t *testing.T) {
	t.Parallel()

	if err := not(eq("a", 1)).Validate(); err != nil {
		t.Fatalf("valid not returned %v", err)
	}
	bad := types.Filter{Op: types.FilterOpNot, Children: []types.Filter{eq("a", 1), eq("b", 2)}}
	if err := bad.Validate(); err == nil {
		t.Fatal("not with two children must fail validation")
	}
	empty := types.Filter{Op: types.FilterOpNot}
	if err := empty.Validate(); err == nil {
		t.Fatal("not with no children must fail validation")
	}
}

func TestFilter_Validate_IsNullHasNoValue(t *testing.T) {
	t.Parallel()

	good := types.Filter{Op: types.FilterOpIsNull, Property: "x"}
	if err := good.Validate(); err != nil {
		t.Fatalf("valid is_null returned %v", err)
	}
	bad := types.Filter{Op: types.FilterOpIsNull, Property: "x", Value: 1}
	if err := bad.Validate(); err == nil {
		t.Fatal("is_null with a value must fail validation")
	}
}

func TestFilter_Validate_InRequiresSliceValue(t *testing.T) {
	t.Parallel()

	good := types.Filter{Op: types.FilterOpIn, Property: "status", Value: []any{"a", "b"}}
	if err := good.Validate(); err != nil {
		t.Fatalf("valid in returned %v", err)
	}
	bad := types.Filter{Op: types.FilterOpIn, Property: "status", Value: "a"}
	if err := bad.Validate(); err == nil {
		t.Fatal("in with non-slice value must fail validation")
	}
}

func TestFilter_Walk_VisitsEveryNode(t *testing.T) {
	t.Parallel()

	f := and(
		eq("a", 1),
		or(
			eq("b", 2),
			not(eq("c", 3)),
		),
	)
	visited := 0
	f.Walk(func(_ types.Filter) { visited++ })
	// Nodes: and, eq(a), or, eq(b), not, eq(c) = 6.
	if visited != 6 {
		t.Fatalf("Walk visited %d nodes, want 6", visited)
	}
}

func TestFilter_PropertiesUsed_DeduplicatesAndSorts(t *testing.T) {
	t.Parallel()

	f := and(
		eq("status", "open"),
		or(eq("region", "US"), eq("status", "pending")),
	)
	got := f.PropertiesUsed()
	sort.Strings(got)
	want := []string{"region", "status"}
	if len(got) != len(want) || got[0] != want[0] || got[1] != want[1] {
		t.Fatalf("PropertiesUsed = %v, want %v", got, want)
	}
}

func TestFilter_IsZero(t *testing.T) {
	t.Parallel()

	var zero types.Filter
	if !zero.IsZero() {
		t.Fatal("zero Filter must report IsZero() true")
	}
	if eq("a", 1).IsZero() {
		t.Fatal("non-empty Filter must report IsZero() false")
	}
}

func TestFilter_And_HelperFlattens(t *testing.T) {
	t.Parallel()

	a := eq("a", 1)
	b := eq("b", 2)
	c := eq("c", 3)

	combined := types.AndFilters(a, b, c)
	if combined.Op != types.FilterOpAnd {
		t.Fatalf("AndFilters Op: want and, got %q", combined.Op)
	}
	if len(combined.Children) != 3 {
		t.Fatalf("AndFilters Children length = %d, want 3", len(combined.Children))
	}
}

func TestFilter_AndFilters_DropsZeroOperands(t *testing.T) {
	t.Parallel()

	a := eq("a", 1)
	combined := types.AndFilters(types.Filter{}, a, types.Filter{})
	// Single non-zero operand collapses to that operand directly.
	if combined.Op != types.FilterOpEq || combined.Property != "a" {
		t.Fatalf("AndFilters did not collapse single operand: %+v", combined)
	}
}

func TestFilter_AndFilters_AllZeroReturnsZero(t *testing.T) {
	t.Parallel()

	combined := types.AndFilters(types.Filter{}, types.Filter{})
	if !combined.IsZero() {
		t.Fatalf("AndFilters of all-zero must be zero, got %+v", combined)
	}
}
