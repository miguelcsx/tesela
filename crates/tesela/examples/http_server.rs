//! Start a Tesela REST HTTP server backed by MemoryBackend.
//!
//! Run this example and then exercise the API:
//!
//! ```bash
//! cargo run --example http_server -p tesela
//!
//! # Create a record
//! curl -X POST http://localhost:8080/v1/objects/user/mutate \
//!   -H 'Content-Type: application/json' \
//!   -d '{"Create":{"values":{"id":"u1","name":"Alice"}}}'
//!
//! # Get a record
//! curl http://localhost:8080/v1/objects/user/u1
//!
//! # Search records
//! curl -X POST http://localhost:8080/v1/objects/user/search \
//!   -H 'Content-Type: application/json' -d '{}'
//!
//! # Health check
//! curl http://localhost:8080/v1/health
//! ```

use std::collections::BTreeMap;
use std::sync::Arc;
use tesela::memory::{DefaultBackendRegistry, MemoryBackend};
use tesela::runtime::{
    auth::StaticActorResolver,
    query::Actor,
    runtime::{Runtime, RuntimeOptions},
};
use tesela::sdk::{App, ObjectTypeBuilder, PropertyBuilder};
use tesela::server::{Server, ServerOptions};
use tesela_core::{ApiName, DataType};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = App::new("demo").object_type(
        ObjectTypeBuilder::new("user")
            .display("User")
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

    let registry = DefaultBackendRegistry::new();
    registry
        .register(ApiName::new_unchecked("memory"), MemoryBackend::new())
        .unwrap();
    let registry_dyn: Arc<dyn tesela::runtime::ports::BackendRegistry> = registry;

    let runtime = Arc::new(
        Runtime::new(
            result.spec.unwrap(),
            RuntimeOptions {
                backend_registry: Some(registry_dyn),
                ..RuntimeOptions::dev()
            },
        )
        .unwrap(),
    );

    let actor = Actor {
        user_id: "dev".to_string(),
        roles: vec!["admin".to_string()],
        claims: BTreeMap::new(),
    };
    let server = Server::new(
        ServerOptions::new(Arc::clone(&runtime))
            .with_actor_resolver(Arc::new(StaticActorResolver::new(actor))),
    );
    println!("Listening on http://0.0.0.0:8080");
    server.serve("0.0.0.0:8080").await.unwrap();
}
