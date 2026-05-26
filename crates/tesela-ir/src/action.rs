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
