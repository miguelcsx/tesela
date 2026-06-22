//! Structural diff engine for specs.

use crate::hash::HasApiName;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::ApiName;
use tesela_ir::Spec;

/// A single entry in a diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffEntry {
    /// Entity API name.
    pub api_name: ApiName,
    /// Collection kind (object_type, action, etc.).
    pub kind: String,
    /// Whether this change is breaking.
    pub breaking: bool,
    /// Human-readable description.
    pub description: String,
}

/// Computed diff between two specs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Diff {
    /// Entities present in new but not in old.
    pub added: Vec<DiffEntry>,
    /// Entities present in old but not in new.
    pub removed: Vec<DiffEntry>,
    /// Entities present in both but with changes.
    pub changed: Vec<DiffEntry>,
}

impl Diff {
    /// Whether any breaking change exists.
    pub fn has_breaking_changes(&self) -> bool {
        self.removed.iter().any(|e| e.breaking) || self.changed.iter().any(|e| e.breaking)
    }
}

/// Compute the structural diff between two specs.
pub fn compute_diff(old: &Spec, new: &Spec) -> Diff {
    let mut diff = Diff::default();

    fn compare_collections<T: HasApiName + serde::Serialize>(
        diff: &mut Diff,
        old_items: &[T],
        new_items: &[T],
        kind: &str,
        is_breaking_remove: fn(&T) -> bool,
        is_breaking_change: fn(&T, &T) -> bool,
    ) {
        let old_map: BTreeMap<ApiName, &T> = old_items
            .iter()
            .map(|item| (item.api_name().clone(), item))
            .collect();
        let new_map: BTreeMap<ApiName, &T> = new_items
            .iter()
            .map(|item| (item.api_name().clone(), item))
            .collect();

        for (name, old_item) in &old_map {
            if let Some(new_item) = new_map.get(name) {
                let changed = match (
                    serde_json::to_value(*old_item),
                    serde_json::to_value(*new_item),
                ) {
                    (Ok(old_json), Ok(new_json)) => old_json != new_json,
                    _ => true,
                };
                if !changed {
                    continue;
                }
                diff.changed.push(DiffEntry {
                    api_name: name.clone(),
                    kind: kind.to_string(),
                    breaking: is_breaking_change(*old_item, *new_item),
                    description: format!("{} '{}' changed", kind, name),
                });
            } else {
                diff.removed.push(DiffEntry {
                    api_name: name.clone(),
                    kind: kind.to_string(),
                    breaking: is_breaking_remove(*old_item),
                    description: format!("{} '{}' removed", kind, name),
                });
            }
        }

        for name in new_map.keys() {
            if !old_map.contains_key(name) {
                diff.added.push(DiffEntry {
                    api_name: name.clone(),
                    kind: kind.to_string(),
                    breaking: false,
                    description: format!("{} '{}' added", kind, name),
                });
            }
        }
    }

    compare_collections(
        &mut diff,
        &old.object_types,
        &new.object_types,
        "object_type",
        |ot| ot.deprecated_at.is_none(),
        |old_ot, new_ot| {
            old_ot.primary_key != new_ot.primary_key
                || old_ot.properties.len() > new_ot.properties.len()
        },
    );
    compare_collections(
        &mut diff,
        &old.link_types,
        &new.link_types,
        "link_type",
        |_| true,
        |_, _| true,
    );
    compare_collections(
        &mut diff,
        &old.actions,
        &new.actions,
        "action",
        |_| true,
        |old_a, new_a| old_a.handler.kind != new_a.handler.kind,
    );
    compare_collections(
        &mut diff,
        &old.roles,
        &new.roles,
        "role",
        |_| false,
        |_, _| false,
    );
    compare_collections(
        &mut diff,
        &old.policies,
        &new.policies,
        "policy",
        |_| true,
        |_, _| true,
    );
    compare_collections(
        &mut diff,
        &old.agents,
        &new.agents,
        "agent",
        |_| false,
        |_, _| false,
    );
    compare_collections(
        &mut diff,
        &old.datasources,
        &new.datasources,
        "datasource",
        |_| true,
        |_, _| true,
    );
    compare_collections(
        &mut diff,
        &old.traits,
        &new.traits,
        "trait",
        |_| false,
        |_, _| false,
    );
    compare_collections(
        &mut diff,
        &old.custom_tools,
        &new.custom_tools,
        "custom_tool",
        |_| false,
        |_, _| false,
    );
    compare_collections(
        &mut diff,
        &old.assets,
        &new.assets,
        "asset",
        |_| false,
        |_, _| false,
    );
    compare_collections(
        &mut diff,
        &old.environments,
        &new.environments,
        "environment",
        |_| false,
        |_, _| false,
    );

    diff
}
