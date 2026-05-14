// Filter is the cross-cutting query predicate AST. It is used by adapter
// queries (translated to SQL or equivalent), by policy row filters, and by
// link traversal join predicates.
//
// The AST is a single struct with an Op tag rather than a sum type with one
// variant per operator. This keeps construction trivial, JSON serialization
// natural, and walking simple. Validate enforces per-Op invariants.

package types

import (
	"fmt"
	"reflect"
	"sort"
)

// FilterOp is the operator tag of a filter node.
type FilterOp string

// Filter operators. The set is closed; adapters must support every entry.
const (
	FilterOpAnd       FilterOp = "and"
	FilterOpOr        FilterOp = "or"
	FilterOpNot       FilterOp = "not"
	FilterOpEq        FilterOp = "eq"
	FilterOpNeq       FilterOp = "neq"
	FilterOpGt        FilterOp = "gt"
	FilterOpGte       FilterOp = "gte"
	FilterOpLt        FilterOp = "lt"
	FilterOpLte       FilterOp = "lte"
	FilterOpIn        FilterOp = "in"
	FilterOpNotIn     FilterOp = "not_in"
	FilterOpLike      FilterOp = "like"
	FilterOpIsNull    FilterOp = "is_null"
	FilterOpIsNotNull FilterOp = "is_not_null"
)

// filterOpKind classifies an operator by its expected shape so Validate can
// stay declarative. Adding a new operator means adding one entry here.
type filterOpKind int

const (
	kindLogical  filterOpKind = iota // and, or
	kindNot                          // exactly one child
	kindLeaf                         // requires Property and Value
	kindLeafIn                       // requires Property and []any Value
	kindLeafNull                     // requires Property only, no Value
)

var filterOpKinds = map[FilterOp]filterOpKind{
	FilterOpAnd:       kindLogical,
	FilterOpOr:        kindLogical,
	FilterOpNot:       kindNot,
	FilterOpEq:        kindLeaf,
	FilterOpNeq:       kindLeaf,
	FilterOpGt:        kindLeaf,
	FilterOpGte:       kindLeaf,
	FilterOpLt:        kindLeaf,
	FilterOpLte:       kindLeaf,
	FilterOpLike:      kindLeaf,
	FilterOpIn:        kindLeafIn,
	FilterOpNotIn:     kindLeafIn,
	FilterOpIsNull:    kindLeafNull,
	FilterOpIsNotNull: kindLeafNull,
}

// Filter is a single node of the filter tree. The zero value (Op == "") is
// treated as "no filter" by combinators such as AndFilters.
type Filter struct {
	Op       FilterOp `json:"op"`
	Property string   `json:"property,omitempty"`
	Value    any      `json:"value,omitempty"`
	Children []Filter `json:"children,omitempty"`
}

// IsZero reports whether the filter is the zero value (no operator). Adapters
// and combinators treat zero filters as a no-op.
func (f Filter) IsZero() bool { return f.Op == "" }

// Validate checks per-Op invariants. Returns nil on the zero value (no filter).
//
// Dispatch uses a small switch over filterOpKind rather than a global map
// because validators recurse back into Validate, which would form a package
// initialization cycle if the table were a package-level var.
func (f Filter) Validate() error {
	if f.IsZero() {
		return nil
	}
	kind, ok := filterOpKinds[f.Op]
	if !ok {
		return fmt.Errorf("unknown filter op %q", f.Op)
	}
	switch kind {
	case kindLogical:
		return validateLogical(f)
	case kindNot:
		return validateNot(f)
	case kindLeaf:
		return validateLeaf(f)
	case kindLeafIn:
		return validateLeafIn(f)
	case kindLeafNull:
		return validateLeafNull(f)
	default:
		return fmt.Errorf("internal: unknown filter kind %d for op %q", kind, f.Op)
	}
}

func validateLogical(f Filter) error {
	if len(f.Children) == 0 {
		return fmt.Errorf("filter %q requires at least one child", f.Op)
	}
	return validateChildren(f.Children)
}

func validateNot(f Filter) error {
	if len(f.Children) != 1 {
		return fmt.Errorf("filter not requires exactly one child, got %d", len(f.Children))
	}
	return f.Children[0].Validate()
}

func validateLeaf(f Filter) error {
	if f.Property == "" {
		return fmt.Errorf("filter %q requires a property", f.Op)
	}
	return nil
}

func validateLeafIn(f Filter) error {
	if err := validateLeaf(f); err != nil {
		return err
	}
	if !isSlice(f.Value) {
		return fmt.Errorf("filter %q requires a slice value, got %T", f.Op, f.Value)
	}
	return nil
}

func validateLeafNull(f Filter) error {
	if f.Property == "" {
		return fmt.Errorf("filter %q requires a property", f.Op)
	}
	if f.Value != nil {
		return fmt.Errorf("filter %q must not carry a value", f.Op)
	}
	return nil
}

func validateChildren(children []Filter) error {
	for i, c := range children {
		if err := c.Validate(); err != nil {
			return fmt.Errorf("child[%d]: %w", i, err)
		}
	}
	return nil
}

func isSlice(v any) bool {
	if v == nil {
		return false
	}
	rv := reflect.ValueOf(v)
	switch rv.Kind() {
	case reflect.Slice, reflect.Array:
		return true
	default:
		return false
	}
}

// Walk invokes visit for every node in pre-order traversal (parent before
// children). Visiting a zero-value filter is a no-op.
func (f Filter) Walk(visit func(Filter)) {
	if f.IsZero() {
		return
	}
	visit(f)
	for _, c := range f.Children {
		c.Walk(visit)
	}
}

// PropertiesUsed returns the unique, sorted set of property names referenced
// by leaf nodes in the filter tree.
func (f Filter) PropertiesUsed() []string {
	seen := make(map[string]struct{})
	f.Walk(func(node Filter) {
		if node.Property != "" {
			seen[node.Property] = struct{}{}
		}
	})
	out := make([]string, 0, len(seen))
	for p := range seen {
		out = append(out, p)
	}
	sort.Strings(out)
	return out
}

// AndFilters combines filters with logical AND, dropping zero operands.
// Returns the zero filter when every operand is zero, the operand directly
// when only one survives, or an And node otherwise.
func AndFilters(filters ...Filter) Filter {
	kept := make([]Filter, 0, len(filters))
	for _, f := range filters {
		if !f.IsZero() {
			kept = append(kept, f)
		}
	}
	switch len(kept) {
	case 0:
		return Filter{}
	case 1:
		return kept[0]
	default:
		return Filter{Op: FilterOpAnd, Children: kept}
	}
}

// OrFilters combines filters with logical OR, with the same zero-skipping
// semantics as AndFilters.
func OrFilters(filters ...Filter) Filter {
	kept := make([]Filter, 0, len(filters))
	for _, f := range filters {
		if !f.IsZero() {
			kept = append(kept, f)
		}
	}
	switch len(kept) {
	case 0:
		return Filter{}
	case 1:
		return kept[0]
	default:
		return Filter{Op: FilterOpOr, Children: kept}
	}
}
