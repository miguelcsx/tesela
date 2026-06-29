//! Object type, trait, and property metadata.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, DataType, Value};

/// A reusable trait for object types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trait {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Properties defined by the trait.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub properties: Vec<Property>,
}

/// A domain entity type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectType {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Physical source mapping.
    pub source: ObjectSource,
    /// Primary key property.
    pub primary_key: ApiName,
    /// Properties.
    pub properties: Vec<Property>,
    /// Traits this object type implements.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub traits: Vec<ApiName>,
    /// Tags for categorization.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
    /// Indexes.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub indexes: Vec<Index>,
    /// Deprecation timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_at: Option<String>,
}

/// Physical source mapping for an object type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSource {
    /// Datasource API name.
    pub datasource: ApiName,
    /// Optional resource name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resource: Option<String>,
}

/// A property on an object type or trait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Data type.
    pub data_type: DataType,
    /// Whether NULL is allowed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nullable: Option<bool>,
    /// Whether indexed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub indexed: Option<bool>,
    /// Whether unique.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unique: Option<bool>,
    /// Tags.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    /// Markings.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub markings: Vec<String>,
    /// Default value.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub default: Option<Value>,
    /// Source column mapping.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_column: Option<String>,
    /// Allowed enum values.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub allowed_values: Option<Vec<Value>>,
    /// Sort order hint.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sort_order: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
    /// Whether this field is encrypted by the platform store.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub encrypted: Option<bool>,
}

/// Index definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    /// API name.
    pub api_name: ApiName,
    /// Properties included in the index.
    pub properties: Vec<ApiName>,
    /// Whether unique.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unique: Option<bool>,
}
