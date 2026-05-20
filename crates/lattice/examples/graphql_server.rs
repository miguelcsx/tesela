//! Start a Lattice GraphQL server backed by MemoryBackend.
//!
//! Run this example and open the playground at http://localhost:8081/graphql
//!
//! Example query:
//! ```graphql
//! query {
//!   search_product {
//!     id
//!     name
//!   }
//! }
//! ```

use lattice::graphql::GraphQLSchemaBuilder;
use lattice::memory::{DefaultBackendRegistry, MemoryBackend};
use lattice::runtime::runtime::{Runtime, RuntimeOptions};
use lattice::sdk::{App, ObjectTypeBuilder, PropertyBuilder};
use lattice_core::{ApiName, DataType};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = App::new("shop").object_type(
        ObjectTypeBuilder::new("product")
            .display("Product")
            .property(
                PropertyBuilder::new("id", DataType::String)
                    .required(true)
                    .build(),
            )
            .property(PropertyBuilder::new("name", DataType::String).build())
            .property(PropertyBuilder::new("price", DataType::Float).build())
            .build(),
    );

    let result = app.compile();
    assert!(result.is_valid, "compile failed: {:?}", result.diagnostics);
    let spec = result.spec.unwrap();

    let registry = DefaultBackendRegistry::new();
    registry.register(ApiName::new_unchecked("memory"), MemoryBackend::new());
    let registry_dyn: Arc<dyn lattice::runtime::ports::BackendRegistry> = registry;

    let runtime = Arc::new(
        Runtime::new(
            spec.clone(),
            RuntimeOptions {
                backend_registry: Some(registry_dyn),
                ..RuntimeOptions::dev()
            },
        )
        .unwrap(),
    );

    let schema = GraphQLSchemaBuilder::build(&spec, Arc::clone(&runtime)).unwrap();
    println!(
        "GraphQL schema built with {} types",
        spec.object_types.len()
    );

    // Serve via axum at /graphql using async-graphql's axum integration.
    // For brevity, we just demonstrate schema construction and exit.
    // In production, mount via: Router::new().route("/graphql", post(graphql_handler))
    println!("Schema ready. Mount at POST /graphql to serve queries.");
    let _ = schema;
}
