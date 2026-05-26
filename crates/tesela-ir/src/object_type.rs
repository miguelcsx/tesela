//! Object type, trait, property, and related configuration types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, DataType, Value};

use crate::LineageEdge;

/// A reusable trait (mixin) for object types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trait {
    /// API name of the trait.
    pub api_name: ApiName,
    /// Human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Properties defined by this trait.
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
    /// Primary key property name.
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
    /// Temporal (bi-temporal) configuration.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub temporal: Option<TemporalConfig>,
    /// Lifecycle configuration (soft delete, archival, retention).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub lifecycle: Option<LifecycleConfig>,
    /// Scoring configuration.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scoring: Option<ScoringConfig>,
    /// Classification (sensitivity, owner, domain).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub classification: Option<ClassificationConfig>,
    /// Quality rules.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub quality_rules: Vec<QualityRule>,
    /// Lineage edges.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub lineage: Vec<LineageEdge>,
    /// Deprecation timestamp (RFC3339).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deprecated_at: Option<String>,
}

/// Physical source mapping for an object type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSource {
    /// Datasource API name.
    pub datasource: ApiName,
    /// Optional resource name (table, collection, etc.).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resource: Option<String>,
}

/// A property on an object type or trait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    /// API name of the property.
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
    /// Tags (e.g., "pii", "sensitive").
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    /// Markings (e.g., classification labels).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub markings: Vec<String>,
    /// Default value.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub default: Option<Value>,
    /// Computed expression.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub computed: Option<Computed>,
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
    /// Whether this field is encrypted at rest.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub encrypted: Option<bool>,
    /// Quality rule references.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub quality: Vec<QualityRuleRef>,
}

/// Computed property expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Computed {
    /// Expression language (e.g., "sql", "cel", "python").
    pub language: String,
    /// Expression string.
    pub expression: String,
}

/// Reference to a quality rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityRuleRef {
    /// API name of the quality rule.
    pub api_name: ApiName,
    /// Kind of check.
    pub kind: String,
    /// Arguments.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub args: Option<BTreeMap<String, Value>>,
}

/// Quality rule definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityRule {
    /// API name.
    pub api_name: ApiName,
    /// Kind of check (not_null, unique, range, regex, etc.).
    pub kind: String,
    /// Target property.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub property: Option<ApiName>,
    /// Severity (error, warning).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub severity: Option<String>,
    /// Arguments.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub args: Option<BTreeMap<String, Value>>,
    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
}

/// Index definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    /// API name of the index.
    pub api_name: ApiName,
    /// Properties included in the index.
    pub properties: Vec<ApiName>,
    /// Whether unique.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unique: Option<bool>,
}

/// Temporal (bi-temporal) configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalConfig {
    /// Valid time start property.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub valid_time_start: Option<ApiName>,
    /// Valid time end property.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub valid_time_end: Option<ApiName>,
    /// System time start property.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system_time_start: Option<ApiName>,
    /// System time end property.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system_time_end: Option<ApiName>,
}

/// Lifecycle configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleConfig {
    /// Soft delete configuration.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub soft_delete: Option<SoftDeleteConfig>,
    /// Archival configuration.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub archival: Option<ArchivalConfig>,
    /// Retention configuration.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub retention: Option<RetentionConfig>,
}

/// Soft delete configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftDeleteConfig {
    /// Column name used for soft deletion flag.
    pub column: String,
    /// Whether to automatically filter soft-deleted rows on query.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filter_on_query: Option<bool>,
}

/// Archival configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivalConfig {
    /// Days after which to archive.
    pub after_days: i32,
    /// Optional archive table name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub archive_table: Option<String>,
}

/// Retention configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Maximum age in days.
    pub max_age_days: i32,
}

/// Scoring configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Scoring expression.
    pub expression: String,
    /// Properties this score depends on.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub depends_on: Vec<ApiName>,
}

/// Classification configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassificationConfig {
    /// Sensitivity level.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sensitivity: Option<String>,
    /// Data owner.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub owner: Option<String>,
    /// Data domain.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data_domain: Option<String>,
    /// Lineage references.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub lineage: Vec<String>,
    /// Retention days.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub retention_days: Option<i32>,
}
