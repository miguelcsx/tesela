// Asset is a dataset declaration: schema, quality rules, ingestion sink, and
// optional column mapping. Assets are populated by the upload pipeline.

package types

import (
	"encoding/json"
	"time"
)

// AssetID is the canonical handle for an asset.
type AssetID string

// Asset is the metadata that drives the upload + ingestion pipeline.
type Asset struct {
	ID                   AssetID           `json:"id"`
	WorkspaceID          WorkspaceID       `json:"workspace_id"`
	APIName              APIName           `json:"api_name"`
	DisplayName          string            `json:"display_name,omitempty"`
	Description          string            `json:"description,omitempty"`
	Metadata             map[string]any    `json:"metadata,omitempty"`
	Tags                 []string          `json:"tags,omitempty"`
	Properties           []Property        `json:"properties"`
	QualityRules         []QualityRule     `json:"quality_rules,omitempty"`
	Dependencies         []AssetDependency `json:"dependencies,omitempty"`
	Sink                 AssetSink         `json:"sink"`
	SavedColumnMapping   []ColumnMapping   `json:"saved_column_mapping,omitempty"`
	UnmappedColumnPolicy string            `json:"unmapped_column_policy,omitempty"` // warn | error | ignore
	CreatedAt            time.Time         `json:"created_at"`
	UpdatedAt            time.Time         `json:"updated_at"`
}

// AssetSink describes where ingested data is written.
type AssetSink struct {
	DatasourceAPIName APIName `json:"datasource"`
	Schema            string  `json:"schema,omitempty"`
	Table             string  `json:"table"`
}

// QualityRule is an asset-level data quality predicate.
type QualityRule struct {
	APIName     APIName             `json:"api_name"`
	Kind        QualityRuleKind     `json:"kind"`
	Property    APIName             `json:"property,omitempty"`
	Severity    QualityRuleSeverity `json:"severity"`
	Args        json.RawMessage     `json:"args,omitempty"`
	Description string              `json:"description,omitempty"`
	Metadata    map[string]any      `json:"metadata,omitempty"`
}

// QualityRuleKind selects the evaluator namespace. Lattice treats this as an
// open string so integrations can register domain-specific rules.
type QualityRuleKind string

const (
	QualityRuleKindNotNull       QualityRuleKind = "not_null"
	QualityRuleKindUnique        QualityRuleKind = "unique"
	QualityRuleKindRange         QualityRuleKind = "range"
	QualityRuleKindAllowedValues QualityRuleKind = "allowed_values"
	QualityRuleKindRegex         QualityRuleKind = "regex"
	QualityRuleKindCustomCEL     QualityRuleKind = "custom_cel"
)

// QualityRuleSeverity decides whether failures block the load.
type QualityRuleSeverity string

const (
	QualityRuleSeverityError   QualityRuleSeverity = "error"
	QualityRuleSeverityWarning QualityRuleSeverity = "warning"
)

// ColumnMapping maps a source file column to a target asset property.
type ColumnMapping struct {
	SourceColumn   string              `json:"source_column"`
	TargetProperty APIName             `json:"target_property"`
	Required       bool                `json:"required,omitempty"`
	TypeCoercion   string              `json:"type_coercion,omitempty"`
	ValueMapping   map[string]string   `json:"value_mapping,omitempty"`
	Transforms     []PropertyTransform `json:"transforms,omitempty"`
}

// AssetDependency captures a declared relationship to another asset or data
// producer so callers can build dependency graphs and impact analysis.
type AssetDependency struct {
	Kind        string         `json:"kind"`
	Target      string         `json:"target"`
	Description string         `json:"description,omitempty"`
	Metadata    map[string]any `json:"metadata,omitempty"`
}
