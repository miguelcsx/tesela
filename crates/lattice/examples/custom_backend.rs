//! Implement the `Backend` trait for a custom dummy backend and register it.
//!
//! This example shows the minimum required to implement a custom backend adapter.

use lattice::memory::DefaultBackendRegistry;
use lattice::runtime::ports::{Backend, BackendRegistry, Getter, Mutator, Searcher};
use lattice::runtime::query::{Actor, BackendCapabilities, Mutation, Query};
use lattice::runtime::runtime::{Runtime, RuntimeOptions};
use lattice::sdk::{App, ObjectTypeBuilder, PropertyBuilder};
use lattice_core::{ApiName, DataType, Error, Value};
use lattice_ir::{MutationResult, Page, Record};
use std::collections::BTreeMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Custom backend
// ---------------------------------------------------------------------------

/// A read-only backend that always returns a single hardcoded record.
struct HardcodedBackend;

impl Backend for HardcodedBackend {
    fn backend_type(&self) -> &str {
        "hardcoded"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            search: true,
            get: true,
            mutate: false,
            aggregate: false,
            traverse: false,
            bulk_load: false,
            rollback: false,
            explain: false,
        }
    }

    fn close(&self) -> Result<(), Error> {
        Ok(())
    }

    fn as_searcher(&self) -> Option<&dyn Searcher> {
        Some(self)
    }

    fn as_getter(&self) -> Option<&dyn Getter> {
        Some(self)
    }

    fn as_mutator(&self) -> Option<&dyn Mutator> {
        Some(self)
    }
}

impl Searcher for HardcodedBackend {
    fn search(&self, _object_type: &ApiName, _query: &Query) -> Result<Page, Error> {
        Ok(Page {
            records: vec![hardcoded_record()],
            next_cursor: None,
        })
    }
}

impl Getter for HardcodedBackend {
    fn get(&self, _object_type: &ApiName, _pk: &Value) -> Result<Option<Record>, Error> {
        Ok(Some(hardcoded_record()))
    }
}

impl Mutator for HardcodedBackend {
    fn mutate(
        &self,
        _object_type: &ApiName,
        _mutation: &Mutation,
    ) -> Result<MutationResult, Error> {
        Err(Error::unsupported("hardcoded backend is read-only"))
    }
}

fn hardcoded_record() -> Record {
    let mut values = BTreeMap::new();
    values.insert(ApiName::new_unchecked("id"), Value::string("static-1"));
    values.insert(
        ApiName::new_unchecked("name"),
        Value::string("Hardcoded Item"),
    );
    Record {
        primary_key: Some(Value::string("static-1")),
        values,
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let app = App::new("demo")
        .datasource(lattice::sdk::DatasourceBuilder::new("hardcoded", "hardcoded").build())
        .object_type(
            ObjectTypeBuilder::new("item")
                .datasource("hardcoded")
                .property(
                    PropertyBuilder::new("id", DataType::String)
                        .required(true)
                        .build(),
                )
                .property(PropertyBuilder::new("name", DataType::String).build())
                .build(),
        );

    let result = app.compile();
    assert!(result.is_valid, "compile failed: {:?}", result.diagnostics);

    // Register the custom backend.
    let registry = DefaultBackendRegistry::new();
    let backend = Arc::new(HardcodedBackend);
    registry.register(ApiName::new_unchecked("hardcoded"), backend).unwrap();
    let registry_dyn: Arc<dyn BackendRegistry> = registry;

    let runtime = Runtime::new(
        result.spec.unwrap(),
        RuntimeOptions {
            backend_registry: Some(registry_dyn),
            ..RuntimeOptions::dev()
        },
    )
    .unwrap();

    let actor = Actor {
        user_id: "demo".to_string(),
        roles: vec!["admin".to_string()],
        claims: BTreeMap::new(),
    };

    let page = runtime
        .search(&actor, &ApiName::new_unchecked("item"), Query::default())
        .unwrap();

    println!("Found {} record(s):", page.records.len());
    for r in &page.records {
        println!("  {:?}", r.values.get(&ApiName::new_unchecked("name")));
    }
}
