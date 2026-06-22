#![deny(warnings)]
#![deny(missing_docs)]

//! Tesela — ontology-driven application runtime.
//!
//! This is the top-level facade crate that re-exports all Tesela subsystems.
//! For most use cases, a single `use tesela::*;` is sufficient.
//!
//! # Crate structure
//!
//! | Crate | Purpose |
//! |-------|---------|
//! | `tesela-core` | Core types: `Value`, `ApiName`, `Error`, `DataType`, `FilterOp` |
//! | `tesela-ir` | Intermediate representation: `Spec`, `ObjectType`, `LinkType`, … |
//! | `tesela-graph` | Dependency graph analysis over specs |
//! | `tesela-compiler` | Validation pipeline: compiler passes over `Spec` |
//! | `tesela-runtime` | Runtime engine: query, mutation, action, agent execution |
//! | `tesela-memory` | In-memory backend implementation |
//! | `tesela-server` | Axum-based HTTP REST server |
//! | `tesela-graphql` | async-graphql dynamic schema integration |
//! | `tesela-mcp` | Model Context Protocol (JSON-RPC 2.0) server |
//! | `tesela-sdk` | Fluent builder API for defining `Spec` objects |

/// Optional runtime adapters re-exported by the facade crate.
pub mod adapters {
    /// APXM agent runtime adapter.
    #[cfg(feature = "adapter-apxm")]
    pub mod apxm {
        pub use tesela_adapter_apxm::*;
    }

    /// BigQuery backend adapter.
    #[cfg(feature = "adapter-bigquery")]
    pub mod bigquery {
        pub use tesela_adapter_bigquery::*;
    }

    /// Google Cloud Storage object-store adapter.
    #[cfg(feature = "adapter-gcs")]
    pub mod gcs {
        pub use tesela_adapter_gcs::*;
    }
}

pub use tesela_compiler as compiler;
pub use tesela_core as core;
pub use tesela_graph as graph;
pub use tesela_graphql as graphql;
pub use tesela_ir as ir;
pub use tesela_mcp as mcp;
pub use tesela_memory as memory;
pub use tesela_runtime as runtime;
pub use tesela_sdk as sdk;
pub use tesela_server as server;

#[cfg(feature = "macros")]
pub use tesela_macros;
#[cfg(feature = "macros")]
pub use tesela_macros::{Agent, ObjectType, action};

// ---------------------------------------------------------------------------
// Commonly-used re-exports at the top level for ergonomic usage
// ---------------------------------------------------------------------------

pub use tesela_core::{ApiName, DataType, Error, FilterOp, LinkCardinality, Operation, Value};
pub use tesela_ir::{
    AggregateResult, Datasource, Filter, LinkMapping, LinkSource, LinkType, MutationResult,
    ObjectSet, ObjectSource, ObjectType, Page, Property, Record, SetOp, Spec,
};
pub use tesela_memory::MemoryBackend;
pub use tesela_runtime::agents::ontology_tools;
pub use tesela_runtime::ports::{
    Aggregator, AuditSink, Backend, DefaultBackendRegistry, EventBus, Getter, Mutator,
    PolicyEvaluator, Searcher,
};
pub use tesela_runtime::query::{
    Actor, AggregateQuery, Aggregation, AuditRecord, BackendCapabilities, Event, Mutation,
    PolicyDecision, PolicyRequest, Query,
};
pub use tesela_runtime::runtime::{Runtime, RuntimeOptions};
pub use tesela_sdk::App;
