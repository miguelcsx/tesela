//! Object set (saved query) types.

use lattice_core::{ApiName, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::Filter;

/// A named, shareable, composable query (Foundry Object Set equivalent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSet {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Target object type.
    pub object_type: ApiName,
    /// Filter predicate (evaluated at query time, not materialised).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub filter: Option<Filter>,
    /// Default sort order.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sort: Vec<ObjectSetSort>,
    /// Maximum rows returned when resolving the set.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub limit: Option<i32>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// Sort directive within an object set definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSetSort {
    /// Property to sort by.
    pub property: ApiName,
    /// Direction: `asc` or `desc`.
    pub direction: SortDirection,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    /// Ascending.
    Asc,
    /// Descending.
    Desc,
}

/// Set composition operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetOp {
    /// Union of all sets.
    Union,
    /// Intersection of all sets.
    Intersect,
    /// Set difference (first minus rest).
    Subtract,
}
