use std::collections::BTreeMap;
use std::sync::Arc;

use tesela_core::{ApiName, DataType, Error, Value};
use tesela_ir::{Datasource, ObjectSource, ObjectType, Property, Spec};
use tesela_store::{Actor, MemoryStore, Mutation, Query, StaticStoreRouter};

use crate::{AllowAllPolicy, Runtime, RuntimeOptions};

#[test]
fn upsert_uses_declared_object_primary_key() -> Result<(), Error> {
    let spec = analytics_spec()?;
    let store = MemoryStore::new();
    store.set_spec(spec.clone())?;
    let router = Arc::new(StaticStoreRouter::new());
    router.register(api_name("analytics")?, store)?;
    let runtime = Runtime::new(
        spec,
        RuntimeOptions {
            store_router: Some(router),
            policy_engine: Some(Arc::new(AllowAllPolicy)),
            ..RuntimeOptions::default()
        },
    )?;
    let actor = test_actor();
    let zones = api_name("zones")?;

    runtime.mutate(
        &actor,
        &zones,
        Mutation::Upsert {
            values: zone_values("scenario-1", "zone-1")?,
        },
    )?;
    runtime.mutate(
        &actor,
        &zones,
        Mutation::Upsert {
            values: zone_values("scenario-1", "zone-2")?,
        },
    )?;

    let page = runtime.search(&actor, &zones, Query::default())?;
    assert_eq!(page.records.len(), 2);
    assert_eq!(
        runtime
            .get(&actor, &zones, &Value::string("zone-1"))?
            .primary_key,
        Some(Value::string("zone-1"))
    );
    assert_eq!(
        runtime
            .get(&actor, &zones, &Value::string("zone-2"))?
            .primary_key,
        Some(Value::string("zone-2"))
    );
    Ok(())
}

#[test]
fn upsert_requires_declared_primary_key_value() -> Result<(), Error> {
    let spec = analytics_spec()?;
    let store = MemoryStore::new();
    store.set_spec(spec.clone())?;
    let router = Arc::new(StaticStoreRouter::new());
    router.register(api_name("analytics")?, store)?;
    let runtime = Runtime::new(
        spec,
        RuntimeOptions {
            store_router: Some(router),
            policy_engine: Some(Arc::new(AllowAllPolicy)),
            ..RuntimeOptions::default()
        },
    )?;
    let actor = test_actor();
    let mut values = BTreeMap::new();
    values.insert(api_name("scenario_id")?, Value::string("scenario-1"));

    let error = runtime
        .mutate(&actor, &api_name("zones")?, Mutation::Upsert { values })
        .err()
        .ok_or_else(|| Error::internal("upsert without primary key succeeded"))?;

    assert!(matches!(error, Error::BadRequest { .. }));
    Ok(())
}

fn analytics_spec() -> Result<Spec, Error> {
    let mut spec = Spec::default();
    spec.datasources.push(Datasource {
        api_name: api_name("analytics")?,
        adapter_type: "memory".to_string(),
        config: None,
        secrets: None,
    });
    spec.object_types.push(ObjectType {
        api_name: api_name("zones")?,
        display: None,
        description: None,
        source: ObjectSource {
            datasource: api_name("analytics")?,
            resource: Some("zones".to_string()),
        },
        primary_key: api_name("zone_id")?,
        properties: vec![
            property("scenario_id", DataType::String)?,
            property("zone_id", DataType::String)?,
        ],
        traits: Vec::new(),
        tags: Vec::new(),
        metadata: None,
        indexes: Vec::new(),
        deprecated_at: None,
    });
    Ok(spec)
}

fn zone_values(scenario_id: &str, zone_id: &str) -> Result<BTreeMap<ApiName, Value>, Error> {
    let mut values = BTreeMap::new();
    values.insert(api_name("scenario_id")?, Value::string(scenario_id));
    values.insert(api_name("zone_id")?, Value::string(zone_id));
    Ok(values)
}

fn property(api_name: &str, data_type: DataType) -> Result<Property, Error> {
    Ok(Property {
        api_name: self::api_name(api_name)?,
        display: None,
        description: None,
        data_type,
        nullable: None,
        indexed: None,
        unique: None,
        tags: Vec::new(),
        markings: Vec::new(),
        metadata: None,
        default: None,
        source_column: None,
        allowed_values: None,
        sort_order: None,
        encrypted: None,
    })
}

fn test_actor() -> Actor {
    Actor {
        user_id: "runtime-test".to_string(),
        roles: vec!["system".to_string()],
        claims: BTreeMap::new(),
    }
}

fn api_name(value: &str) -> Result<ApiName, Error> {
    ApiName::new(value).map_err(|error| Error::bad_request(format!("invalid api name: {error}")))
}
