//! Transform pipeline types with dynamic execution model.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};

/// A DAG of transform steps that produce object types from other object types.
///
/// The pipeline supports dynamic execution: steps can be conditionally skipped,
/// and `Decision` steps can inject, remove, or reroute other steps mid-execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransformPipeline {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Ordered list of transform steps.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub steps: Vec<TransformStep>,
    /// Cron-style execution schedule.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub schedule: Option<PipelineSchedule>,
    /// Execution mode.
    #[serde(default)]
    pub mode: ExecutionMode,
    /// Initial context variables for condition evaluation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub context: Option<BTreeMap<String, Value>>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A single step in a transform pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransformStep {
    /// API name (must be unique within the pipeline).
    pub api_name: ApiName,
    /// Source object type or other pipeline to read from.
    pub source: ApiName,
    /// Target object type to write into.
    pub target: ApiName,
    /// Expression that maps source records to target records.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub expression: Option<String>,
    /// Language for the expression ("cel", "sql", "identity").
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub language: Option<String>,
    /// Only process records modified after the last successful run.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub watermark_property: Option<ApiName>,
    /// Boolean expression evaluated at runtime; step is skipped when false.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub when: Option<String>,
    /// Error handling strategy for this step.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub on_error: Option<ErrorStrategy>,
    /// Runtime-resolved source api_name (overrides `source` when set).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub dynamic_source: Option<String>,
    /// Step kind — determines execution behavior.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kind: Option<StepKind>,
}

/// What kind of step this is in the pipeline DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// Read source, transform, write target.
    #[default]
    Transform,
    /// Evaluate expression and return a directive to mutate the live DAG.
    Decision,
    /// Split execution into parallel branches (sequential for now).
    Fork,
    /// Barrier for forked branches.
    Join,
}

/// Error handling strategy for a pipeline step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "strategy")]
pub enum ErrorStrategy {
    /// Log error and continue with the next step.
    Skip,
    /// Abort the entire pipeline immediately.
    Abort,
    /// Execute a named fallback step instead.
    Fallback {
        /// API name of the fallback step within this pipeline.
        step: ApiName,
    },
}

/// Directive returned by a Decision step to mutate the live DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StepDirective {
    /// Steps to inject into the remaining execution queue.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub inject: Vec<TransformStep>,
    /// Steps to remove from the remaining execution queue.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub remove: Vec<ApiName>,
    /// Reroute: change a pending step's source to a different api_name.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub reroute: Vec<RouteChange>,
}

/// A single source reroute within a [`StepDirective`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteChange {
    /// Step whose source should change.
    pub step: ApiName,
    /// New source api_name.
    pub new_source: ApiName,
}

/// Mutable context carried through pipeline execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PipelineContext {
    /// User-provided key/value pairs for condition evaluation.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub metadata: BTreeMap<String, Value>,
    /// Variables accumulated during execution (step outputs, decision results).
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub variables: BTreeMap<String, Value>,
}

/// How the pipeline refreshes the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Only process records changed since the last run.
    #[default]
    Incremental,
    /// Truncate and re-populate the target on every run.
    Snapshot,
}

/// When the pipeline should run automatically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineSchedule {
    /// Standard cron expression (e.g. `"0 * * * *"`).
    Cron(String),
    /// Only triggered explicitly via the API.
    Manual,
}

/// Per-step execution result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepResult {
    /// Step API name.
    pub step: ApiName,
    /// Execution status.
    pub status: StepStatus,
    /// Records written by this step.
    #[serde(default)]
    pub records_written: i64,
    /// Error message if the step failed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    /// Steps injected by this step (for Decision steps).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub injected_steps: Vec<ApiName>,
}

/// Status of a step after execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Ran successfully.
    Executed,
    /// Skipped due to `when` condition or error strategy.
    Skipped,
    /// Failed during execution.
    Failed,
    /// Was dynamically injected by a Decision step.
    Injected,
}

/// Result returned after executing a pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineResult {
    /// Records written across all steps.
    pub records_written: i64,
    /// Wall-clock execution time in milliseconds.
    pub duration_ms: u64,
    /// Per-step error messages (non-fatal if the step is optional).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<String>,
    /// Per-step execution results.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub step_results: Vec<StepResult>,
}
