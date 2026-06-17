//! Action type definitions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};

/// A named action that can be executed against the ontology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionType {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Subject object type (optional).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub subject: Option<ApiName>,
    /// Handler configuration.
    pub handler: ActionHandler,
    /// Input JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_schema: Option<Value>,
    /// Output JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_schema: Option<Value>,
    /// Execution mode (sync, async).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mode: Option<String>,
    /// Risk level (high, medium, low).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub risk_level: Option<String>,
    /// Idempotency key template.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub idempotency_key: Option<String>,
    /// Deprecation timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// Handler configuration for an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionHandler {
    /// Handler kind (crud, webhook, callback, composite).
    pub kind: String,
    /// Target (URL, function name, etc.).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub target: Option<String>,
    /// Additional configuration.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<BTreeMap<String, Value>>,
}

/// A proposed action invocation awaiting validation, approval, or execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionProposal {
    /// Proposal identifier.
    pub id: String,
    /// Action API name.
    pub action: ApiName,
    /// Optional subject resource API name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub subject: Option<ApiName>,
    /// Input payload proposed for the action.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input: Option<Value>,
    /// Actor or system that proposed the action.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub proposed_by: Option<String>,
    /// Human-readable reason for the proposal.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
    /// Whether approval is required before execution.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub requires_approval: Option<bool>,
    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A concrete action execution attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRun {
    /// Run identifier.
    pub id: String,
    /// Action API name.
    pub action: ApiName,
    /// Proposal that created this run, when applicable.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub proposal_id: Option<String>,
    /// Execution status.
    pub status: String,
    /// Input payload used for execution.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input: Option<Value>,
    /// Output payload produced by execution.
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
