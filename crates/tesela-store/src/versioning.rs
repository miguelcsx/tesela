//! Generic versioning helpers for ontology records.

use std::collections::BTreeMap;

use tesela_core::{ApiName, Error, Value};
use tesela_ir::Record;

/// Declarative field mapping for a versioned object snapshot.
#[derive(Debug, Clone, Copy)]
pub struct VersionedRecordSchema<'a> {
    /// Fields captured into each snapshot.
    pub snapshot_fields: &'a [ApiName],
    /// Field that stores the monotonically increasing version number.
    pub version_field: &'a ApiName,
}

impl<'a> VersionedRecordSchema<'a> {
    /// Create a versioned object schema.
    #[must_use]
    pub fn new(snapshot_fields: &'a [ApiName], version_field: &'a ApiName) -> Self {
        Self {
            snapshot_fields,
            version_field,
        }
    }
}

/// Default value for a snapshot field when the source record has no value.
#[derive(Debug, Clone, Copy)]
pub struct SnapshotDefault<'a> {
    /// Snapshot field.
    pub field: &'a ApiName,
    /// Default value.
    pub value: &'a Value,
}

/// Build a JSON snapshot from a record using a stable field contract.
#[must_use]
pub fn snapshot_record(
    record: &Record,
    schema: &VersionedRecordSchema<'_>,
    defaults: &[SnapshotDefault<'_>],
) -> Value {
    let mut snapshot = serde_json::Map::new();
    for field in schema.snapshot_fields {
        let json_value = match record.values.get(field) {
            Some(value) => serde_json::Value::from(value.clone()),
            None => match default_value(field, defaults) {
                Some(value) => value,
                None => serde_json::Value::Null,
            },
        };
        snapshot.insert(field.to_string(), json_value);
    }
    Value::new(serde_json::Value::Object(snapshot))
}

/// Convert a snapshot back into mutation values for the versioned object.
pub fn restore_values(
    snapshot: &Value,
    fields: &[ApiName],
) -> Result<BTreeMap<ApiName, Value>, Error> {
    let object = snapshot
        .as_object()
        .ok_or_else(|| Error::validation("version snapshot must be an object"))?;
    let mut values = BTreeMap::new();
    for field in fields {
        if let Some(value) = object.get(field.as_ref()) {
            values.insert(field.clone(), Value::new(value.clone()));
        }
    }
    Ok(values)
}

/// Return the next monotonically increasing version for a set of version records.
#[must_use]
pub fn next_version(records: &[Record], schema: &VersionedRecordSchema<'_>) -> i64 {
    records
        .iter()
        .filter_map(|record| record.values.get(schema.version_field))
        .filter_map(Value::as_i64)
        .max()
        .map_or(1, |version| version + 1)
}

fn default_value(field: &ApiName, defaults: &[SnapshotDefault<'_>]) -> Option<serde_json::Value> {
    defaults
        .iter()
        .find(|default| default.field == field)
        .map(|default| serde_json::Value::from(default.value.clone()))
}

#[cfg(test)]
mod tests;
