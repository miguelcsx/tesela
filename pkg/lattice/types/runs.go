// ActionRun and AgentRun runtime entities + their state machines.

package types

import (
	"encoding/json"
	"time"
)

// RunStatus is the state of an ActionRun or AgentRun.
type RunStatus string

const (
	RunStatusPending          RunStatus = "pending"
	RunStatusRunning          RunStatus = "running"
	RunStatusAwaitingApproval RunStatus = "awaiting_approval"
	RunStatusDone             RunStatus = "done"
	RunStatusFailed           RunStatus = "failed"
	RunStatusCancelled        RunStatus = "cancelled"
)

// IsTerminal reports whether the status represents a final state.
func (s RunStatus) IsTerminal() bool {
	switch s {
	case RunStatusDone, RunStatusFailed, RunStatusCancelled:
		return true
	default:
		return false
	}
}

// ActionRunID is the canonical handle for an action run.
type ActionRunID string

// ActionRun is a single execution attempt of an action type.
type ActionRun struct {
	ID             ActionRunID     `json:"id"`
	WorkspaceID    WorkspaceID     `json:"workspace_id"`
	ActionType     APIName         `json:"action_type"`
	IdempotencyKey string          `json:"idempotency_key"`
	Subject        string          `json:"subject,omitempty"`
	ActorUserID    string          `json:"actor_user_id"`
	ActorRoles     []string        `json:"actor_roles,omitempty"`
	Input          json.RawMessage `json:"input"`
	Output         json.RawMessage `json:"output,omitempty"`
	Status         RunStatus       `json:"status"`
	ErrorCode      string          `json:"error_code,omitempty"`
	ErrorMessage   string          `json:"error_message,omitempty"`
	StartedAt      *time.Time      `json:"started_at,omitempty"`
	FinishedAt     *time.Time      `json:"finished_at,omitempty"`
	CreatedAt      time.Time       `json:"created_at"`
	UpdatedAt      time.Time       `json:"updated_at"`
}

// AgentRunID is the canonical handle for an agent run.
type AgentRunID string

// AgentRun is a single execution of an agent.
type AgentRun struct {
	ID            AgentRunID      `json:"id"`
	WorkspaceID   WorkspaceID     `json:"workspace_id"`
	Agent         APIName         `json:"agent"`
	ParentRunID   AgentRunID      `json:"parent_run_id,omitempty"`
	ActorUserID   string          `json:"actor_user_id"`
	ActorRoles    []string        `json:"actor_roles,omitempty"`
	Input         json.RawMessage `json:"input"`
	Plan          json.RawMessage `json:"plan,omitempty"`
	ContextRefs   json.RawMessage `json:"context_refs,omitempty"`
	MemoryRefs    json.RawMessage `json:"memory_refs,omitempty"`
	FinalResponse string          `json:"final_response,omitempty"`
	Status        RunStatus       `json:"status"`
	ErrorCode     string          `json:"error_code,omitempty"`
	ErrorMessage  string          `json:"error_message,omitempty"`
	TokensUsed    int             `json:"tokens_used"`
	ToolCallCount int             `json:"tool_call_count"`
	CostUSD       float64         `json:"cost_usd"`
	StartedAt     *time.Time      `json:"started_at,omitempty"`
	FinishedAt    *time.Time      `json:"finished_at,omitempty"`
	CreatedAt     time.Time       `json:"created_at"`
	UpdatedAt     time.Time       `json:"updated_at"`
}

// ToolCallTrace is one entry in the tool execution trace of an agent run.
type ToolCallTrace struct {
	ID             string          `json:"id"`
	AgentRunID     AgentRunID      `json:"agent_run_id"`
	Sequence       int             `json:"sequence"`
	ToolName       string          `json:"tool_name"`
	Input          json.RawMessage `json:"input"`
	Output         json.RawMessage `json:"output,omitempty"`
	Status         RunStatus       `json:"status"`
	PolicyDecision AuditDecision   `json:"policy_decision"`
	DurationMS     int64           `json:"duration_ms"`
	ErrorMessage   string          `json:"error_message,omitempty"`
	OccurredAt     time.Time       `json:"occurred_at"`
}

// AgentMessageTrace is one message, note, summary, or coordination event in an
// agent run. This makes planning, memory, and subagent communication observable.
type AgentMessageTrace struct {
	ID         string          `json:"id"`
	AgentRunID AgentRunID      `json:"agent_run_id"`
	Sequence   int             `json:"sequence"`
	Role       string          `json:"role"`
	Kind       string          `json:"kind"`
	Content    string          `json:"content,omitempty"`
	Name       string          `json:"name,omitempty"`
	ToolCallID string          `json:"tool_call_id,omitempty"`
	Metadata   json.RawMessage `json:"metadata,omitempty"`
	OccurredAt time.Time       `json:"occurred_at"`
}

// AgentMemoryRecord is one persisted memory item for an agent namespace.
type AgentMemoryRecord struct {
	ID          string         `json:"id"`
	WorkspaceID WorkspaceID    `json:"workspace_id"`
	Namespace   string         `json:"namespace"`
	Scope       string         `json:"scope,omitempty"`
	ActorUserID string         `json:"actor_user_id,omitempty"`
	Agent       APIName        `json:"agent,omitempty"`
	Kind        string         `json:"kind"`
	Content     string         `json:"content"`
	Summary     string         `json:"summary,omitempty"`
	Metadata    map[string]any `json:"metadata,omitempty"`
	CreatedAt   time.Time      `json:"created_at"`
	UpdatedAt   time.Time      `json:"updated_at"`
}
