package types_test

import (
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

func TestOperation_Validate(t *testing.T) {
	t.Parallel()

	for _, op := range []types.Operation{
		types.OperationRead, types.OperationSearch, types.OperationAggregate,
		types.OperationTraverse, types.OperationCreate, types.OperationUpdate,
		types.OperationDelete, types.OperationExecute,
	} {
		if err := op.Validate(); err != nil {
			t.Fatalf("Validate(%q) = %v", op, err)
		}
	}
	for _, op := range []types.Operation{"", "list", "patch", "do"} {
		if err := op.Validate(); err == nil {
			t.Fatalf("Validate(%q) = nil, want error", op)
		}
	}
}

func TestOperation_IsRead(t *testing.T) {
	t.Parallel()

	cases := map[types.Operation]bool{
		types.OperationRead:      true,
		types.OperationSearch:    true,
		types.OperationAggregate: true,
		types.OperationTraverse:  true,
		types.OperationCreate:    false,
		types.OperationUpdate:    false,
		types.OperationDelete:    false,
		types.OperationExecute:   false,
	}
	for op, want := range cases {
		if got := op.IsRead(); got != want {
			t.Fatalf("IsRead(%q) = %v, want %v", op, got, want)
		}
	}
}
