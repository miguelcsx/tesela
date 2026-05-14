// CustomTool is a user-defined capability available to agents.

package types

import (
	"encoding/json"
	"time"
)

// CustomToolID is the canonical handle for a custom tool.
type CustomToolID string

// CustomToolKind selects the tool's executor.
type CustomToolKind string

const (
	CustomToolKindSQL       CustomToolKind = "sql"
	CustomToolKindWebhook   CustomToolKind = "webhook"
	CustomToolKindComposite CustomToolKind = "composite"
	CustomToolKindCallback  CustomToolKind = "callback"
)

// CustomTool is a tool that agents can invoke. The set of kinds intentionally
// mirrors action handler kinds so the same dispatch infrastructure can be
// shared.
type CustomTool struct {
	ID           CustomToolID      `json:"id"`
	WorkspaceID  WorkspaceID       `json:"workspace_id"`
	APIName      APIName           `json:"api_name"`
	DisplayName  string            `json:"display_name,omitempty"`
	Description  string            `json:"description,omitempty"`
	Kind         CustomToolKind    `json:"kind"`
	InputSchema  json.RawMessage   `json:"input_schema"`
	OutputSchema json.RawMessage   `json:"output_schema,omitempty"`
	SQL          *SQLToolSpec      `json:"sql,omitempty"`
	Webhook      *WebhookHandler   `json:"webhook,omitempty"`
	Composite    *CompositeHandler `json:"composite,omitempty"`
	CreatedAt    time.Time         `json:"created_at"`
	UpdatedAt    time.Time         `json:"updated_at"`
}

// SQLToolSpec is the configuration of a SQL-based custom tool.
type SQLToolSpec struct {
	DatasourceAPIName APIName `json:"datasource"`
	Statement         string  `json:"statement"`
}
