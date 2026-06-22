//! Tool assembly and orchestration for agent runtime.

use crate::constants::*;
use crate::ports::SubagentRuntime;
use std::collections::BTreeMap;
use std::sync::Arc;
use tesela_core::Error;
use tesela_core::{ApiName, Value};

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

/// Build ontology runtime tool definitions for agents.
///
/// These tools are backend-agnostic: callers execute them through their
/// `Runtime`/backend wiring and keep product-specific authorization outside
/// this catalog.
#[must_use]
pub fn ontology_tools() -> Vec<crate::query::ToolDef> {
    vec![
        tool(
            "tesela.spec",
            "Return the active Tesela ontology spec with object types, datasources, and properties.",
            serde_json::json!({"type": "object"}),
        ),
        tool(
            "tesela.search",
            "Search records for any Tesela object type.",
            serde_json::json!({
                "type": "object",
                "required": ["object_type"],
                "properties": {
                    "object_type": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 100}
                }
            }),
        ),
        tool(
            "tesela.get",
            "Get one record by object type and id.",
            serde_json::json!({
                "type": "object",
                "required": ["object_type", "id"],
                "properties": {
                    "object_type": {"type": "string"},
                    "id": {"type": "string"}
                }
            }),
        ),
        tool(
            "tesela.aggregate",
            "Count records for any Tesela object type.",
            serde_json::json!({
                "type": "object",
                "required": ["object_type"],
                "properties": {"object_type": {"type": "string"}}
            }),
        ),
        tool(
            "tesela.object_set.resolve",
            "Resolve a named Tesela object set.",
            serde_json::json!({
                "type": "object",
                "required": ["name"],
                "properties": {"name": {"type": "string"}}
            }),
        ),
        tool(
            "tesela.object_set.compose",
            "Compose named Tesela object sets with union, intersect, or subtract.",
            serde_json::json!({
                "type": "object",
                "required": ["names", "op"],
                "properties": {
                    "names": {"type": "array", "items": {"type": "string"}},
                    "op": {"type": "string", "enum": ["union", "intersect", "subtract"]}
                }
            }),
        ),
        tool(
            "tesela.links.list",
            "List declared Tesela links between object types.",
            serde_json::json!({"type": "object"}),
        ),
        tool(
            "tesela.traverse",
            "Traverse a declared Tesela link from one source object id.",
            serde_json::json!({
                "type": "object",
                "required": ["link", "source_id"],
                "properties": {
                    "link": {"type": "string"},
                    "source_id": {"type": "string"},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 100}
                }
            }),
        ),
        tool(
            "tesela.actions.list",
            "List declared Tesela actions available in the active ontology without executing them.",
            serde_json::json!({"type": "object"}),
        ),
        tool(
            "tesela.action.describe",
            "Describe one declared Tesela action by API name without executing it.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {"action": {"type": "string"}}
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, parameters: serde_json::Value) -> crate::query::ToolDef {
    crate::query::ToolDef {
        name: name.to_string(),
        description: description.to_string(),
        parameters: Value::new(parameters),
    }
}

#[cfg(test)]
mod tests {
    use super::ontology_tools;

    #[test]
    fn ontology_tools_expose_runtime_primitives() {
        let tools = ontology_tools();
        let names = tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"tesela.spec"));
        assert!(names.contains(&"tesela.search"));
        assert!(names.contains(&"tesela.get"));
        assert!(names.contains(&"tesela.traverse"));
        assert!(names.contains(&"tesela.actions.list"));
        assert!(tools.iter().all(|tool| {
            tool.parameters
                .as_object()
                .and_then(|schema| schema.get("type"))
                .and_then(serde_json::Value::as_str)
                == Some("object")
        }),);
    }
}
