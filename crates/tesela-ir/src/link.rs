//! Link type definitions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, LinkCardinality, Value};

/// A named, directional relationship between two object types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkType {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Source object type.
    pub from: ApiName,
    /// Target object type.
    pub to: ApiName,
    /// Cardinality.
    pub cardinality: LinkCardinality,
    /// Physical source mapping.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<LinkSource>,
    /// Property join mappings.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mappings: Vec<LinkMapping>,
    /// Junction table configuration (for many-to-many).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub junction: Option<JunctionConfig>,
    /// Deprecation timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// Physical source for a link type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkSource {
    /// Datasource API name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub datasource: Option<ApiName>,
    /// Resource name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resource: Option<String>,
}

/// Property mapping for a link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkMapping {
    /// Property on the source object type.
    pub from_property: ApiName,
    /// Property on the target object type.
    pub to_property: ApiName,
}

/// Junction table configuration for many-to-many links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JunctionConfig {
    /// Datasource API name.
    pub datasource: ApiName,
    /// Resource name.
    pub resource: String,
    /// Column referencing the source side.
    pub from_column: String,
    /// Column referencing the target side.
    pub to_column: String,
    /// Additional columns to expose.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub properties: Vec<String>,
}
