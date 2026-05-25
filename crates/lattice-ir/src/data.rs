//! Runtime data types (records, pages, results).

use lattice_core::{ApiName, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A single record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// Primary key value.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary_key: Option<Value>,
    /// Field values keyed by property API name.
    #[serde(default)]
    pub values: BTreeMap<ApiName, Value>,
}

/// A page of records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    /// Records in this page.
    #[serde(default)]
    pub records: Vec<Record>,
    /// Next page cursor (empty if no more pages).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub next_cursor: Option<String>,
}

/// Result of a mutation operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationResult {
    /// The mutated record (for create/update/upsert).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub record: Option<Record>,
    /// Number of rows affected.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rows_affected: Option<i64>,
}

/// Result of an action execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionResult {
    /// Status (success, failed, rejected, queued).
    pub status: String,
    /// Output payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<Value>,
    /// Error message.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    /// Run ID.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub run_id: Option<String>,
}

/// Result of an aggregation query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateResult {
    /// Aggregated groups.
    #[serde(default)]
    pub groups: Vec<BTreeMap<String, Value>>,
}

/// Result of an upload operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadResult {
    /// Upload run ID.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub run_id: Option<String>,
    /// Load ID.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub load_id: Option<String>,
    /// Number of rows loaded.
    #[serde(default)]
    pub rows_loaded: i64,
    /// Number of rows skipped.
    #[serde(default)]
    pub rows_skipped: i64,
    /// Skipped rows with reasons.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub skipped_rows: Vec<Value>,
    /// Quality check results.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub quality: Vec<Value>,
}

/// Result of an agent run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRun {
    /// Run ID.
    pub id: String,
    /// Status (running, completed, failed, timed_out).
    pub status: String,
    /// Final output.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<String>,
    /// Error message.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    /// Messages exchanged.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub messages: Vec<BTreeMap<String, Value>>,
    /// Number of tool calls made.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_calls: Option<i32>,
    /// Tokens used.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tokens_used: Option<i32>,
    /// Cost in USD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cost_usd: Option<f64>,
    /// Evaluation result attached when an evaluator is configured.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub eval_passed: Option<bool>,
    /// Evaluation score in `[0.0, 1.0]`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub eval_score: Option<f64>,
    /// Human-readable evaluation notes.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub eval_notes: Option<String>,
}

/// An explain plan for a query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplainPlan {
    /// Execution steps.
    #[serde(default)]
    pub steps: Vec<BTreeMap<String, Value>>,
}

/// Health status response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall status (healthy, degraded, unhealthy).
    pub status: String,
    /// Spec version.
    pub spec_version: String,
    /// Workspace name.
    pub workspace: String,
    /// Number of registered datasources.
    pub datasource_count: usize,
    /// Number of active policy rules.
    pub policy_count: usize,
    /// Number of defined roles.
    pub role_count: usize,
}

/// Capability advertisement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Capabilities as a flat map.
    #[serde(flatten)]
    pub values: BTreeMap<String, Value>,
}
