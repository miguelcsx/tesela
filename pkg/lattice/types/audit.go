// AuditRecord is a single immutable entry in the audit log. The store enforces
// append-only semantics at the database level; this struct is the in-memory
// shape produced by the audit writer.

package types

import "time"

// AuditDecision summarizes the policy outcome attached to an audited event.
type AuditDecision string

const (
	AuditDecisionAllow AuditDecision = "allow"
	AuditDecisionDeny  AuditDecision = "deny"
)

// AuditRecord is a single audit entry.
type AuditRecord struct {
	ID                 string         `json:"id"`
	WorkspaceID        WorkspaceID    `json:"workspace_id"`
	OccurredAt         time.Time      `json:"occurred_at"`
	RequestID          string         `json:"request_id,omitempty"`
	TraceID            string         `json:"trace_id,omitempty"`
	ActorUserID        string         `json:"actor_user_id"`
	ActorRoles         []string       `json:"actor_roles,omitempty"`
	Operation          Operation      `json:"operation"`
	ResourceKind       string         `json:"resource_kind"` // e.g., "object_type", "action_type"
	ResourceAPIName    APIName        `json:"resource_api_name,omitempty"`
	SubjectKey         string         `json:"subject_key,omitempty"`
	PolicyDecision     AuditDecision  `json:"policy_decision"`
	MatchedRules       []APIName      `json:"matched_rules,omitempty"`
	RedactedProperties []APIName      `json:"redacted_properties,omitempty"`
	ResultCount        int64          `json:"result_count,omitempty"`
	DurationMS         int64          `json:"duration_ms,omitempty"`
	ErrorCode          string         `json:"error_code,omitempty"`
	ActionRunID        string         `json:"action_run_id,omitempty"`
	AgentRunID         string         `json:"agent_run_id,omitempty"`
	UploadID           UploadID       `json:"upload_id,omitempty"`
	Metadata           map[string]any `json:"metadata,omitempty"`
}
