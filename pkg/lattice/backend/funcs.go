package backend

import (
	"context"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// SearchFunc is the closure shape for inline search backends.
type SearchFunc func(ctx context.Context, q types.QuerySpec) (types.Page, error)

// GetFunc is the closure shape for inline primary-key lookups.
type GetFunc func(ctx context.Context, pk any) (types.Record, error)

// MutateFunc is the closure shape for inline mutators.
type MutateFunc func(ctx context.Context, mut types.Mutation) (types.MutationResult, error)
