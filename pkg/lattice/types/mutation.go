// Mutation is the structured input adapters consume to perform a write.

package types

// MutationKind selects the write semantics.
type MutationKind string

const (
	MutationKindInsert MutationKind = "insert"
	MutationKindUpdate MutationKind = "update"
	MutationKindDelete MutationKind = "delete"
	MutationKindUpsert MutationKind = "upsert"
)

// Mutation is the adapter-level write operation.
//
// For insert and upsert, Values supplies the column-value map.
// For update, PrimaryKey identifies the target row and Values the changes.
// For delete, only PrimaryKey is required.
type Mutation struct {
	Kind       MutationKind    `json:"kind"`
	PrimaryKey any             `json:"primary_key,omitempty"`
	Values     map[APIName]any `json:"values,omitempty"`
	// ReturnFields requests specific fields back in MutationResult.Returned.
	// Empty means "no return".
	ReturnFields []APIName `json:"return_fields,omitempty"`
}

// MutationResult is the outcome of an adapter mutation.
type MutationResult struct {
	AffectedRows int64           `json:"affected_rows"`
	PrimaryKey   any             `json:"primary_key,omitempty"`
	Returned     map[APIName]any `json:"returned,omitempty"`
}
