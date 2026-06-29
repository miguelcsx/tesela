use std::collections::BTreeMap;

use tesela_core::{ApiName, Value};
use tesela_ir::Record;

use super::{
    SnapshotDefault, VersionedRecordSchema, next_version, restore_values, snapshot_record,
};

#[test]
fn snapshot_uses_declared_fields_and_defaults() {
    let name = ApiName::from("name");
    let status = ApiName::from("status");
    let version = ApiName::from("version");
    let schema_fields = [name.clone(), status.clone()];
    let schema = VersionedRecordSchema::new(&schema_fields, &version);
    let active = Value::string("active");
    let defaults = [SnapshotDefault {
        field: &status,
        value: &active,
    }];
    let mut values = BTreeMap::new();
    values.insert(name, Value::string("Road layer"));
    let record = Record {
        primary_key: None,
        values,
    };

    let snapshot = snapshot_record(&record, &schema, &defaults);

    assert_eq!(
        snapshot.as_object().and_then(|object| object.get("name")),
        Some(&serde_json::Value::from("Road layer"))
    );
    assert_eq!(
        snapshot.as_object().and_then(|object| object.get("status")),
        Some(&serde_json::Value::from("active"))
    );
}

#[test]
fn restore_values_projects_allowed_fields_only() -> Result<(), tesela_core::Error> {
    let name = ApiName::from("name");
    let status = ApiName::from("status");
    let fields = [name.clone(), status.clone()];
    let snapshot = Value::new(serde_json::json!({
        "name": "Trips",
        "status": "archived",
        "ignored": true
    }));

    let values = restore_values(&snapshot, &fields)?;

    assert_eq!(values.get(&name), Some(&Value::string("Trips")));
    assert_eq!(values.get(&status), Some(&Value::string("archived")));
    assert_eq!(values.len(), 2);
    Ok(())
}

#[test]
fn next_version_advances_max_existing_version() {
    let version = ApiName::from("version");
    let schema = VersionedRecordSchema::new(&[], &version);
    let records = [version_record(&version, 1), version_record(&version, 7)];

    assert_eq!(next_version(&records, &schema), 8);
    assert_eq!(next_version(&[], &schema), 1);
}

fn version_record(field: &ApiName, version: i64) -> Record {
    let mut values = BTreeMap::new();
    values.insert(field.clone(), Value::integer(version));
    Record {
        primary_key: None,
        values,
    }
}
