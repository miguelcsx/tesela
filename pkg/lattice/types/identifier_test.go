package types_test

import (
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func TestAPIName_Validate(t *testing.T) {
	t.Parallel()

	good := []types.APIName{"Customer", "Order", "Order.lineItems", "Customer_ABC", "x"}
	for _, n := range good {
		if err := n.Validate(); err != nil {
			t.Fatalf("Validate(%q) = %v, want nil", n, err)
		}
	}
	bad := []types.APIName{"", "1Customer", " Customer", "Customer ", "with space", "weird-char"}
	for _, n := range bad {
		if err := n.Validate(); err == nil {
			t.Fatalf("Validate(%q) = nil, want error", n)
		}
	}
}

func TestAPIName_String(t *testing.T) {
	t.Parallel()

	if got := types.APIName("Customer").String(); got != "Customer" {
		t.Fatalf("String() = %q, want %q", got, "Customer")
	}
}
