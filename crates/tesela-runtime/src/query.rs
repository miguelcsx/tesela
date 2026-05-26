//! Query and data types for the Tesela runtime.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};

/// A search query.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Query {
    /// Filter tree.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filter: Option<tesela_ir::Filter>,
    /// Sort directives.
    #[serde(default)]
    pub sort: Vec<Sort>,
    /// Pagination limit.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limit: Option<i32>,
    /// Pagination offset.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub offset: Option<i32>,
    /// Cursor-based pagination.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cursor: Option<String>,
}

impl Query {
    /// Merge an additional filter with AND semantics.
    ///
    /// If no existing filter is set, the new filter becomes the sole filter.
    /// If one already exists, a new `And(existing, additional)` node is created.
    #[must_use]
    pub fn and_filter(mut self, additional: tesela_ir::Filter) -> Self {
        self.filter = Some(match self.filter.take() {
            Some(existing) => tesela_ir::Filter::and(vec![existing, additional]),
            None => additional,
        });
        self
    }
}

/// A sort directive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sort {
    /// Property to sort by.
    pub property: ApiName,
    /// Direction: `asc` or `desc`.
    pub direction: String,
}

/// A mutation payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mutation {
    /// Create a single record.
    Create {
        /// Record values.
        values: BTreeMap<ApiName, Value>,
    },
    /// Update a record by primary key.
    Update {
        /// Primary key.
        primary_key: Value,
        /// New values.
        values: BTreeMap<ApiName, Value>,
    },
    /// Delete a record by primary key.
    Delete {
        /// Primary key.
        primary_key: Value,
    },
    /// Upsert (insert or replace).
    Upsert {
        /// Record values.
        values: BTreeMap<ApiName, Value>,
    },
    /// Batch of mutations.
    Batch {
        /// Individual mutations.
        items: Vec<Mutation>,
    },
}

/// An aggregate query.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AggregateQuery {
    /// Filter before aggregation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filter: Option<tesela_ir::Filter>,
    /// Group-by properties.
    #[serde(default)]
    pub group_by: Vec<ApiName>,
    /// Aggregations to compute.
    #[serde(default)]
    pub aggregations: Vec<Aggregation>,
    /// Optional time bucket for temporal rollups.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub time_bucket: Option<tesela_ir::TimeBucket>,
    /// Optional spatial extent descriptor.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub spatial_extent: Option<tesela_ir::SpatialExtent>,
    /// Require backend/adapter pushdown; do not emulate in the runtime.
    #[serde(default)]
    pub require_pushdown: bool,
}

/// A single aggregation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aggregation {
    /// Function: count, sum, avg, min, max.
    pub function: String,
    /// Property to aggregate (optional for count).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub property: Option<ApiName>,
    /// Alias for the result.
    pub alias: String,
}

/// A traversal query over a link.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TraversalQuery {
    /// Starting primary key in the source object.
    pub source_pk: Value,
    /// Optional filter on target side.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filter: Option<tesela_ir::Filter>,
    /// Sort directives.
    #[serde(default)]
    pub sort: Vec<Sort>,
    /// Pagination limit.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limit: Option<i32>,
    /// Pagination offset.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub offset: Option<i32>,
}

/// Request to execute an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRequest {
    /// Action API name.
    pub action: ApiName,
    /// Input payload.
    #[serde(default)]
    pub input: Value,
    /// Actor initiating the action.
    pub actor: Actor,
    /// Optional run ID for idempotency.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub run_id: Option<String>,
}

/// An actor (authenticated user / service).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    /// User or service identifier.
    pub user_id: String,
    /// Assigned roles.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Arbitrary claims.
    #[serde(default)]
    pub claims: BTreeMap<String, Value>,
}

/// A constrained capability attached to a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Token identifier.
    pub id: String,
    /// Grant API name.
    pub grant: ApiName,
    /// Subject the grant was issued to.
    pub subject: String,
    /// Allowed operations.
    #[serde(default)]
    pub operations: Vec<tesela_core::Operation>,
    /// Resource kind.
    pub resource_kind: String,
    /// Optional resource name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resource: Option<ApiName>,
    /// Expiration timestamp.
    pub expires_at: String,
    /// Adapter- or policy-owned constraints.
    #[serde(default)]
    pub constraints: BTreeMap<String, Value>,
}

/// Concrete resource context supplied to policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResourceContext {
    /// Object primary key, artifact key, run ID, or other resource instance ID.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub id: Option<Value>,
    /// Resource instance field values known before adapter access.
    #[serde(default)]
    pub values: BTreeMap<String, Value>,
    /// Related resource IDs or attributes.
    #[serde(default)]
    pub relationships: BTreeMap<String, Value>,
}

/// Metadata extracted from an incoming request.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RequestMeta {
    /// Authorization header (or other bearer).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authorization: Option<String>,
    /// Request headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Client IP.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub remote_addr: Option<String>,
    /// Workspace / tenant override from the inbound boundary.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub workspace: Option<String>,
    /// Correlation ID for audit, events, and job linkage.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub correlation_id: Option<String>,
}

/// A policy evaluation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRequest {
    /// Actor.
    pub actor: Actor,
    /// Operation being performed.
    pub operation: tesela_core::Operation,
    /// Resource kind (object_type, action, agent, etc.).
    pub resource_kind: String,
    /// Resource API name.
    pub resource: ApiName,
    /// Extra context.
    #[serde(default)]
    pub context: BTreeMap<String, Value>,
    /// Concrete resource instance context.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resource_instance: Option<ResourceContext>,
    /// Request metadata from the inbound boundary.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub request_meta: Option<RequestMeta>,
    /// Capability token being exercised.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub capability: Option<CapabilityToken>,
    /// Operation parameters relevant to policy.
    #[serde(default)]
    pub operation_params: BTreeMap<String, Value>,
}

/// Result of a policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// Whether the operation is allowed.
    pub allow: bool,
    /// Human-readable reason.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
    /// Row-level filter to apply.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub row_filter: Option<tesela_ir::Filter>,
    /// Properties to redact from results.
    #[serde(default)]
    pub redactions: Vec<ApiName>,
    /// Obligations to execute.
    #[serde(default)]
    pub obligations: Vec<tesela_ir::Obligation>,
}

/// A single audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Record ID.
    pub id: String,
    /// Timestamp.
    pub occurred_at: String,
    /// Actor user ID.
    pub actor_user_id: String,
    /// Operation.
    pub operation: String,
    /// Resource kind.
    pub resource_kind: String,
    /// Resource name.
    pub resource: String,
    /// Policy decision.
    pub decision: String,
    /// Result count.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub result_count: Option<i64>,
    /// Error code if failed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_code: Option<String>,
    /// Extra metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

/// An event published to the event bus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Event ID.
    pub id: String,
    /// Event kind.
    pub kind: String,
    /// Workspace name.
    pub workspace: String,
    /// Object type (if applicable).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub object_type: Option<String>,
    /// Actor user ID.
    pub actor_user_id: String,
    /// Timestamp.
    pub occurred_at: String,
    /// Payload.
    #[serde(default)]
    pub payload: BTreeMap<String, Value>,
    /// Logical event type from the IR, if known.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub event_type: Option<ApiName>,
    /// Topic or stream name selected by the adapter.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub topic: Option<String>,
    /// Correlation ID linking jobs, actions, uploads, and events.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub correlation_id: Option<String>,
    /// Causation ID for event chains.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub causation_id: Option<String>,
}

/// Backend capability advertisement.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BackendCapabilities {
    /// Supports search.
    pub search: bool,
    /// Supports get by PK.
    pub get: bool,
    /// Supports mutation.
    pub mutate: bool,
    /// Supports aggregation.
    pub aggregate: bool,
    /// Supports traversal.
    pub traverse: bool,
    /// Supports bulk load.
    pub bulk_load: bool,
    /// Supports rollback.
    pub rollback: bool,
    /// Supports explain.
    pub explain: bool,
}

/// A work item for the queue / scheduler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItem {
    /// Work item kind.
    pub kind: String,
    /// Optional declared job type.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub job_type: Option<ApiName>,
    /// Run identifier.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub run_id: Option<String>,
    /// Idempotency key.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub idempotency_key: Option<String>,
    /// Correlation ID.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub correlation_id: Option<String>,
    /// Payload.
    #[serde(default)]
    pub payload: BTreeMap<String, Value>,
}

/// Request for human approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Action or agent being approved.
    pub resource: String,
    /// Actor requesting.
    pub actor: Actor,
    /// Reason / context.
    pub reason: String,
}

/// Decision from an approval provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    /// Approved or denied.
    pub approved: bool,
    /// Approver identity.
    pub approver: String,
    /// Optional reason.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
}

/// Signed upload URL response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedUpload {
    /// URL to upload to.
    pub url: String,
    /// Logical path/key in the object store.
    pub path: String,
    /// Expiration timestamp.
    pub expires_at: String,
    /// Headers the client must include.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Opaque upload session ID.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub upload_id: Option<String>,
}

/// Signed artifact read response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactLocator {
    /// URL or adapter-owned locator.
    pub url: String,
    /// Logical path/key in the object store.
    pub path: String,
    /// Expiration timestamp.
    pub expires_at: String,
    /// Media type, if known.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub media_type: Option<String>,
    /// Headers the client must include.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Metadata about the artifact.
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

/// Object-store metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ObjectMetadata {
    /// Logical path/key.
    pub path: String,
    /// Size in bytes.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub size_bytes: Option<i64>,
    /// Media type.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub media_type: Option<String>,
    /// ETag or content hash.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub etag: Option<String>,
    /// Last modified timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_modified: Option<String>,
    /// Adapter-owned metadata.
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

/// A job/action/upload run record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRecord {
    /// Run ID.
    pub id: String,
    /// Resource kind: action, job, upload.
    pub kind: String,
    /// Resource API name.
    pub resource: ApiName,
    /// Current status.
    pub status: String,
    /// Actor user ID.
    pub actor_user_id: String,
    /// Idempotency key.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub idempotency_key: Option<String>,
    /// Correlation ID.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub correlation_id: Option<String>,
    /// Input payload.
    #[serde(default)]
    pub input: BTreeMap<String, Value>,
    /// Output payload.
    #[serde(default)]
    pub output: BTreeMap<String, Value>,
    /// Adapter-owned step results.
    #[serde(default)]
    pub steps: Vec<BTreeMap<String, Value>>,
    /// Created timestamp.
    pub created_at: String,
    /// Updated timestamp.
    pub updated_at: String,
}

/// Instructs the model how to select a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    /// Model decides whether to call a tool.
    Auto,
    /// Model must not call any tool.
    None,
    /// Model must call at least one tool.
    Required,
    /// Model must call exactly this named tool.
    Specific(String),
}

/// Instructs the model how to format its response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Plain text (default).
    Text,
    /// Any valid JSON object.
    JsonObject,
    /// JSON that matches the given JSON Schema.
    JsonSchema {
        /// JSON Schema the output must conform to.
        schema: Value,
    },
}

/// Model provider request.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModelRequest {
    /// System prompt.
    #[serde(default)]
    pub system: String,
    /// Conversation messages.
    #[serde(default)]
    pub messages: Vec<BTreeMap<String, Value>>,
    /// Tools available.
    #[serde(default)]
    pub tools: Vec<ToolDef>,
    /// Maximum tokens.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_tokens: Option<i32>,
    /// Temperature.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub temperature: Option<f64>,
    /// How the model selects tools.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_choice: Option<ToolChoice>,
    /// Response format constraint.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub response_format: Option<ResponseFormat>,
    /// Allow the model to call multiple tools in a single turn.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parallel_tool_calls: Option<bool>,
}

/// Model provider response.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ModelResponse {
    /// Response content.
    pub content: String,
    /// Tool calls requested by the model.
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    /// Tokens consumed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tokens_used: Option<i32>,
    /// Structured JSON output when `response_format` was `JsonSchema` or `JsonObject`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub structured_output: Option<Value>,
}

/// Definition of a tool available to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDef {
    /// Tool name.
    pub name: String,
    /// Description.
    pub description: String,
    /// JSON schema for parameters.
    pub parameters: Value,
}

/// A tool call from the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name.
    pub name: String,
    /// Call ID.
    pub id: String,
    /// Arguments JSON.
    pub arguments: String,
}

/// A message in an agent conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Role: system, user, assistant, tool.
    pub role: String,
    /// Content.
    pub content: String,
    /// Optional tool calls.
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    /// Optional tool call ID this message responds to.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_call_id: Option<String>,
}

/// A plan from the planner.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Plan {
    /// Planned steps.
    #[serde(default)]
    pub steps: Vec<String>,
}

/// Interceptor operation kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterceptorOp {
    /// Before operation.
    Before,
    /// After operation.
    After,
}
