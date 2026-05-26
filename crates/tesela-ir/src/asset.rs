//! Asset, column mapping, lineage, and environment types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};

use crate::Property;

/// A dataset asset produced by the upload pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Sink destination.
    pub sink: AssetSink,
    /// Properties (schema of the asset).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub properties: Vec<Property>,
    /// Tags.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    /// Deprecation timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
    /// Lineage edges.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub lineage: Vec<LineageEdge>,
}

/// Asset sink configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetSink {
    /// Target datasource.
    pub datasource: ApiName,
    /// Target resource (table, bucket, etc.).
    pub resource: String,
}

/// Column mapping for upload/ingestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnMapping {
    /// Source column name.
    pub source_column: String,
    /// Target property API name.
    pub target_property: ApiName,
    /// Whether this mapping is required.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub required: Option<bool>,
    /// Type coercion hint.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub type_coercion: Option<String>,
    /// Value mappings (source value -> target value).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value_mapping: Option<BTreeMap<String, String>>,
}

/// A lineage edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Source entity.
    pub source: ApiName,
    /// Kind of relationship (produces, consumes, derives_from).
    pub kind: String,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// An environment configuration (dev, staging, prod).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Configuration values.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<BTreeMap<String, Value>>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}
