//! Transform pipeline types.

use tesela_core::{ApiName, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A DAG of transform steps that produce object types from other object types.
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
}
