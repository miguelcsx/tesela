#![deny(warnings)]
#![deny(missing_docs)]

//! PostgreSQL backend adapter for the Lattice runtime.
//!
//! Implements the [`Backend`], [`Searcher`], [`Getter`], [`Mutator`],
//! [`Aggregator`], [`Traverser`], [`BulkLoader`], [`Rollbacker`], and
//! [`SearchExplainer`] port traits backed by a PostgreSQL connection pool.
//!
//! # Usage
//!
//! ```rust,ignore
//! use lattice_adapter_postgres::PostgresBackend;
//!
//! let backend = PostgresBackend::connect("postgres://localhost/lattice").await?;
//! registry.register(ApiName::new("postgres")?, backend);
//! ```

mod backend;

pub use backend::PostgresBackend;
