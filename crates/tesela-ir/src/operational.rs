//! Operational primitives for adapter-owned runtime workflows.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};

use crate::{ColumnMapping, Property, QualityRule};

/// A byte-oriented artifact addressable through an object-store adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactType {
    /// API name.
    pub api_name: ApiName,
    /// Human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Object store adapter API name.
    pub store: ApiName,
    /// Template used by adapters to resolve logical artifact paths.
    pub path_template: String,
    /// Optional media type, e.g. `application/vnd.apache.arrow.file`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub media_type: Option<String>,
    /// Schema for artifact metadata.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub metadata_schema: Vec<Property>,
    /// Lifecycle state names.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub lifecycle: Vec<String>,
    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// Declarative upload and ingestion flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadFlow {
    /// API name.
    pub api_name: ApiName,
    /// Object store adapter used for incoming objects.
    pub store: ApiName,
    /// Accepted file or payload formats.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub accepted_formats: Vec<String>,
    /// Maximum accepted object size in bytes.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_bytes: Option<i64>,
    /// Upload path template.
    pub path_template: String,
    /// Optional target object type for loaded records.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub target_object_type: Option<ApiName>,
    /// Column mappings for record-oriented uploads.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mappings: Vec<ColumnMapping>,
    /// Quality rules applied before load.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub quality_rules: Vec<QualityRule>,
    /// Whether schema discovery is allowed for this flow.
    #[serde(default)]
    pub discover_schema: bool,
    /// Whether rollback is required for committed loads.
    #[serde(default)]
    pub rollback_required: bool,
    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A long-running adapter-owned process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobType {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Queue or executor adapter name.
    pub executor: ApiName,
    /// Input JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_schema: Option<Value>,
    /// Output JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_schema: Option<Value>,
    /// Lifecycle state names.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub states: Vec<String>,
    /// Idempotency key template.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub idempotency_key: Option<String>,
    /// Event emitted when the job starts.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start_event: Option<ApiName>,
    /// Event expected when the job completes or fails.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub result_event: Option<ApiName>,
    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A logical event routed through a message bus adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventType {
    /// API name.
    pub api_name: ApiName,
    /// Message bus adapter name.
    pub bus: ApiName,
    /// Logical topic or stream.
    pub topic: String,
    /// Payload JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub payload_schema: Option<Value>,
    /// Field names used for correlation.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub correlation_keys: Vec<String>,
    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// Declarative capability grant for constrained access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGrant {
    /// API name.
    pub api_name: ApiName,
    /// Resource kind this capability can access.
    pub resource_kind: String,
    /// Optional resource API name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resource: Option<ApiName>,
    /// Operations allowed by this capability.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub operations: Vec<tesela_core::Operation>,
    /// Default time-to-live in seconds.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ttl_seconds: Option<u64>,
    /// Extra constraints interpreted by the capability adapter or policy engine.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub constraints: Option<BTreeMap<String, Value>>,
    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// Named aggregate view backed by an object type and adapter pushdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateView {
    /// API name.
    pub api_name: ApiName,
    /// Source object type.
    pub object_type: ApiName,
    /// Optional base filter.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filter: Option<crate::Filter>,
    /// Group-by properties.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub group_by: Vec<ApiName>,
    /// Aggregate measures.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub measures: Vec<AggregateMeasure>,
    /// Optional time bucket.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub time_bucket: Option<TimeBucket>,
    /// Optional spatial extent descriptor.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub spatial_extent: Option<SpatialExtent>,
    /// Require the adapter to execute this aggregate rather than fallback.
    #[serde(default)]
    pub require_pushdown: bool,
    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A typed aggregate measure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateMeasure {
    /// Function name.
    pub function: AggregateFunction,
    /// Optional property.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub property: Option<ApiName>,
    /// Result alias.
    pub alias: String,
    /// Whether to aggregate distinct values.
    #[serde(default)]
    pub distinct: bool,
}

/// Supported aggregate functions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateFunction {
    /// Count rows or values.
    Count,
    /// Sum numeric values.
    Sum,
    /// Average numeric values.
    Avg,
    /// Minimum value.
    Min,
    /// Maximum value.
    Max,
    /// Count distinct values.
    CountDistinct,
    /// Spatial bounding box or extent.
    SpatialExtent,
}

/// Time-bucketing descriptor for aggregate views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeBucket {
    /// Timestamp property.
    pub property: ApiName,
    /// Bucket size, e.g. `1h`, `15m`, `1d`.
    pub interval: String,
    /// Optional timezone.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timezone: Option<String>,
}

/// Spatial extent descriptor for aggregate views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpatialExtent {
    /// Geometry property.
    pub property: ApiName,
    /// Output shape, e.g. `bbox`, `centroid`, `polygon`.
    pub output: String,
}
