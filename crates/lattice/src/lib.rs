#![deny(warnings)]
#![deny(missing_docs)]

//! Lattice — ontology-driven application runtime.
//!
//! This is the top-level facade crate that re-exports all Lattice subsystems.
//! For most use cases, a single `use lattice::*;` is sufficient.
//!
//! # Crate structure
//!
//! | Crate | Purpose |
//! |-------|---------|
//! | `lattice-core` | Core types: `Value`, `ApiName`, `Error`, `DataType`, `FilterOp` |
//! | `lattice-ir` | Intermediate representation: `Spec`, `ObjectType`, `LinkType`, … |
//! | `lattice-graph` | Dependency graph analysis over specs |
//! | `lattice-compiler` | Validation pipeline: compiler passes over `Spec` |
//! | `lattice-runtime` | Runtime engine: query, mutation, action, agent execution |
//! | `lattice-memory` | In-memory backend implementation |
//! | `lattice-server` | Axum-based HTTP REST server |
//! | `lattice-graphql` | async-graphql dynamic schema integration |
//! | `lattice-mcp` | Model Context Protocol (JSON-RPC 2.0) server |
//! | `lattice-sdk` | Fluent builder API for defining `Spec` objects |

pub use lattice_compiler as compiler;
pub use lattice_core as core;
pub use lattice_graph as graph;
pub use lattice_graphql as graphql;
pub use lattice_ir as ir;
pub use lattice_mcp as mcp;
pub use lattice_memory as memory;
pub use lattice_runtime as runtime;
pub use lattice_sdk as sdk;
pub use lattice_server as server;

#[cfg(feature = "macros")]
pub use lattice_macros;
#[cfg(feature = "macros")]
pub use lattice_macros::{action, Agent, ObjectType};

// ---------------------------------------------------------------------------
// Commonly-used re-exports at the top level for ergonomic usage
// ---------------------------------------------------------------------------

pub use lattice_core::{ApiName, DataType, Error, FilterOp, Value};
pub use lattice_ir::Spec;
pub use lattice_memory::MemoryBackend;
pub use lattice_runtime::runtime::{Runtime, RuntimeOptions};
pub use lattice_sdk::App;
