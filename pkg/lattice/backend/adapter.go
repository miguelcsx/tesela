// Backend is the user-supplied data integration. The runtime composes
// capability interfaces a-la-carte: a backend that only does reads
// implements Searcher; a fully-featured one implements every capability.
//
// This file defines the contract; concrete impls live in user code, in
// `examples/adapters/*` for reference, or are constructed inline via the
// closure sugar in `pkg/lattice/lattice.go` (`ot.Search(func(...) ...)`).

package backend

import (
	"context"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Backend is the per-driver factory. Stateless: live state belongs to the
// Connection it returns from Connect. Backends advertise their capabilities
// through the optional interfaces below; the runtime detects which methods
// are implemented and routes accordingly.
type Backend interface {
	// Type is the adapter identifier as referenced by Datasource.AdapterType
	// (e.g., "postgres", "snowflake", "my-rest-api").
	Type() string
	// Connect builds a live Connection from resolved configuration.
	Connect(ctx context.Context, cfg types.ConfigMap) (Connection, error)
}

// Connection is the lifecycle handle returned by Backend.Connect. The actual
// query/mutation methods live on capability interfaces (Searcher, Getter,
// etc.); the runtime checks each at call time.
type Connection interface {
	// Ping verifies connectivity. Optional: implement Pinger interface
	// instead if you want to opt out (returns nil by default).
	Ping(ctx context.Context) error
	// Close releases resources. Always required.
	Close(ctx context.Context) error
}

// Searcher executes a multi-row query.
type Searcher interface {
	Search(ctx context.Context, src types.SourceConfig, ot types.ObjectType, q types.QuerySpec, filter types.Filter) (types.Page, error)
}

// Getter fetches a single object by primary key.
type Getter interface {
	Get(ctx context.Context, src types.SourceConfig, ot types.ObjectType, pk any, filter types.Filter) (types.Record, error)
}

// Aggregator runs grouped aggregations.
type Aggregator interface {
	Aggregate(ctx context.Context, src types.SourceConfig, ot types.ObjectType, agg types.AggregateSpec, filter types.Filter) (types.AggregateResult, error)
}

// Traverser walks a link from one or more source records.
type Traverser interface {
	Traverse(ctx context.Context, src types.SourceConfig, lt types.LinkType, target types.ObjectType, sourceKeys []any, q types.QuerySpec, filter types.Filter) (types.Page, error)
}

// SearchExplainer returns a structured execution plan for Search.
type SearchExplainer interface {
	ExplainSearch(ctx context.Context, src types.SourceConfig, ot types.ObjectType, q types.QuerySpec, filter types.Filter) (types.QueryPlan, error)
}

// AggregateExplainer returns a structured execution plan for Aggregate.
type AggregateExplainer interface {
	ExplainAggregate(ctx context.Context, src types.SourceConfig, ot types.ObjectType, agg types.AggregateSpec, filter types.Filter) (types.QueryPlan, error)
}

// TraverseExplainer returns a structured execution plan for Traverse.
type TraverseExplainer interface {
	ExplainTraverse(ctx context.Context, src types.SourceConfig, lt types.LinkType, target types.ObjectType, sourceKeys []any, q types.QuerySpec, filter types.Filter) (types.QueryPlan, error)
}

// Mutator performs writes (insert/update/delete/upsert).
type Mutator interface {
	Mutate(ctx context.Context, src types.SourceConfig, mut types.Mutation) (types.MutationResult, error)
}

// BulkLoader ingests rows from object storage. Used by the upload pipeline.
type BulkLoader interface {
	BulkLoad(ctx context.Context, src types.SourceConfig, source ObjectStorageRef, mapping ColumnMapping) (BulkLoadResult, error)
	RollbackUpload(ctx context.Context, src types.SourceConfig, uploadID string) error
}

// ObjectStorageRef points at a file in object storage that an adapter can
// load from natively (e.g., COPY ... FROM 's3://bucket/key').
type ObjectStorageRef struct {
	URL    string `json:"url"`
	Format string `json:"format"` // csv | parquet | jsonl | avro
}

// ColumnMapping pairs a source-file column with a target object property.
type ColumnMapping struct {
	UploadID string               `json:"upload_id"`
	Entries  []ColumnMappingEntry `json:"entries"`
	Options  map[string]any       `json:"options,omitempty"`
}

// ColumnMappingEntry is a single source→target binding.
type ColumnMappingEntry struct {
	SourceColumn   string        `json:"source_column"`
	TargetProperty types.APIName `json:"target_property"`
	Transform      string        `json:"transform,omitempty"` // optional CEL expression
}

// BulkLoadResult is the summary returned to the upload pipeline.
type BulkLoadResult struct {
	RowsLoaded int64  `json:"rows_loaded"`
	StagingRef string `json:"staging_ref,omitempty"` // adapter-internal for rollback
}
