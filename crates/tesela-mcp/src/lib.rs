#![deny(warnings)]
#![deny(missing_docs)]

//! Model Context Protocol (MCP) server for Tesela runtime.
//!
//! Exposes the Tesela ontology as MCP tools over JSON-RPC 2.0.
//! Supports two transports:
//! - **HTTP** (`POST /mcp`): single-request JSON-RPC.
//! - **stdio**: newline-delimited JSON-RPC, suitable for subprocess MCP clients.
//!
//! # Tool naming convention
//!
//! | Ontology entity | MCP tools generated |
//! |---|---|
//! | Object type `foo` | `search_foo`, `get_foo`, `aggregate_foo` |
//! | Link type `bar` | `traverse_bar` |
//! | Action `baz` | `execute_baz` |
//! | Agent `qux` | `agent_start_qux`, `agent_get_run_qux` |

mod handlers;
mod tools;
mod types;

pub use handlers::McpServer;
pub use types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpTool};

#[cfg(test)]
mod tests {
    use super::*;
    use tesela_ir::Spec;
    use tesela_runtime::runtime::Runtime;

    fn make_server() -> McpServer {
        let spec = Spec::default();
        let opts = tesela_runtime::runtime::RuntimeOptions::dev();
        let rt = Runtime::new(spec, opts).unwrap();
        McpServer::new(rt)
    }

    #[test]
    fn test_initialize() {
        let server = make_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "initialize".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = server.handle(req);
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
    }

    #[test]
    fn test_tools_list_empty_spec() {
        let server = make_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/list".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = server.handle(req);
        assert!(resp.error.is_none());
        let tools = &resp.result.unwrap()["tools"];
        assert!(tools.is_array());
        assert_eq!(tools.as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_unknown_method() {
        let server = make_server();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "nonsense".to_string(),
            params: serde_json::Value::Null,
        };
        let resp = server.handle(req);
        assert!(resp.error.is_some());
    }
}
