//! MCP tool generators from Lattice ontology entities.

use crate::types::McpTool;
use lattice_ir::{
    ActionType, Agent, AggregateView, ArtifactType, JobType, LinkType, ObjectType, UploadFlow,
};

/// Generate search, get, aggregate, and describe tools for an object type.
pub(crate) fn object_type_tools(ot: &ObjectType) -> Vec<McpTool> {
    let name = ot.api_name.as_ref();
    let display = ot.display.as_deref().unwrap_or(name);

    vec![
        McpTool {
            name: format!("search_{}", name),
            description: format!(
                "Search {} records with optional filters and pagination.",
                display
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max records to return." },
                    "offset": { "type": "integer", "description": "Records to skip." }
                }
            }),
        },
        McpTool {
            name: format!("get_{}", name),
            description: format!("Get a single {} record by primary key.", display),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["pk"],
                "properties": {
                    "pk": { "type": "string", "description": "Primary key value." }
                }
            }),
        },
        McpTool {
            name: format!("aggregate_{}", name),
            description: format!("Aggregate {} records.", display),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        McpTool {
            name: format!("describe_{}", name),
            description: format!("Return the schema definition for {}.", display),
            input_schema: serde_json::json!({ "type": "object", "properties": {} }),
        },
    ]
}

/// Generate a traversal tool for a link type.
pub(crate) fn link_type_tool(lt: &LinkType) -> McpTool {
    let name = lt.api_name.as_ref();
    McpTool {
        name: format!("traverse_{}", name),
        description: format!(
            "Traverse link '{}' from a source record to related target records.",
            name
        ),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["source_pk"],
            "properties": {
                "source_pk": { "description": "Primary key of the source record." },
                "limit": { "type": "integer" },
                "offset": { "type": "integer" }
            }
        }),
    }
}

/// Generate an execution tool for an action type.
pub(crate) fn action_tool(action: &ActionType) -> McpTool {
    McpTool {
        name: format!("execute_{}", action.api_name),
        description: action
            .description
            .clone()
            .unwrap_or_else(|| format!("Execute action '{}'.", action.api_name)),
        input_schema: action
            .input_schema
            .as_ref()
            .map(|v| v.0.clone())
            .unwrap_or_else(|| {
                serde_json::json!({
                    "type": "object",
                    "properties": { "input": { "description": "Action input payload." } }
                })
            }),
    }
}

/// Generate start and get-run tools for an agent.
pub(crate) fn agent_tools(agent: &Agent) -> Vec<McpTool> {
    let name = agent.api_name.as_ref();
    let display = agent.display.as_deref().unwrap_or(name);

    vec![
        McpTool {
            name: format!("agent_start_{}", name),
            description: format!("Start an agent run for '{}'.", display),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "description": "Agent input payload." }
                }
            }),
        },
        McpTool {
            name: format!("agent_get_run_{}", name),
            description: format!("Get the result of a '{}' agent run by run_id.", display),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["run_id"],
                "properties": {
                    "run_id": { "type": "string", "description": "Run ID returned by agent_start." }
                }
            }),
        },
    ]
}

/// Generate a tool for a named aggregate view.
pub(crate) fn aggregate_view_tool(view: &AggregateView) -> McpTool {
    McpTool {
        name: format!("aggregate_view_{}", view.api_name),
        description: format!("Execute aggregate view '{}'.", view.api_name),
        input_schema: serde_json::json!({"type": "object", "properties": {}}),
    }
}

/// Generate a tool for an artifact locator.
pub(crate) fn artifact_tool(artifact: &ArtifactType) -> McpTool {
    McpTool {
        name: format!("artifact_read_{}", artifact.api_name),
        description: format!("Authorize and locate artifact '{}'.", artifact.api_name),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "params": {"type": "object"},
                "ttl": {"type": "integer"}
            }
        }),
    }
}

/// Generate a tool for an upload flow.
pub(crate) fn upload_flow_tool(flow: &UploadFlow) -> McpTool {
    McpTool {
        name: format!("upload_flow_{}", flow.api_name),
        description: format!("Initiate upload flow '{}'.", flow.api_name),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "params": {"type": "object"},
                "ttl": {"type": "integer"}
            }
        }),
    }
}

/// Generate a tool for a job type.
pub(crate) fn job_tool(job: &JobType) -> McpTool {
    McpTool {
        name: format!("start_job_{}", job.api_name),
        description: format!("Start job '{}'.", job.api_name),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "input": {"type": "object"},
                "idempotency_key": {"type": "string"}
            }
        }),
    }
}
