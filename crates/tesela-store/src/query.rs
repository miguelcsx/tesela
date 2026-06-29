//! Query, actor, policy, and execution request types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Operation, Value};

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
    /// Offset pagination start.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub offset: Option<i32>,
    /// Cursor pagination token.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cursor: Option<String>,
}

impl Query {
    /// Merge an additional filter with AND semantics.
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
    /// Sort direction.
    pub direction: SortDirection,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

/// Mutation payload accepted by the runtime convenience API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mutation {
    /// Create a record.
    Create {
        /// Field values.
        values: BTreeMap<ApiName, Value>,
    },
    /// Update a record.
    Update {
        /// Primary key.
        primary_key: Value,
        /// New values.
        values: BTreeMap<ApiName, Value>,
    },
    /// Delete a record.
    Delete {
        /// Primary key.
        primary_key: Value,
    },
    /// Insert or replace a record.
    Upsert {
        /// Field values.
        values: BTreeMap<ApiName, Value>,
    },
    /// Apply several mutations.
    Batch {
        /// Mutations.
        items: Vec<Mutation>,
    },
}

/// Aggregate query.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AggregateQuery {
    /// Filter before aggregation.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filter: Option<tesela_ir::Filter>,
    /// Group-by properties.
    #[serde(default)]
    pub group_by: Vec<ApiName>,
    /// Aggregations.
    #[serde(default)]
    pub aggregations: Vec<Aggregation>,
    /// Require backend pushdown.
    #[serde(default)]
    pub require_pushdown: bool,
}

/// A single aggregation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aggregation {
    /// Aggregation function.
    pub function: AggregationFunction,
    /// Property to aggregate.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub property: Option<ApiName>,
    /// Output alias.
    pub alias: String,
}

/// Supported aggregation functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregationFunction {
    /// Count records.
    Count,
    /// Sum numeric values.
    Sum,
    /// Average numeric values.
    Avg,
    /// Minimum value.
    Min,
    /// Maximum value.
    Max,
}

impl AggregationFunction {
    /// SQL function name for pushdown-capable stores.
    #[must_use]
    pub fn as_sql_name(self) -> &'static str {
        match self {
            Self::Count => "COUNT",
            Self::Sum => "SUM",
            Self::Avg => "AVG",
            Self::Min => "MIN",
            Self::Max => "MAX",
        }
    }
}

/// Link traversal query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalQuery {
    /// Starting primary key in the source object.
    pub source_pk: Value,
    /// Optional target-side filter.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filter: Option<tesela_ir::Filter>,
    /// Sort directives.
    #[serde(default)]
    pub sort: Vec<Sort>,
    /// Limit.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limit: Option<i32>,
    /// Offset.
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
    /// Optional idempotency key or run id.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub run_id: Option<String>,
}

/// Authenticated actor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    /// User or service ID.
    pub user_id: String,
    /// Assigned roles.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Arbitrary claims.
    #[serde(default)]
    pub claims: BTreeMap<String, Value>,
}

/// Policy request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRequest {
    /// Actor.
    pub actor: Actor,
    /// Operation.
    pub operation: Operation,
    /// Resource kind.
    pub resource_kind: String,
    /// Resource name.
    pub resource: ApiName,
    /// Extra context.
    #[serde(default)]
    pub context: BTreeMap<String, Value>,
}

/// Policy decision.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// Whether the operation is allowed.
    pub allow: bool,
    /// Denial reason or note.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
    /// Row-level filter.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub row_filter: Option<tesela_ir::Filter>,
    /// Properties to redact.
    #[serde(default)]
    pub redactions: Vec<ApiName>,
}

/// Store capability advertisement.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StoreCapabilities {
    /// Supports search.
    pub search: bool,
    /// Supports get.
    pub get: bool,
    /// Supports create.
    pub create: bool,
    /// Supports update.
    pub update: bool,
    /// Supports delete.
    pub delete: bool,
    /// Supports aggregate.
    pub aggregate: bool,
    /// Supports traversal.
    pub traverse: bool,
    /// Supports action execution.
    pub execute_action: bool,
}
