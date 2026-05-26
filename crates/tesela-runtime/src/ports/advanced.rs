//! Advanced port traits (interceptors, computed, quality, vector, lineage, etc.).

use crate::query::*;
use tesela_core::{ApiName, Error, Value};
use tesela_ir::{
    Branch, BranchStatus, MigrationPlan, ObjectType, Page, PipelineResult, Record, Spec,
    TransformPipeline,
};

/// Interceptor hook around runtime operations.
pub trait Interceptor: Send + Sync {
    /// Intercept an operation.
    fn intercept(
        &self,
        op: InterceptorOp,
        operation: tesela_core::Operation,
        object_type: &ApiName,
        context: &mut std::collections::BTreeMap<String, Value>,
    ) -> Result<(), Error>;
}

/// Language an expression is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputedLang {
    /// Common Expression Language.
    Cel,
    /// SQL expression (evaluated by the backend).
    Sql,
    /// Python (evaluated by an embedded interpreter).
    Python,
}

impl ComputedLang {
    /// Parse from the string stored in the spec.
    pub fn from_spec_str(s: &str) -> Self {
        match s {
            "cel" => Self::Cel,
            "sql" => Self::Sql,
            "python" => Self::Python,
            _ => Self::Cel,
        }
    }
}

/// Context passed to a computed property evaluator.
pub struct RecordContext<'a> {
    /// The record being enriched.
    pub record: &'a Record,
    /// API name of the object type.
    pub object_type: &'a ApiName,
}

/// Evaluates computed property expressions against a record.
pub trait ComputedEvaluator: Send + Sync {
    /// Evaluate one expression and return the computed value.
    fn evaluate(
        &self,
        lang: ComputedLang,
        expr: &str,
        ctx: &RecordContext<'_>,
    ) -> Result<Value, Error>;
}

/// Validates a record against the quality rules defined on an object type.
pub trait QualityRuleEvaluator: Send + Sync {
    /// Check all quality rules for the given object type against `record`.
    fn validate(&self, object_type: &ObjectType, record: &Record) -> Result<(), Error>;
}

/// A nearest-neighbour vector search query.
#[derive(Debug, Clone)]
pub struct VectorSearchQuery {
    /// Object type to search within.
    pub object_type: ApiName,
    /// Query embedding vector.
    pub query_vector: Vec<f32>,
    /// Number of nearest neighbours to return.
    pub top_k: usize,
    /// HNSW `ef` search parameter (higher = more accurate, slower).
    pub ef: usize,
    /// Optional metadata pre-filter applied before ANN scoring.
    pub filter: Option<tesela_ir::Filter>,
}

/// A single result from a vector search.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorResult {
    /// The matched record.
    pub record: Record,
    /// L2 distance (or cosine similarity, depending on the backend).
    pub distance: f32,
}

/// Backend that stores and searches embedding vectors.
pub trait VectorBackend: Send + Sync {
    /// Search for the `top_k` nearest neighbours of `query`.
    fn vector_search(&self, query: &VectorSearchQuery) -> Result<Vec<VectorResult>, Error>;
    /// Add or update the vector for a record identified by `pk`.
    fn index_vector(&self, object_type: &ApiName, pk: &Value, vector: &[f32]) -> Result<(), Error>;
    /// Remove the vector for a record (called on delete).
    fn delete_vector(&self, object_type: &ApiName, pk: &Value) -> Result<(), Error>;
}

/// Converts text into an embedding vector.
pub trait Embedder: Send + Sync {
    /// Embed `text` and return a float vector of the model's native dimension.
    fn embed(&self, text: &str) -> Result<Vec<f32>, Error>;
}

/// Edge kind for a runtime lineage record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageKind {
    /// The source produced the target.
    Produces,
    /// The source consumed the target as input.
    Consumes,
    /// The target was derived from the source.
    DerivesFrom,
}

/// A runtime lineage record linking two object instances.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LineageRecord {
    /// Unique ID.
    pub id: String,
    /// Source object type.
    pub source_object_type: ApiName,
    /// Source record primary key.
    pub source_pk: Value,
    /// Target object type.
    pub target_object_type: ApiName,
    /// Target record primary key.
    pub target_pk: Value,
    /// Edge semantics.
    pub edge_kind: LineageKind,
    /// Actor who caused the write.
    pub actor_user_id: String,
    /// RFC 3339 timestamp.
    pub occurred_at: String,
    /// Pipeline that caused the write, if any.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub pipeline: Option<ApiName>,
}

/// Stores and queries runtime lineage edges.
pub trait LineageStore: Send + Sync {
    /// Record a new lineage edge.
    fn record(&self, edge: LineageRecord) -> Result<(), Error>;
    /// Return all edges connected to a given record up to `depth` hops.
    fn query_lineage(
        &self,
        object_type: &ApiName,
        pk: &Value,
        depth: Option<u32>,
    ) -> Result<Vec<LineageRecord>, Error>;
}

/// Plans and executes schema migrations derived from a spec diff.
pub trait MigrationExecutor: Send + Sync {
    /// Build a migration plan from a compiler diff without executing it.
    fn plan(&self, diff: &tesela_compiler::Diff) -> Result<MigrationPlan, Error>;
    /// Execute a previously computed migration plan against the given backend.
    fn execute(&self, plan: &MigrationPlan, backend: &dyn super::Backend) -> Result<(), Error>;
    /// Roll back a migration plan (best-effort; not always possible).
    fn rollback(&self, plan: &MigrationPlan, backend: &dyn super::Backend) -> Result<(), Error>;
}

/// Stores draft spec branches.
pub trait BranchStore: Send + Sync {
    /// Create a new draft branch forked from `base`.
    fn create_branch(&self, base: &Spec, display: &str, author: &str) -> Result<Branch, Error>;
    /// Retrieve a branch by ID.
    fn get_branch(&self, id: &str) -> Result<Option<Branch>, Error>;
    /// Replace the draft spec on an existing branch.
    fn update_draft(&self, id: &str, spec: Spec) -> Result<(), Error>;
    /// Transition status (e.g. Draft → Review).
    fn set_status(&self, id: &str, status: BranchStatus) -> Result<(), Error>;
    /// List all branches (any status).
    fn list_branches(&self) -> Result<Vec<Branch>, Error>;
    /// Permanently remove a branch record.
    fn delete_branch(&self, id: &str) -> Result<(), Error>;
}

/// Rewrite plan for an aggregate query.
#[derive(Debug, Clone)]
pub struct AggregatePlan {
    /// Whether the backend can execute the aggregate natively.
    pub push_down: bool,
    /// Backend-specific serialised query (populated when `push_down` is true).
    pub native_query: Option<Value>,
    /// Pure-Rust fallback query (always populated).
    pub fallback_query: AggregateQuery,
    /// Estimated relative cost (lower is better).
    pub estimated_cost: u64,
}

/// Plans query execution to take advantage of backend-native capabilities.
pub trait QueryPlanner: Send + Sync {
    /// Decide whether to push an aggregate query down to the backend.
    fn plan_aggregate(
        &self,
        object_type: &ObjectType,
        query: &AggregateQuery,
    ) -> Result<AggregatePlan, Error>;
}

/// A search query directed at a specific datasource.
#[derive(Debug, Clone)]
pub struct FederatedQuery {
    /// Datasource to search.
    pub datasource: ApiName,
    /// Object type within that datasource.
    pub object_type: ApiName,
    /// Search predicate and pagination.
    pub query: Query,
}

/// Executes a fan-out search across multiple backends and merges results.
pub trait FederatedBackend: Send + Sync {
    /// Execute each query on its respective backend and return the merged page.
    fn federated_search(&self, queries: &[FederatedQuery]) -> Result<Page, Error>;
}

/// Executes a transform pipeline DAG.
pub trait PipelineExecutor: Send + Sync {
    /// Execute `pipeline` in the given `mode`.
    fn execute(
        &self,
        pipeline: &TransformPipeline,
        mode: tesela_ir::ExecutionMode,
    ) -> Result<PipelineResult, Error>;
}
