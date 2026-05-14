// Query-side value types: QuerySpec, AggregateSpec, SortSpec, PageSpec, and
// the related result containers.

package types

// QuerySpec describes a Search request to an adapter.
type QuerySpec struct {
	Filter Filter       `json:"filter,omitempty"`
	Sort   []SortSpec   `json:"sort,omitempty"`
	Page   PageSpec     `json:"page,omitempty"`
	Select []APIName    `json:"select,omitempty"`
	Hints  QueryHints   `json:"hints,omitempty"`
	Cache  CacheControl `json:"cache,omitempty"`
}

// SortDirection is the direction of an order-by clause.
type SortDirection string

const (
	SortAsc  SortDirection = "asc"
	SortDesc SortDirection = "desc"
)

// SortSpec orders search results by a property.
type SortSpec struct {
	Property  APIName       `json:"property"`
	Direction SortDirection `json:"direction"`
}

// PageSpec is cursor-based pagination. Cursor is opaque to callers; adapters
// encode the last-key state required to resume.
type PageSpec struct {
	Limit  int    `json:"limit,omitempty"`
	Cursor string `json:"cursor,omitempty"`
}

// AggregateSpec describes a grouped aggregation query.
type AggregateSpec struct {
	Filter  Filter       `json:"filter,omitempty"`
	GroupBy []APIName    `json:"group_by,omitempty"`
	Metrics []MetricSpec `json:"metrics"`
	Sort    []SortSpec   `json:"sort,omitempty"`
	Page    PageSpec     `json:"page,omitempty"`
	Hints   QueryHints   `json:"hints,omitempty"`
	Cache   CacheControl `json:"cache,omitempty"`
}

// MetricFunc is the closed set of aggregation functions adapters must support.
type MetricFunc string

const (
	MetricFuncCount MetricFunc = "count"
	MetricFuncSum   MetricFunc = "sum"
	MetricFuncAvg   MetricFunc = "avg"
	MetricFuncMin   MetricFunc = "min"
	MetricFuncMax   MetricFunc = "max"
)

// MetricSpec is a single metric in an aggregation.
type MetricSpec struct {
	Function MetricFunc `json:"function"`
	Property APIName    `json:"property,omitempty"` // omitted for count
	Alias    string     `json:"alias,omitempty"`
}

// Page is a paginated set of records returned by Search.
type Page struct {
	Records    []Record `json:"records"`
	TotalCount int64    `json:"total_count,omitempty"`
	NextCursor string   `json:"next_cursor,omitempty"`
	Truncated  bool     `json:"truncated,omitempty"`
}

// AggregateResult is the result of an Aggregate call.
type AggregateResult struct {
	Groups []AggregateGroup `json:"groups"`
}

// AggregateGroup is a single output row of an aggregation.
type AggregateGroup struct {
	Keys    map[APIName]any `json:"keys,omitempty"`
	Metrics map[string]any  `json:"metrics"`
}

// QueryHints are explicit optimizer hints supplied by callers or framework
// integrations. Adapters may honor, ignore, or reject them.
type QueryHints struct {
	JoinOrder      []APIName      `json:"join_order,omitempty"`
	IndexHints     []IndexHint    `json:"index_hints,omitempty"`
	ExecutionTags  []string       `json:"execution_tags,omitempty"`
	AdapterOptions map[string]any `json:"adapter_options,omitempty"`
}

// IndexHint requests a specific access path for one property.
type IndexHint struct {
	Property APIName `json:"property"`
	Name     string  `json:"name,omitempty"`
}

// CacheControl tells integrations how aggressively a request may use cached
// results. Cache behavior is always explicit.
type CacheControl struct {
	Mode       CacheMode `json:"mode,omitempty"`
	Namespace  string    `json:"namespace,omitempty"`
	TTLSeconds int       `json:"ttl_seconds,omitempty"`
}

type CacheMode string

const (
	CacheModeDefault CacheMode = "default"
	CacheModeBypass  CacheMode = "bypass"
	CacheModeRequire CacheMode = "require"
	CacheModeRefresh CacheMode = "refresh"
)

// QueryPlan is a structured explanation returned by adapters that implement
// the optional explain interfaces.
type QueryPlan struct {
	Summary       string         `json:"summary,omitempty"`
	EstimatedCost float64        `json:"estimated_cost,omitempty"`
	Cacheable     bool           `json:"cacheable,omitempty"`
	Nodes         []PlanNode     `json:"nodes,omitempty"`
	Warnings      []string       `json:"warnings,omitempty"`
	Metadata      map[string]any `json:"metadata,omitempty"`
}

type PlanNode struct {
	Kind     string         `json:"kind"`
	Label    string         `json:"label,omitempty"`
	Object   APIName        `json:"object,omitempty"`
	Children []PlanNode     `json:"children,omitempty"`
	Metadata map[string]any `json:"metadata,omitempty"`
}
