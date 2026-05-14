// Agent is a defined AI assistant configuration. The agent runtime composes
// tools from the ontology subset declared here, filters by the actor's policy,
// and dispatches model calls using the configured ModelConfig.

package types

import "time"

// AgentID is the canonical handle for an agent definition.
type AgentID string

// Agent is the configuration of a single AI assistant.
type Agent struct {
	ID                        AgentID                  `json:"id"`
	WorkspaceID               WorkspaceID              `json:"workspace_id"`
	APIName                   APIName                  `json:"api_name"`
	DisplayName               string                   `json:"display_name,omitempty"`
	Description               string                   `json:"description,omitempty"`
	SystemPrompt              string                   `json:"system_prompt"`
	Model                     ModelConfig              `json:"model"`
	FromObjectTypes           []APIName                `json:"from_object_types,omitempty"`
	FromLinkTypes             []APIName                `json:"from_link_types,omitempty"`
	FromActions               []APIName                `json:"from_actions,omitempty"`
	CustomTools               []APIName                `json:"custom_tools,omitempty"`
	ContextSources            []AgentContextSource     `json:"context_sources,omitempty"`
	Memory                    AgentMemoryConfig        `json:"memory,omitempty"`
	Planning                  AgentPlanningConfig      `json:"planning,omitempty"`
	Compaction                AgentCompactionConfig    `json:"compaction,omitempty"`
	Subagents                 AgentSubagentConfig      `json:"subagents,omitempty"`
	Communication             AgentCommunicationConfig `json:"communication,omitempty"`
	AllowedRoles              []APIName                `json:"allowed_roles,omitempty"`
	Limits                    AgentLimits              `json:"limits"`
	RequireApprovalForActions bool                     `json:"require_approval_for_actions,omitempty"`
	CreatedAt                 time.Time                `json:"created_at"`
	UpdatedAt                 time.Time                `json:"updated_at"`
}

// ModelConfig selects which provider and model to use for inference.
type ModelConfig struct {
	Provider    string  `json:"provider"`
	Model       string  `json:"model"`
	Temperature float64 `json:"temperature,omitempty"`
	MaxTokens   int     `json:"max_tokens,omitempty"`
}

// AgentLimits enforces per-run resource caps.
type AgentLimits struct {
	MaxToolCalls   int     `json:"max_tool_calls"`
	MaxTokens      int     `json:"max_tokens"`
	MaxCostUSD     float64 `json:"max_cost_usd"`
	TimeoutSeconds int     `json:"timeout_seconds"`
}

// AgentContextSource describes one runtime source of context the agent may
// consult or have injected into its prompt.
type AgentContextSource struct {
	Name          string         `json:"name"`
	Kind          string         `json:"kind"`
	Ref           string         `json:"ref,omitempty"`
	Description   string         `json:"description,omitempty"`
	Required      bool           `json:"required,omitempty"`
	MaxItems      int            `json:"max_items,omitempty"`
	MaxTokens     int            `json:"max_tokens,omitempty"`
	RefreshMode   string         `json:"refresh_mode,omitempty"`
	QueryTemplate string         `json:"query_template,omitempty"`
	Metadata      map[string]any `json:"metadata,omitempty"`
}

// AgentMemoryConfig describes how the runtime should retrieve and persist
// memory for the agent outside the active context window.
type AgentMemoryConfig struct {
	Enabled         bool           `json:"enabled,omitempty"`
	Namespace       string         `json:"namespace,omitempty"`
	Scope           string         `json:"scope,omitempty"`
	ReadMode        string         `json:"read_mode,omitempty"`
	WriteMode       string         `json:"write_mode,omitempty"`
	MaxEntries      int            `json:"max_entries,omitempty"`
	MaxBytes        int64          `json:"max_bytes,omitempty"`
	IncludeInPrompt bool           `json:"include_in_prompt,omitempty"`
	Summarize       bool           `json:"summarize,omitempty"`
	Metadata        map[string]any `json:"metadata,omitempty"`
}

// AgentPlanningConfig controls plan generation, updates, and persistence.
type AgentPlanningConfig struct {
	Enabled              bool           `json:"enabled,omitempty"`
	Mode                 string         `json:"mode,omitempty"`
	GoalPrompt           string         `json:"goal_prompt,omitempty"`
	ReplanAfterToolCalls int            `json:"replan_after_tool_calls,omitempty"`
	PersistPlan          bool           `json:"persist_plan,omitempty"`
	Metadata             map[string]any `json:"metadata,omitempty"`
}

// AgentCompactionConfig controls how long-running conversations are compacted.
type AgentCompactionConfig struct {
	Enabled                bool           `json:"enabled,omitempty"`
	TriggerTokens          int            `json:"trigger_tokens,omitempty"`
	PreserveRecentMessages int            `json:"preserve_recent_messages,omitempty"`
	PreserveToolCalls      int            `json:"preserve_tool_calls,omitempty"`
	SummaryPrompt          string         `json:"summary_prompt,omitempty"`
	Metadata               map[string]any `json:"metadata,omitempty"`
}

// AgentSubagentConfig configures delegation to other agents.
type AgentSubagentConfig struct {
	Enabled               bool      `json:"enabled,omitempty"`
	AgentRefs             []APIName `json:"agent_refs,omitempty"`
	MaxConcurrent         int       `json:"max_concurrent,omitempty"`
	CommunicationMode     string    `json:"communication_mode,omitempty"`
	SharedMemoryNamespace string    `json:"shared_memory_namespace,omitempty"`
}

// AgentCommunicationConfig declares how runs exchange messages and status.
type AgentCommunicationConfig struct {
	Channels []AgentCommunicationChannel `json:"channels,omitempty"`
}

// AgentCommunicationChannel describes one persistent or ephemeral mailbox.
type AgentCommunicationChannel struct {
	Name      string         `json:"name"`
	Kind      string         `json:"kind"`
	Scope     string         `json:"scope,omitempty"`
	Retention string         `json:"retention,omitempty"`
	Metadata  map[string]any `json:"metadata,omitempty"`
}
