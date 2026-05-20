//! Schema migration and branch types.

use lattice_core::{ApiName, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::Spec;

/// A structural migration step derived from a spec diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationStep {
    /// What kind of change this step represents.
    pub kind: MigrationKind,
    /// Object type affected.
    pub object_type: ApiName,
    /// Additional parameters (property names, type details, etc.).
    #[serde(default)]
    pub details: BTreeMap<String, Value>,
}

/// Classification of a schema change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationKind {
    /// Add a new property column.
    AddProperty,
    /// Drop an existing property column (destructive).
    RemoveProperty,
    /// Rename a property column.
    RenameProperty,
    /// Change a property's data type (may require backfill).
    ChangePropertyType,
    /// Create a new object type table / collection.
    AddObjectType,
    /// Drop an object type table / collection (destructive).
    RemoveObjectType,
    /// Create an index.
    AddIndex,
    /// Drop an index.
    RemoveIndex,
}

/// A migration plan derived from a [`lattice_compiler::Diff`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Ordered list of migration steps.
    pub steps: Vec<MigrationStep>,
    /// Whether any step is destructive (data loss possible).
    pub is_destructive: bool,
    /// Whether any step requires a data backfill.
    pub requires_backfill: bool,
}

/// A draft ontology branch (Foundry Branch equivalent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Branch {
    /// Unique branch identifier.
    pub id: String,
    /// Human-readable name.
    pub display: String,
    /// SHA-256 hash of the spec the branch was forked from.
    pub base_spec_hash: String,
    /// The proposed spec changes (full spec snapshot).
    pub draft_spec: Spec,
    /// Current lifecycle status.
    pub status: BranchStatus,
    /// RFC 3339 creation timestamp.
    pub created_at: String,
    /// Author user ID.
    pub author: String,
}

/// Lifecycle status of a branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchStatus {
    /// Under active development.
    Draft,
    /// Submitted for peer review.
    Review,
    /// Merged into the live spec.
    Merged,
    /// Discarded without merging.
    Discarded,
}
