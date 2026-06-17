//! Layer contracts for reusable data and execution boundaries.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};

/// A reusable layer definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerDefinition {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Layer kind interpreted by the host runtime or adapter.
    pub kind: String,
    /// Input JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_schema: Option<Value>,
    /// Output JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_schema: Option<Value>,
    /// Layer dependencies by API name.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub dependencies: Vec<ApiName>,
    /// Adapter or runtime configuration.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<BTreeMap<String, Value>>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A concrete instance of a layer definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerInstance {
    /// Instance identifier.
    pub id: String,
    /// Layer definition API name.
    pub layer: ApiName,
    /// Instance state.
    pub state: String,
    /// Input values bound to this instance.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub inputs: BTreeMap<String, Value>,
    /// Output values produced by this instance.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub outputs: BTreeMap<String, Value>,
    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created_at: Option<String>,
    /// Last update timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub updated_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A run of a layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerRun {
    /// Run identifier.
    pub id: String,
    /// Layer definition API name.
    pub layer: ApiName,
    /// Layer instance identifier.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub instance_id: Option<String>,
    /// Run status.
    pub status: String,
    /// Input payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input: Option<Value>,
    /// Output payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<Value>,
    /// Artifacts produced by the run.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub artifacts: Vec<LayerArtifact>,
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

/// An artifact emitted by a layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerArtifact {
    /// Artifact identifier.
    pub id: String,
    /// Artifact kind.
    pub kind: String,
    /// Logical or physical artifact URI.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub uri: Option<String>,
    /// Optional inline artifact value.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<Value>,
    /// Media type.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub media_type: Option<String>,
    /// Producing run identifier.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub produced_by: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}
