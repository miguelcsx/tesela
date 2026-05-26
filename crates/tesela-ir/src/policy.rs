//! Role, policy rule, obligation, and filter types.

use tesela_core::{ApiName, FilterOp, Operation, PolicyEffect, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A role definition with inheritance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Parent roles this role inherits from.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub inherits: Vec<ApiName>,
}

/// A policy rule for access control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRule {
    /// API name.
    pub api_name: ApiName,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Allow or deny.
    pub effect: PolicyEffect,
    /// Specific actor user IDs this rule applies to.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub actors: Vec<String>,
    /// Roles this rule applies to.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub roles: Vec<String>,
    /// Operations this rule applies to.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub operations: Vec<Operation>,
    /// Resource kind (object_type, action, link_type, etc.).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resource_kind: Option<String>,
    /// Specific resource API name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resource: Option<ApiName>,
    /// Optional CEL-style condition expression.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub condition: Option<String>,
    /// Row-level filter to apply.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub row_filter: Option<Filter>,
    /// Fields to redact.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub redactions: Vec<ApiName>,
    /// Obligations to execute.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub obligations: Vec<Obligation>,
    /// Priority (higher = evaluated first).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub priority: Option<i32>,
}

/// An obligation attached to an allow policy decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obligation {
    /// Kind of obligation (notify, log, mask, etc.).
    pub kind: String,
    /// Parameters.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub params: Option<BTreeMap<String, Value>>,
}

/// A filter AST node for queries and policy row filters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filter {
    /// Operator.
    pub op: FilterOp,
    /// Field name (for scalar operators).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub field: Option<ApiName>,
    /// Single value (for scalar operators).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<Value>,
    /// Multiple values (for In, NotIn, Between).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub values: Vec<Value>,
    /// Sub-filters (for And, Or, Not).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub args: Vec<Filter>,
}

impl Filter {
    /// Create an equality filter.
    pub fn eq(field: ApiName, value: impl Into<Value>) -> Self {
        Self {
            op: FilterOp::Eq,
            field: Some(field),
            value: Some(value.into()),
            values: Vec::new(),
            args: Vec::new(),
        }
    }

    /// Create an AND filter combining multiple sub-filters.
    pub fn and(filters: Vec<Filter>) -> Self {
        Self {
            op: FilterOp::And,
            field: None,
            value: None,
            values: Vec::new(),
            args: filters,
        }
    }

    /// Create an OR filter combining multiple sub-filters.
    pub fn or(filters: Vec<Filter>) -> Self {
        Self {
            op: FilterOp::Or,
            field: None,
            value: None,
            values: Vec::new(),
            args: filters,
        }
    }

    /// Create a NOT filter.
    pub fn negate(filter: Filter) -> Self {
        Self {
            op: FilterOp::Not,
            field: None,
            value: None,
            values: Vec::new(),
            args: vec![filter],
        }
    }
}
