//! Tool assembly and orchestration for agent runtime.

use crate::constants::*;
use crate::ports::SubagentRuntime;
use tesela_core::Error;
use tesela_core::{ApiName, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Routes sub-agent spawns by capability tags declared on agent definitions.
pub struct AgentOrchestrator {
    capability_index: BTreeMap<String, ApiName>,
    subagent_runtime: Arc<dyn SubagentRuntime>,
}

impl AgentOrchestrator {
    /// Build an orchestrator from the agent index.
    pub fn new(agents: &[tesela_ir::Agent], subagent_runtime: Arc<dyn SubagentRuntime>) -> Self {
        let mut capability_index = BTreeMap::new();
        for agent in agents {
            for cap in &agent.capabilities {
                capability_index
                    .entry(cap.clone())
                    .or_insert_with(|| agent.api_name.clone());
            }
        }
        Self {
            capability_index,
            subagent_runtime,
        }
    }

    /// Return the agent API name that can handle `capability`, if any.
    pub fn route(&self, capability: &str) -> Option<&ApiName> {
        self.capability_index.get(capability)
    }

    /// Spawn the best-match agent for `capability`.
    pub fn spawn_by_capability(
        &self,
        _capability: &str,
        agent_def: &tesela_ir::Agent,
        input: Value,
        parent_run_id: &str,
    ) -> Result<String, Error> {
        self.subagent_runtime.spawn(agent_def, input, parent_run_id)
    }
}

/// Build the memory tool definitions.
pub(crate) fn memory_tools() -> Vec<crate::query::ToolDef> {
    vec![
        crate::query::ToolDef {
            name: TOOL_MEMORY_REMEMBER.to_string(),
            description: "Store a key-value pair in memory.".to_string(),
            parameters: Value::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string"},
                    "value": {"type": "string"},
                },
                "required": ["key", "value"]
            })),
        },
        crate::query::ToolDef {
            name: TOOL_MEMORY_RECALL.to_string(),
            description: "Recall a value from memory by key.".to_string(),
            parameters: Value::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string"},
                },
                "required": ["key"]
            })),
        },
        crate::query::ToolDef {
            name: TOOL_MEMORY_SEARCH.to_string(),
            description: "Search memory by query.".to_string(),
            parameters: Value::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                },
                "required": ["query"]
            })),
        },
        crate::query::ToolDef {
            name: TOOL_MEMORY_FORGET.to_string(),
            description: "Delete a key from memory.".to_string(),
            parameters: Value::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string"},
                },
                "required": ["key"]
            })),
        },
    ]
}

/// Build the channel tool definitions.
pub(crate) fn channel_tools() -> Vec<crate::query::ToolDef> {
    vec![
        crate::query::ToolDef {
            name: TOOL_CHANNEL_SEND.to_string(),
            description: "Send a message to a channel.".to_string(),
            parameters: Value::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "channel": {"type": "string"},
                    "message": {"type": "string"},
                },
                "required": ["channel", "message"]
            })),
        },
        crate::query::ToolDef {
            name: TOOL_CHANNEL_RECEIVE.to_string(),
            description: "Receive a message from a channel.".to_string(),
            parameters: Value::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "channel": {"type": "string"},
                    "timeout_ms": {"type": "integer"},
                },
                "required": ["channel", "timeout_ms"]
            })),
        },
    ]
}

/// Build the subagent tool definitions.
pub(crate) fn subagent_tools() -> Vec<crate::query::ToolDef> {
    vec![
        crate::query::ToolDef {
            name: TOOL_SUBAGENT_SPAWN.to_string(),
            description: "Spawn a subagent.".to_string(),
            parameters: Value::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_name": {"type": "string"},
                    "input": {"type": "object"},
                },
                "required": ["agent_name", "input"]
            })),
        },
        crate::query::ToolDef {
            name: TOOL_SUBAGENT_WAIT.to_string(),
            description: "Wait for a subagent run to complete.".to_string(),
            parameters: Value::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "run_id": {"type": "string"},
                    "timeout_ms": {"type": "integer"},
                },
                "required": ["run_id", "timeout_ms"]
            })),
        },
    ]
}
