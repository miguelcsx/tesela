//! Define a spec via SDK, compile it, build a Runtime with MemoryBackend,
//! then search/get/mutate records.

use lattice::memory::{DefaultBackendRegistry, MemoryBackend};
use lattice::runtime::query::{Actor, Mutation, Query};
use lattice::runtime::runtime::{Runtime, RuntimeOptions};
use lattice::sdk::{App, ObjectTypeBuilder, PropertyBuilder};
use lattice_core::{ApiName, DataType, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

fn main() {
    // 1. Define the spec using the fluent SDK.
    let app = App::new("demo").object_type(
        ObjectTypeBuilder::new("product")
            .display("Product")
            .property(
                PropertyBuilder::new("id", DataType::String)
                    .required(true)
                    .build(),
            )
            .property(
                PropertyBuilder::new("name", DataType::String)
                    .required(true)
                    .build(),
            )
            .property(PropertyBuilder::new("price", DataType::Float).build())
            .build(),
    );

    let result = app.compile();
    assert!(result.is_valid, "compile failed: {:?}", result.diagnostics);
    let spec = result.spec.unwrap();
    println!("Compiled spec: {} object type(s)", spec.object_types.len());

    // 2. Build a Runtime backed by MemoryBackend.
    let registry = DefaultBackendRegistry::new();
    let backend = MemoryBackend::new();
    registry.register(ApiName::new_unchecked("memory"), backend).unwrap();
    // DefaultBackendRegistry::new() already returns Arc<Self>; upcast to dyn BackendRegistry.
    let registry_dyn: Arc<dyn lattice::runtime::ports::BackendRegistry> = registry;
    let opts = RuntimeOptions {
        backend_registry: Some(registry_dyn),
        ..RuntimeOptions::dev()
    };
    let runtime = Runtime::new(spec, opts).unwrap();
    let actor = Actor {
        user_id: "demo".to_string(),
        roles: vec!["admin".to_string()],
        claims: BTreeMap::new(),
    };
    let obj = ApiName::new_unchecked("product");

    // 3. Create a record.
    let mut values = BTreeMap::new();
    values.insert(ApiName::new_unchecked("id"), Value::string("p1"));
    values.insert(ApiName::new_unchecked("name"), Value::string("Widget"));
    values.insert(ApiName::new_unchecked("price"), Value::float(9.99));
    runtime
        .mutate(&actor, &obj, Mutation::Create { values })
        .unwrap();

    // 4. Get the record.
    let record = runtime.get(&actor, &obj, &Value::string("p1")).unwrap();
    println!(
        "Retrieved: {:?}",
        record.values.get(&ApiName::new_unchecked("name"))
    );

    // 5. Search all records.
    let page = runtime.search(&actor, &obj, Query::default()).unwrap();
    println!("Total records: {}", page.records.len());
}
