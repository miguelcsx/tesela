//! Agent, custom tool, and related types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};

use crate::ActionHandler;

/// An AI agent definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Model identifier (e.g., "claude-sonnet-4-7").
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub model: Option<String>,
    /// Model provider identifier (e.g., "anthropic", "openai").
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub model_provider: Option<String>,
    /// System prompt / instructions.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub instructions: Option<String>,
    /// Allowed tool API names.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub allowed_tools: Vec<ApiName>,
    /// Custom tool API names available to this agent.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub custom_tools: Vec<ApiName>,
    /// Context sources.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub context_sources: Vec<ContextSource>,
    /// Memory configuration.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub memory: Option<AgentMemory>,
    /// Execution limits.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limits: Option<AgentLimits>,
    /// Whether actions require human approval.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub requires_approval: Option<bool>,
    /// JSON Schema that the agent's final output must validate against.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_schema: Option<Value>,
    /// Object type to auto-upsert when the agent completes with structured output.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_object_type: Option<ApiName>,
    /// Capability tags used by the orchestrator for capability-based routing.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub capabilities: Vec<String>,
    /// Deprecation timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// Agent memory configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMemory {
    /// Whether memory is enabled.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub enabled: Option<bool>,
    /// Memory namespace.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub namespace: Option<String>,
    /// Memory scope.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scope: Option<String>,
}

/// Agent execution limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentLimits {
    /// Maximum number of tool calls.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_tool_calls: Option<i32>,
    /// Maximum tokens.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_tokens: Option<i32>,
    /// Maximum cost in USD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_cost_usd: Option<f64>,
    /// Timeout in seconds.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timeout_seconds: Option<i32>,
    /// Sampling temperature (0.0-1.0).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub temperature: Option<f64>,
    /// Token budget for context-window compaction.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub token_budget: Option<u32>,
}

/// A context source for an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSource {
    /// Source name.
    pub name: String,
    /// Source kind (object_type, link_type, action, etc.).
    pub kind: String,
    /// Reference API name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub r#ref: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Maximum items to include.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_items: Option<i32>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A custom tool that agents can invoke.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomTool {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Tool kind (sql, webhook, composite, callback).
    pub kind: String,
    /// Input JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_schema: Option<Value>,
    /// Output JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_schema: Option<Value>,
    /// Handler configuration.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub handler: Option<ActionHandler>,
    /// Deprecation timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}
