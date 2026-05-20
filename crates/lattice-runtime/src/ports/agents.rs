//! Agent runtime, model provider, memory, and evaluation port traits.

use crate::query::*;
use lattice_core::{Error, Value};
use std::collections::BTreeMap;

/// Runtime for executing agent runs.
pub trait AgentRuntime: Send + Sync {
    /// Start a new agent run. Returns the run ID.
    fn start_run(
        &self,
        agent: &lattice_ir::Agent,
        input: Value,
        actor: &Actor,
    ) -> Result<String, Error>;
    /// Get the current state of a run.
    fn get_run(&self, run_id: &str) -> Result<lattice_ir::AgentRun, Error>;
}

/// Abstraction over an LLM / model provider.
pub trait ModelProvider: Send + Sync {
    /// Call the model.
    fn call(&self, request: ModelRequest) -> Result<ModelResponse, Error>;
}

/// Plans agent steps ahead of execution.
pub trait Planner: Send + Sync {
    /// Produce a plan given context and available tools.
    fn plan(&self, context: &str, tools: &[ToolDef]) -> Result<Option<Plan>, Error>;
}

/// Compacts a message history when context exceeds threshold.
pub trait Compactor: Send + Sync {
    /// Compact messages.
    fn compact(&self, messages: &[Message]) -> Result<Vec<Message>, Error>;
}

/// Agent memory store.
pub trait AgentMemoryStore: Send + Sync {
    /// Store a value.
    fn remember(&self, namespace: &str, key: &str, value: &str) -> Result<(), Error>;
    /// Retrieve a value.
    fn recall(&self, namespace: &str, key: &str) -> Result<Option<String>, Error>;
    /// Search memory.
    fn search_memory(&self, namespace: &str, query: &str) -> Result<Vec<String>, Error>;
    /// Delete a value.
    fn forget(&self, namespace: &str, key: &str) -> Result<(), Error>;
}

/// Communication channel between agents.
pub trait AgentCommunicationChannel: Send + Sync {
    /// Send a message.
    fn send(&self, channel: &str, message: &str) -> Result<(), Error>;
    /// Receive a message with timeout.
    fn receive(&self, channel: &str, timeout_ms: u64) -> Result<Option<String>, Error>;
}

/// Subagent runtime for spawning child agents.
pub trait SubagentRuntime: Send + Sync {
    /// Spawn a subagent run.
    fn spawn(
        &self,
        agent: &lattice_ir::Agent,
        input: Value,
        parent_run_id: &str,
    ) -> Result<String, Error>;
    /// Wait for a subagent run to complete.
    fn wait(&self, run_id: &str, timeout_ms: u64) -> Result<lattice_ir::AgentRun, Error>;
}

/// Evaluation context provided alongside an agent run.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EvalContext {
    /// Expected final output (for exact-match evaluation).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub expected_output: Option<Value>,
    /// Ordered list of tool names expected to be called.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub golden_tool_calls: Option<Vec<String>>,
    /// Arbitrary metadata passed through to the evaluator.
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

/// Result produced by an [`AgentEvaluator`] after a run completes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvalResult {
    /// Whether the run passed evaluation.
    pub passed: bool,
    /// Numeric score in `[0.0, 1.0]` (evaluator-defined).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub score: Option<f64>,
    /// Human-readable notes from the evaluator.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub notes: Option<String>,
}

/// Agnostic agent evaluation port.
pub trait AgentEvaluator: Send + Sync {
    /// Evaluate a completed agent run.
    fn evaluate(&self, run: &lattice_ir::AgentRun, ctx: &EvalContext) -> Result<EvalResult, Error>;
}

/// Simple evaluator that checks exact output match against the expected value.
pub struct GoldenSetEvaluator;

impl AgentEvaluator for GoldenSetEvaluator {
    fn evaluate(&self, run: &lattice_ir::AgentRun, ctx: &EvalContext) -> Result<EvalResult, Error> {
        let passed = ctx.expected_output.as_ref().is_none_or(|exp| {
            run.output
                .as_ref()
                .is_some_and(|o| o.as_str() == exp.as_str().unwrap_or(""))
        });
        Ok(EvalResult {
            passed,
            score: Some(if passed { 1.0 } else { 0.0 }),
            notes: if passed {
                None
            } else {
                Some("output did not match expected".to_string())
            },
        })
    }
}
