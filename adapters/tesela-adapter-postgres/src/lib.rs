#![deny(warnings)]
#![deny(missing_docs)]

//! PostgreSQL backend adapter for the Tesela runtime.
//!
//! Implements the [`tesela_runtime::ports::Backend`], [`tesela_runtime::ports::Searcher`],
//! [`tesela_runtime::ports::Getter`], [`tesela_runtime::ports::Mutator`],
//! [`tesela_runtime::ports::Aggregator`], [`tesela_runtime::ports::Traverser`],
//! [`tesela_runtime::ports::BulkLoader`], [`tesela_runtime::ports::Rollbacker`], and
//! [`tesela_runtime::ports::SearchExplainer`] port traits backed by a PostgreSQL connection pool.
//!
//! # Usage
//!
//! ```rust,ignore
//! use tesela_adapter_postgres::PostgresBackend;
//!
//! let backend = PostgresBackend::connect("postgres://localhost/tesela").await?;
//! registry.register(ApiName::new("postgres")?, backend);
//! ```

mod backend;

pub use backend::PostgresBackend;
