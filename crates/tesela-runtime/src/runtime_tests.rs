use std::collections::BTreeMap;
use std::sync::Arc;

use tesela_core::{ApiName, DataType, Error, Value, Version};
use tesela_ir::{
    ActionHandler, ActionResult, ActionType, Datasource, MutationResult, ObjectSource, ObjectType,
    Page, Property, Record, Spec,
};
use tesela_store::{
    ActionRequest, Actor, AuditEvent, AuditSink, DenyAllPolicy, MemoryStore, Mutation,
    OntologyStore, Query, StaticStoreRouter, StoreCapabilities,
};

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

struct ActionStore(Arc<MemoryStore>);

impl OntologyStore for ActionStore {
    fn store_type(&self) -> &str {
        "action_test"
    }
    fn capabilities(&self) -> StoreCapabilities {
        let mut value = self.0.capabilities();
        value.execute_action = true;
        value
    }
    fn search(&self, object_type: &ApiName, query: &Query) -> Result<Page, Error> {
        self.0.search(object_type, query)
    }
    fn get(&self, object_type: &ApiName, primary_key: &Value) -> Result<Option<Record>, Error> {
        self.0.get(object_type, primary_key)
    }
    fn create(
        &self,
        object_type: &ApiName,
        values: BTreeMap<ApiName, Value>,
    ) -> Result<MutationResult, Error> {
        self.0.create(object_type, values)
    }
    fn update(
        &self,
        object_type: &ApiName,
        primary_key: &Value,
        values: BTreeMap<ApiName, Value>,
    ) -> Result<MutationResult, Error> {
        self.0.update(object_type, primary_key, values)
    }
    fn delete(&self, object_type: &ApiName, primary_key: &Value) -> Result<MutationResult, Error> {
        self.0.delete(object_type, primary_key)
    }
    fn execute_action(&self, request: ActionRequest) -> Result<ActionResult, Error> {
        Ok(ActionResult {
            status: "success".into(),
            output: Some(request.input),
            error: None,
            run_id: request.run_id,
        })
    }
}

#[derive(Default)]
struct TestAudit(std::sync::Mutex<Vec<AuditEvent>>);

impl AuditSink for TestAudit {
    fn record(&self, event: AuditEvent) -> Result<(), Error> {
        self.0
            .lock()
            .map_err(|error| Error::internal(error.to_string()))?
            .push(event);
        Ok(())
    }
}

#[test]
fn actions_are_routed_authorized_and_audited() -> Result<(), Error> {
    let mut spec = analytics_spec()?;
    spec.actions.push(ActionType {
        api_name: api_name("sync_zones")?,
        display: None,
        description: None,
        subject: Some(api_name("zones")?),
        handler: ActionHandler {
            kind: "callback".into(),
            target: None,
            config: None,
        },
        input_schema: None,
        output_schema: None,
        mode: None,
        risk_level: None,
        idempotency_key: None,
        deprecated_at: None,
        metadata: None,
    });
    let memory = MemoryStore::new();
    memory.set_spec(spec.clone())?;
    let router = Arc::new(StaticStoreRouter::new());
    router.register(api_name("analytics")?, Arc::new(ActionStore(memory)))?;
    let audit = Arc::new(TestAudit::default());
    let runtime = Runtime::new(
        spec.clone(),
        RuntimeOptions {
            store_router: Some(router.clone()),
            policy_engine: Some(Arc::new(AllowAllPolicy)),
            audit_sink: Some(audit.clone()),
            ..RuntimeOptions::default()
        },
    )?;
    let request = ActionRequest {
        action: api_name("sync_zones")?,
        input: Value::string("ok"),
        actor: test_actor(),
        run_id: None,
    };
    assert_eq!(
        runtime.execute_action(request.clone())?.output,
        Some(Value::string("ok"))
    );
    assert_eq!(
        audit
            .0
            .lock()
            .map_err(|error| Error::internal(error.to_string()))?
            .len(),
        1
    );

    let denied = Runtime::new(
        spec.clone(),
        RuntimeOptions {
            store_router: Some(router),
            policy_engine: Some(Arc::new(DenyAllPolicy)),
            ..RuntimeOptions::default()
        },
    )?;
    assert!(matches!(
        denied.execute_action(request.clone()),
        Err(Error::PolicyDenied { .. })
    ));
    spec.actions[0].subject = None;
    let mut invalid = spec.clone();
    invalid.version = Version::new("tesela.spec.v2");
    assert!(matches!(
        runtime.apply_spec(invalid),
        Err(Error::Validation { .. })
    ));
    runtime.apply_spec(spec)?;
    assert!(matches!(
        runtime.execute_action(request),
        Err(Error::UnsupportedCapability { .. })
    ));
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
