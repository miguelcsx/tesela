use tesela_core::{Error, Value};
use tesela_runtime::{ports::AgentRuntime, query::Actor};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// APXM agent runtime that delegates to an APXM HTTP service.
///
/// Maps Tesela agent executions to APXM skill executions:
///
/// - [`AgentRuntime::start_run`] → `POST /v1/skills/{skill_id}/execute`
///   (or `POST /v1/generate` as fallback for simple agents)
/// - [`AgentRuntime::get_run`] → `GET /v1/executions/{execution_id}`
///
/// The skill ID is resolved from `agent.metadata["apxm_skill_id"]`, falling
/// back to the agent's `api_name`.
pub struct ApxmAgentRuntime {
    base_url: String,
    client: reqwest::Client,
    timeout: Duration,
}

impl ApxmAgentRuntime {
    /// Create a new APXM adapter pointing at the given base URL.
    ///
    /// Uses a default timeout of 120 seconds for HTTP requests.
    pub fn new(base_url: impl Into<String>) -> Self {
        let timeout = Duration::from_secs(DEFAULT_TIMEOUT_SECS);
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap_or_default(),
            timeout,
        }
    }

    /// Create with a custom HTTP client and timeout.
    pub fn with_client(
        base_url: impl Into<String>,
        client: reqwest::Client,
        timeout: Duration,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            client,
            timeout,
        }
    }

    /// Create with a custom timeout in seconds.
    pub fn with_timeout(base_url: impl Into<String>, timeout_secs: u64) -> Self {
        let timeout = Duration::from_secs(timeout_secs);
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap_or_default(),
            timeout,
        }
    }

    fn block_on<F: std::future::Future>(&self, f: F) -> F::Output {
        tokio::runtime::Handle::current().block_on(f)
    }

    fn skill_id(agent: &tesela_ir::Agent) -> String {
        agent
            .metadata
            .as_ref()
            .and_then(|m| m.get("apxm_skill_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| agent.api_name.to_string())
    }

    fn map_reqwest_error(e: reqwest::Error) -> Error {
        if e.is_timeout() {
            Error::timeout(format!("APXM request timed out: {e}"))
        } else if e.is_connect() {
            Error::adapter(format!("APXM connection failed: {e}"))
        } else {
            Error::adapter(format!("APXM HTTP error: {e}"))
        }
    }

    async fn do_start_run(
        &self,
        agent: &tesela_ir::Agent,
        input: Value,
        actor: &Actor,
    ) -> Result<String, Error> {
        let skill_id = Self::skill_id(agent);
        let url = format!("{}/v1/skills/{}/execute", self.base_url, skill_id);

        let body = SkillExecuteRequest {
            args: Vec::new(),
            session_id: None,
            tesela_context: Some(TeselaContext {
                agent_name: agent.api_name.to_string(),
                actor_id: actor.user_id.clone(),
                input,
                instructions: agent.instructions.clone(),
                model: agent.model.clone(),
            }),
        };

        tracing::info!(
            skill_id = %skill_id,
            actor = %actor.user_id,
            "starting APXM skill execution"
        );

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(Self::map_reqwest_error)?;

        let status = resp.status();
        let resp_text = resp.text().await.map_err(Self::map_reqwest_error)?;

        if !status.is_success() {
            let apxm_err: Option<ApxmErrorResponse> = serde_json::from_str(&resp_text).ok();
            let msg = apxm_err
                .and_then(|e| e.error.map(|e| e.message))
                .unwrap_or_else(|| format!("APXM returned {status}: {resp_text}"));
            return Err(Error::adapter(msg));
        }

        let result: SkillExecuteResponse =
            serde_json::from_str(&resp_text).map_err(|e| {
                Error::adapter(format!("failed to parse APXM response: {e}"))
            })?;

        tracing::info!(
            execution_id = %result.execution_id,
            executed_nodes = result.stats.executed_nodes,
            "APXM skill execution completed"
        );

        Ok(result.execution_id)
    }

    async fn do_get_run(&self, run_id: &str) -> Result<tesela_ir::AgentRun, Error> {
        let url = format!("{}/v1/executions/{}", self.base_url, run_id);

        tracing::debug!(run_id = %run_id, "fetching APXM execution record");

        let resp = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(Self::map_reqwest_error)?;

        let status = resp.status();

        if status.as_u16() == 404 {
            return Err(Error::not_found("agent_run", run_id));
        }

        let resp_text = resp.text().await.map_err(Self::map_reqwest_error)?;

        if !status.is_success() {
            let apxm_err: Option<ApxmErrorResponse> = serde_json::from_str(&resp_text).ok();
            let msg = apxm_err
                .and_then(|e| e.error.map(|e| e.message))
                .unwrap_or_else(|| format!("APXM returned {status}"));
            return Err(Error::adapter(msg));
        }

        let record: ExecutionRecord =
            serde_json::from_str(&resp_text).map_err(|e| {
                Error::adapter(format!("failed to parse execution record: {e}"))
            })?;

        Ok(map_execution_to_agent_run(record))
    }
}

impl AgentRuntime for ApxmAgentRuntime {
    fn start_run(
        &self,
        agent: &tesela_ir::Agent,
        input: Value,
        actor: &Actor,
    ) -> Result<String, Error> {
        self.block_on(self.do_start_run(agent, input, actor))
    }

    fn get_run(&self, run_id: &str) -> Result<tesela_ir::AgentRun, Error> {
        self.block_on(self.do_get_run(run_id))
    }
}

// ---------------------------------------------------------------------------
// APXM HTTP API types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SkillExecuteRequest {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tesela_context: Option<TeselaContext>,
}

#[derive(Serialize)]
struct TeselaContext {
    agent_name: String,
    actor_id: String,
    input: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
}

#[derive(Deserialize)]
struct SkillExecuteResponse {
    execution_id: String,
    content: Option<String>,
    #[serde(default)]
    stats: ExecutionStats,
}

#[derive(Deserialize, Default)]
struct ExecutionStats {
    #[serde(default)]
    executed_nodes: usize,
}

#[derive(Deserialize)]
struct ExecutionRecord {
    execution_id: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    result: Option<SkillExecuteResponse>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    node_outputs: Vec<NodeOutputRecord>,
    #[serde(default)]
    node_metrics: Vec<NodeMetricsRecord>,
}

#[derive(Deserialize)]
struct NodeOutputRecord {
    #[serde(default)]
    node_name: Option<String>,
    #[serde(default)]
    output: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct NodeMetricsRecord {
    #[serde(default)]
    input_tokens: Option<usize>,
    #[serde(default)]
    output_tokens: Option<usize>,
}

#[derive(Deserialize)]
struct ApxmErrorResponse {
    error: Option<ApxmErrorDetail>,
}

#[derive(Deserialize)]
struct ApxmErrorDetail {
    #[serde(default)]
    message: String,
}

// ---------------------------------------------------------------------------
// Mapping APXM → Tesela
// ---------------------------------------------------------------------------

fn map_execution_to_agent_run(record: ExecutionRecord) -> tesela_ir::AgentRun {
    let status = match record.status.as_str() {
        "succeeded" => "completed",
        "failed" => "failed",
        "running" => "running",
        other => other,
    };

    let output = record
        .result
        .as_ref()
        .and_then(|r| r.content.clone());

    let total_tokens: usize = record
        .node_metrics
        .iter()
        .map(|m| m.input_tokens.unwrap_or(0) + m.output_tokens.unwrap_or(0))
        .sum();

    let messages = record
        .node_outputs
        .iter()
        .map(|n| {
            let mut msg = BTreeMap::new();
            msg.insert(
                "role".to_string(),
                Value::from("assistant"),
            );
            if let Some(name) = &n.node_name {
                msg.insert("node".to_string(), Value::from(name.as_str()));
            }
            if let Some(out) = &n.output {
                msg.insert(
                    "content".to_string(),
                    Value::from(out.to_string()),
                );
            }
            msg
        })
        .collect();

    let tool_calls_count: i32 = record
        .result
        .as_ref()
        .map(|r| r.stats.executed_nodes as i32)
        .unwrap_or(0);

    tesela_ir::AgentRun {
        id: record.execution_id,
        status: status.to_string(),
        output,
        error: record.error,
        messages,
        tool_calls: Some(tool_calls_count),
        tokens_used: if total_tokens > 0 {
            Some(total_tokens as i32)
        } else {
            None
        },
        cost_usd: None,
        eval_passed: None,
        eval_score: None,
        eval_notes: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_succeeded_execution() {
        let record = ExecutionRecord {
            execution_id: "exec-001".to_string(),
            status: "succeeded".to_string(),
            result: Some(SkillExecuteResponse {
                execution_id: "exec-001".to_string(),
                content: Some("Hello from APXM".to_string()),
                stats: ExecutionStats {
                    executed_nodes: 3,
                },
            }),
            error: None,
            node_outputs: vec![
                NodeOutputRecord {
                    node_name: Some("ask_0".to_string()),
                    output: Some(serde_json::json!("Hello from APXM")),
                },
            ],
            node_metrics: vec![
                NodeMetricsRecord {
                    input_tokens: Some(100),
                    output_tokens: Some(50),
                },
            ],
        };

        let run = map_execution_to_agent_run(record);
        assert_eq!(run.id, "exec-001");
        assert_eq!(run.status, "completed");
        assert_eq!(run.output, Some("Hello from APXM".to_string()));
        assert_eq!(run.error, None);
        assert_eq!(run.tool_calls, Some(3));
        assert_eq!(run.tokens_used, Some(150));
        assert_eq!(run.messages.len(), 1);
    }

    #[test]
    fn test_map_failed_execution() {
        let record = ExecutionRecord {
            execution_id: "exec-002".to_string(),
            status: "failed".to_string(),
            result: None,
            error: Some("model timeout".to_string()),
            node_outputs: vec![],
            node_metrics: vec![],
        };

        let run = map_execution_to_agent_run(record);
        assert_eq!(run.status, "failed");
        assert_eq!(run.error, Some("model timeout".to_string()));
        assert_eq!(run.output, None);
        assert_eq!(run.tokens_used, None);
    }

    #[test]
    fn test_map_running_execution() {
        let record = ExecutionRecord {
            execution_id: "exec-003".to_string(),
            status: "running".to_string(),
            result: None,
            error: None,
            node_outputs: vec![],
            node_metrics: vec![],
        };

        let run = map_execution_to_agent_run(record);
        assert_eq!(run.status, "running");
        assert_eq!(run.output, None);
    }

    fn test_agent(metadata: Option<BTreeMap<String, Value>>) -> tesela_ir::Agent {
        tesela_ir::Agent {
            api_name: "my_agent".parse().unwrap(),
            display: None,
            description: None,
            model: None,
            model_provider: None,
            instructions: None,
            allowed_tools: Vec::new(),
            custom_tools: Vec::new(),
            context_sources: Vec::new(),
            memory: None,
            limits: None,
            requires_approval: None,
            output_schema: None,
            output_object_type: None,
            capabilities: Vec::new(),
            deprecated_at: None,
            metadata,
        }
    }

    #[test]
    fn test_skill_id_from_metadata() {
        let mut m = BTreeMap::new();
        m.insert("apxm_skill_id".to_string(), Value::from("custom-skill"));
        let agent = test_agent(Some(m));
        assert_eq!(ApxmAgentRuntime::skill_id(&agent), "custom-skill");
    }

    #[test]
    fn test_skill_id_fallback_to_api_name() {
        let agent = test_agent(None);
        assert_eq!(ApxmAgentRuntime::skill_id(&agent), "my_agent");
    }
}
