//! Function contracts for adapter or runtime executed units.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};

/// A reusable function definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Runtime or adapter kind.
    pub runtime: String,
    /// Handler identifier within the runtime.
    pub handler: String,
    /// Input JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_schema: Option<Value>,
    /// Output JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_schema: Option<Value>,
    /// Whether the function is expected to be deterministic.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deterministic: Option<bool>,
    /// Timeout in seconds.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timeout_seconds: Option<u64>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A concrete function execution attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionRun {
    /// Run identifier.
    pub id: String,
    /// Function definition API name.
    pub function: ApiName,
    /// Execution status.
    pub status: String,
    /// Input payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input: Option<Value>,
    /// Output payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<Value>,
    /// Error message.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    /// Start timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub started_at: Option<String>,
    /// Completion timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub completed_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}
