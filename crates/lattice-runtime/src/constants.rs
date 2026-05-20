//! Shared constants used across the runtime.

// ── Tool names ───────────────────────────────────────────────────────

/// Tool: store a key-value pair in agent memory.
pub const TOOL_MEMORY_REMEMBER: &str = "memory_remember";

/// Tool: recall a value from agent memory by key.
pub const TOOL_MEMORY_RECALL: &str = "memory_recall";

/// Tool: search agent memory by query.
pub const TOOL_MEMORY_SEARCH: &str = "memory_search";

/// Tool: delete a key from agent memory.
pub const TOOL_MEMORY_FORGET: &str = "memory_forget";

/// Tool: send a message to a channel.
pub const TOOL_CHANNEL_SEND: &str = "channel_send";

/// Tool: receive a message from a channel.
pub const TOOL_CHANNEL_RECEIVE: &str = "channel_receive";

/// Tool: spawn a sub-agent.
pub const TOOL_SUBAGENT_SPAWN: &str = "subagent_spawn";

/// Tool: wait for a sub-agent run to complete.
pub const TOOL_SUBAGENT_WAIT: &str = "subagent_wait";

// ── Tool-name prefixes (operation inference) ─────────────────────────

/// Prefix for search tools generated from the ontology.
pub const TOOL_PREFIX_SEARCH: &str = "search_";

/// Prefix for get tools generated from the ontology.
pub const TOOL_PREFIX_GET: &str = "get_";

/// Prefix for mutate tools generated from the ontology.
pub const TOOL_PREFIX_MUTATE: &str = "mutate_";

/// Prefix for aggregate tools generated from the ontology.
pub const TOOL_PREFIX_AGGREGATE: &str = "aggregate_";

/// Prefix for traverse tools generated from the ontology.
pub const TOOL_PREFIX_TRAVERSE: &str = "traverse_";

/// Prefix for execute tools generated from the ontology.
pub const TOOL_PREFIX_EXECUTE: &str = "execute_";

// ── Resource kinds ───────────────────────────────────────────────────

/// Resource kind used when evaluating tool-level policy.
pub const RESOURCE_KIND_TOOL: &str = "tool";

// ── Message roles ────────────────────────────────────────────────────

/// Role: system message.
pub const ROLE_SYSTEM: &str = "system";

/// Role: user message.
pub const ROLE_USER: &str = "user";

/// Role: assistant message.
pub const ROLE_ASSISTANT: &str = "assistant";

/// Role: tool result message.
pub const ROLE_TOOL: &str = "tool";

// ── Agent defaults ───────────────────────────────────────────────────

/// Estimated characters per token for byte-level approximation.
pub const CHARS_PER_TOKEN: usize = 4;

/// Default maximum tool calls per agent run.
pub const DEFAULT_MAX_TOOL_CALLS: usize = 50;

/// Default compaction threshold (number of messages before compaction fires).
pub const DEFAULT_COMPACTION_THRESHOLD: usize = 40;

/// Default channel receive timeout in milliseconds.
pub const DEFAULT_CHANNEL_TIMEOUT_MS: u64 = 5000;

/// Default sub-agent wait timeout in milliseconds.
pub const DEFAULT_SUBAGENT_TIMEOUT_MS: u64 = 30_000;

/// Default max tokens for compaction summaries.
pub const DEFAULT_COMPACTION_MAX_TOKENS: i32 = 512;

// ── Prompts ──────────────────────────────────────────────────────────

/// System prompt used by the rolling compactor to summarise conversation excerpts.
pub const COMPACTION_SYSTEM_PROMPT: &str =
    "Summarise the following conversation excerpt concisely.";

// ── Agent run statuses ───────────────────────────────────────────────

/// Status: agent run completed successfully.
pub const AGENT_STATUS_COMPLETED: &str = "completed";

// ── Approval reasons ─────────────────────────────────────────────────

/// Reason attached to approval requests for high-risk agent execution.
pub const APPROVAL_REASON_HIGH_RISK_AGENT: &str = "high-risk agent execution";
