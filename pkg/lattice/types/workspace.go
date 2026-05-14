// Workspace is the top-level isolation boundary in Lattice.

package types

import "time"

// WorkspaceID is the canonical handle for a workspace.
type WorkspaceID string

// Workspace is a tenant: every other entity in the ontology belongs to exactly
// one workspace, and operational quotas apply per workspace.
type Workspace struct {
	ID          WorkspaceID       `json:"id"`
	APIName     APIName           `json:"api_name"`
	DisplayName string            `json:"display_name"`
	Description string            `json:"description,omitempty"`
	Settings    WorkspaceSettings `json:"settings"`
	CreatedAt   time.Time         `json:"created_at"`
	UpdatedAt   time.Time         `json:"updated_at"`
}

// WorkspaceSettings collects per-workspace operational limits and toggles.
// Defaults are populated at creation time; tuning is done through the
// workspace API.
type WorkspaceSettings struct {
	// MaxRowsPerQuery caps the number of records Search may return.
	MaxRowsPerQuery int64 `json:"max_rows_per_query"`
	// MaxBytesScanned caps the bytes-read budget per query (best-effort,
	// adapter-dependent).
	MaxBytesScanned int64 `json:"max_bytes_scanned"`
	// AuditRetentionDays controls how long audit records are retained
	// before partition pruning.
	AuditRetentionDays int `json:"audit_retention_days"`
	// DefaultPageSize is applied when a search request omits an explicit limit.
	DefaultPageSize int `json:"default_page_size"`
}
