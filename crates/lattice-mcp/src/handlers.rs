//! JSON-RPC dispatch and HTTP/stdio transport handlers.

use crate::tools;
use crate::types::{JsonRpcRequest, JsonRpcResponse, McpTool};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use lattice_core::{ApiName, Error, Value};
use lattice_runtime::{
    query::{Actor, AggregateQuery, Query, TraversalQuery},
    runtime::Runtime,
};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The Lattice MCP server.
pub struct McpServer {
    runtime: Arc<Runtime>,
}

impl McpServer {
    /// Create a new MCP server wrapping the given runtime.
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }

    /// Generate the list of all available MCP tools from the current spec.
    pub fn list_tools(&self) -> Vec<McpTool> {
        let spec = self.runtime.spec();
        let mut result = Vec::new();

        for ot in &spec.object_types {
            result.extend(tools::object_type_tools(ot));
        }
        for lt in &spec.link_types {
            result.push(tools::link_type_tool(lt));
        }
        for action in &spec.actions {
            result.push(tools::action_tool(action));
        }
        for agent in &spec.agents {
            result.extend(tools::agent_tools(agent));
        }
        for view in &spec.aggregate_views {
            result.push(tools::aggregate_view_tool(view));
        }
        for artifact in &spec.artifact_types {
            result.push(tools::artifact_tool(artifact));
        }
        for flow in &spec.upload_flows {
            result.push(tools::upload_flow_tool(flow));
        }
        for job in &spec.job_types {
            result.push(tools::job_tool(job));
        }

        result
    }

    /// Handle a single JSON-RPC request and return a response.
    pub fn handle(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone();
        match self.dispatch(&req) {
            Ok(result) => JsonRpcResponse::ok(id, result),
            Err(e) => {
                let code = lattice_error_to_rpc_code(&e);
                JsonRpcResponse::err(id, code, &e.to_string())
            }
        }
    }

    fn dispatch(&self, req: &JsonRpcRequest) -> Result<serde_json::Value, Error> {
        match req.method.as_str() {
            "initialize" => Ok(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "lattice-mcp", "version": "0.1.0" }
            })),

            "tools/list" => {
                let tools = self.list_tools();
                Ok(serde_json::json!({ "tools": tools }))
            }

            "tools/call" => {
                let tool_name = req
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::bad_request("missing 'name' in params"))?
                    .to_string();

                let args = req.params.get("arguments").cloned().unwrap_or_default();
                self.call_tool(&tool_name, &args)
            }

            other => Err(Error::bad_request(format!("unknown method: {}", other))),
        }
    }

    fn call_tool(&self, name: &str, args: &serde_json::Value) -> Result<serde_json::Value, Error> {
        let actor = Actor {
            user_id: "mcp".to_string(),
            roles: vec!["admin".to_string()],
            claims: BTreeMap::new(),
        };

        let spec = self.runtime.spec();

        if let Some(obj_name) = name.strip_prefix("search_") {
            let obj = obj_name
                .parse::<ApiName>()
                .map_err(|e| Error::bad_request(format!("invalid name: {}", e)))?;
            let limit = args.get("limit").and_then(|v| v.as_i64()).map(|v| v as i32);
            let q = Query {
                limit,
                ..Default::default()
            };
            let page = self.runtime.search(&actor, &obj, q)?;
            return Ok(serde_json::to_value(&page).unwrap_or_default());
        }

        if let Some(obj_name) = name.strip_prefix("get_") {
            let obj = obj_name
                .parse::<ApiName>()
                .map_err(|e| Error::bad_request(format!("invalid name: {}", e)))?;
            let pk_str = args
                .get("pk")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::bad_request("missing 'pk'"))?
                .to_string();
            let record = self.runtime.get(&actor, &obj, &Value::string(pk_str))?;
            return Ok(serde_json::to_value(&record).unwrap_or_default());
        }

        if let Some(obj_name) = name.strip_prefix("aggregate_") {
            let obj = obj_name
                .parse::<ApiName>()
                .map_err(|e| Error::bad_request(format!("invalid name: {}", e)))?;
            let q = AggregateQuery::default();
            let result = self.runtime.aggregate(&actor, &obj, q)?;
            return Ok(serde_json::to_value(&result).unwrap_or_default());
        }

        if let Some(link_name) = name.strip_prefix("traverse_") {
            let link = link_name
                .parse::<ApiName>()
                .map_err(|e| Error::bad_request(format!("invalid name: {}", e)))?;
            let source_pk = args.get("source_pk").cloned().unwrap_or_default();
            let q = TraversalQuery {
                source_pk: Value::new(source_pk),
                ..Default::default()
            };
            let page = self.runtime.traverse(&actor, &link, q)?;
            return Ok(serde_json::to_value(&page).unwrap_or_default());
        }

        if let Some(action_name) = name.strip_prefix("execute_") {
            let action = action_name
                .parse::<ApiName>()
                .map_err(|e| Error::bad_request(format!("invalid name: {}", e)))?;
            let input = Value::new(args.get("input").cloned().unwrap_or_default());
            let result = self.runtime.execute_action(&actor, &action, input)?;
            return Ok(serde_json::to_value(&result).unwrap_or_default());
        }

        if let Some(agent_name) = name.strip_prefix("agent_start_") {
            let agent = agent_name
                .parse::<ApiName>()
                .map_err(|e| Error::bad_request(format!("invalid name: {}", e)))?;
            let input = Value::new(args.get("input").cloned().unwrap_or_default());
            let run_id = self.runtime.start_agent_run(&actor, &agent, input)?;
            return Ok(serde_json::json!({ "run_id": run_id }));
        }

        if let Some(agent_name) = name.strip_prefix("agent_get_run_") {
            let _ = agent_name;
            let run_id = args
                .get("run_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::bad_request("missing 'run_id'"))?
                .to_string();
            let run = self.runtime.get_agent_run(&actor, &run_id)?;
            return Ok(serde_json::to_value(&run).unwrap_or_default());
        }

        if let Some(view_name) = name.strip_prefix("aggregate_view_") {
            let view = view_name
                .parse::<ApiName>()
                .map_err(|e| Error::bad_request(format!("invalid name: {}", e)))?;
            let result = self.runtime.aggregate_view(&actor, &view)?;
            return Ok(serde_json::to_value(&result).unwrap_or_default());
        }

        if let Some(artifact_name) = name.strip_prefix("artifact_read_") {
            let artifact = artifact_name
                .parse::<ApiName>()
                .map_err(|e| Error::bad_request(format!("invalid name: {}", e)))?;
            let params = args
                .get("params")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();
            let ttl = args.get("ttl").and_then(|v| v.as_u64()).unwrap_or(300);
            let result = self
                .runtime
                .authorize_artifact_read(&actor, &artifact, params, ttl)?;
            return Ok(serde_json::to_value(&result).unwrap_or_default());
        }

        if let Some(flow_name) = name.strip_prefix("upload_flow_") {
            let flow = flow_name
                .parse::<ApiName>()
                .map_err(|e| Error::bad_request(format!("invalid name: {}", e)))?;
            let params = args
                .get("params")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();
            let ttl = args.get("ttl").and_then(|v| v.as_u64()).unwrap_or(900);
            let result = self
                .runtime
                .initiate_upload_flow(&actor, &flow, params, ttl)?;
            return Ok(serde_json::to_value(&result).unwrap_or_default());
        }

        if let Some(job_name) = name.strip_prefix("start_job_") {
            let job = job_name
                .parse::<ApiName>()
                .map_err(|e| Error::bad_request(format!("invalid name: {}", e)))?;
            let input = args
                .get("input")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();
            let idempotency_key = args
                .get("idempotency_key")
                .and_then(|v| v.as_str())
                .map(ToString::to_string);
            let result = self
                .runtime
                .start_job(&actor, &job, input, idempotency_key)?;
            return Ok(serde_json::to_value(&result).unwrap_or_default());
        }

        if let Some(obj_name) = name.strip_prefix("describe_") {
            let obj = spec
                .object_types
                .iter()
                .find(|o| o.api_name.as_ref() == obj_name);
            return match obj {
                Some(o) => Ok(serde_json::to_value(o).unwrap_or_default()),
                None => Err(Error::not_found("object_type", obj_name)),
            };
        }

        Err(Error::not_found("tool", name))
    }

    /// Build an Axum router that handles `POST /mcp`.
    pub fn router(self) -> axum::Router {
        let server = Arc::new(self);
        axum::Router::new()
            .route("/mcp", axum::routing::post(http_handler))
            .with_state(server)
    }

    /// Listen on `addr` and serve the HTTP MCP endpoint.
    pub async fn serve(self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let addr: std::net::SocketAddr = addr.parse()?;
        tracing::info!("Lattice MCP server listening on {}", addr);
        let router = self.router();
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await?;
        Ok(())
    }

    /// Run the stdio transport: read newline-delimited JSON-RPC from stdin, write to stdout.
    pub fn run_stdio(self) {
        use std::io::{BufRead, Write};
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("stdio read error: {}", e);
                    break;
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(req) => self.handle(req),
                Err(e) => JsonRpcResponse::err(None, -32700, &format!("parse error: {}", e)),
            };
            let out = serde_json::to_string(&response).unwrap_or_default();
            let mut out_lock = stdout.lock();
            writeln!(out_lock, "{}", out).ok();
        }
    }
}

async fn http_handler(State(server): State<Arc<McpServer>>, body: axum::body::Bytes) -> Response {
    let req: JsonRpcRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            let resp = JsonRpcResponse::err(None, -32700, &format!("parse error: {}", e));
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };
    let response = server.handle(req);
    (StatusCode::OK, Json(response)).into_response()
}

fn lattice_error_to_rpc_code(e: &Error) -> i32 {
    match e {
        Error::BadRequest { .. } | Error::Validation { .. } => -32602,
        Error::NotFound { .. } => -32001,
        Error::Unauthorized { .. } => -32002,
        Error::PolicyDenied { .. } => -32003,
        Error::Conflict { .. } => -32004,
        Error::Timeout { .. } => -32005,
        Error::UnsupportedCapability { .. } => -32006,
        Error::Adapter { .. } | Error::Internal { .. } => -32000,
    }
}
