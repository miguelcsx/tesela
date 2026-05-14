// Upload + ingestion runtime entities.

package types

import "time"

// UploadID is the canonical handle for an upload.
type UploadID string

// UploadStatus is the state of an upload in the ingestion lifecycle.
//
// Transitions are linear, with terminal states {Completed, Failed}:
//
//	Initiated → Pending → Uploaded → Discovering → ReadyForMapping →
//	  MappingConfirmed → Validating → Loading → ValidatingPost →
//	  Committing → Completed
//	(any) → Failed
type UploadStatus string

const (
	UploadStatusInitiated        UploadStatus = "initiated"
	UploadStatusPending          UploadStatus = "pending"
	UploadStatusUploaded         UploadStatus = "uploaded"
	UploadStatusDiscovering      UploadStatus = "discovering"
	UploadStatusReadyForMapping  UploadStatus = "ready_for_mapping"
	UploadStatusMappingConfirmed UploadStatus = "mapping_confirmed"
	UploadStatusValidating       UploadStatus = "validating"
	UploadStatusLoading          UploadStatus = "loading"
	UploadStatusValidatingPost   UploadStatus = "validating_post"
	UploadStatusCommitting       UploadStatus = "committing"
	UploadStatusCompleted        UploadStatus = "completed"
	UploadStatusFailed           UploadStatus = "failed"
)

// IsTerminal reports whether the upload has reached a final state.
func (s UploadStatus) IsTerminal() bool {
	return s == UploadStatusCompleted || s == UploadStatusFailed
}

// Upload is a single file ingestion attempt.
type Upload struct {
	ID                    UploadID          `json:"id"`
	WorkspaceID           WorkspaceID       `json:"workspace_id"`
	Asset                 APIName           `json:"asset"`
	Status                UploadStatus      `json:"status"`
	StorageURL            string            `json:"storage_url,omitempty"`
	SignedURL             string            `json:"signed_url,omitempty"`
	SignedURLExpires      *time.Time        `json:"signed_url_expires,omitempty"`
	DiscoveredSchema      *DiscoveredSchema `json:"discovered_schema,omitempty"`
	ColumnMapping         []ColumnMapping   `json:"column_mapping,omitempty"`
	ProposedColumnMapping []ColumnMapping   `json:"proposed_column_mapping,omitempty"`
	MappingConfidence     float64           `json:"mapping_confidence,omitempty"`
	MappingProposedAt     *time.Time        `json:"mapping_proposed_at,omitempty"`
	MappingModelConfig    *ModelConfig      `json:"mapping_model_config,omitempty"`
	ErrorReportURL        string            `json:"error_report_url,omitempty"`
	ErrorMessage          string            `json:"error_message,omitempty"`
	ActorUserID           string            `json:"actor_user_id"`
	Metadata              map[string]any    `json:"metadata,omitempty"`
	CreatedAt             time.Time         `json:"created_at"`
	UpdatedAt             time.Time         `json:"updated_at"`
}

// DiscoveredSchema is the result of inspecting an uploaded file.
type DiscoveredSchema struct {
	Format   string             `json:"format"`
	Columns  []DiscoveredColumn `json:"columns"`
	Metadata map[string]any     `json:"metadata,omitempty"`
}

// DiscoveredColumn is one column observed in the upload sample.
type DiscoveredColumn struct {
	Name           string         `json:"name"`
	InferredType   DataType       `json:"inferred_type"`
	NullRate       float64        `json:"null_rate"`
	UniqueRate     float64        `json:"unique_rate"`
	ObservedCount  int            `json:"observed_count,omitempty"`
	DistinctCount  int            `json:"distinct_count,omitempty"`
	TypeConfidence float64        `json:"type_confidence,omitempty"`
	MinValue       string         `json:"min_value,omitempty"`
	MaxValue       string         `json:"max_value,omitempty"`
	SampleValues   []string       `json:"sample_values,omitempty"`
	Metadata       map[string]any `json:"metadata,omitempty"`
}

// AssetVersion records a committed snapshot of an asset's data.
type AssetVersion struct {
	ID          string         `json:"id"`
	WorkspaceID WorkspaceID    `json:"workspace_id"`
	AssetID     AssetID        `json:"asset_id"`
	UploadID    UploadID       `json:"upload_id"`
	RowCount    int64          `json:"row_count"`
	Status      string         `json:"status"` // staging | published | invalidated
	Lineage     map[string]any `json:"lineage,omitempty"`
	Metadata    map[string]any `json:"metadata,omitempty"`
	Committed   *time.Time     `json:"committed,omitempty"`
	CreatedAt   time.Time      `json:"created_at"`
	UpdatedAt   time.Time      `json:"updated_at"`
}
