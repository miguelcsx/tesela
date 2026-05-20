//! Runtime engine for Lattice ontologies.
//!
//! Provides the execution engine, policy evaluation, action dispatch,
//! agent runtime, audit, events, and all port traits.

#![deny(warnings)]
#![deny(missing_docs)]

pub mod actions;
pub mod agents;
pub mod audit;
pub mod auth;
pub mod branch;
pub mod computed;
pub mod config;
pub mod constants;
pub mod context_engineering;
pub mod crypto;
pub mod encrypt;
pub mod evals;
pub mod events;
pub mod federated;
pub mod interceptors;
pub mod lineage;
pub mod pipeline;
pub mod policy;
pub mod pool;
pub mod ports;
pub mod quality;
pub mod query;
pub mod ratelimit;
pub mod runtime;
pub mod runtime_internal;
pub mod runtime_ontology;
pub mod runtime_operational;
pub mod runtime_read;
pub mod runtime_write;
pub mod schedule;
pub mod secrets;
pub mod telemetry;
pub mod upload;
pub mod upload_mapping;
pub mod workflow;

pub use actions::*;
pub use agents::*;
pub use audit::*;
pub use auth::*;
pub use branch::*;
pub use computed::*;
pub use config::*;
pub use constants::*;
pub use context_engineering::*;
pub use crypto::*;
pub use encrypt::*;
pub use evals::*;
pub use events::*;
pub use federated::*;
pub use interceptors::*;
pub use lineage::*;
pub use pipeline::*;
pub use policy::*;
pub use pool::*;
pub use ports::*;
pub use quality::*;
pub use query::*;
pub use ratelimit::*;
pub use runtime::*;
pub use runtime_internal::{DefaultIdGenerator, SystemClock};
pub use schedule::*;
pub use secrets::*;
pub use telemetry::*;
pub use upload::*;
pub use upload_mapping::*;
pub use workflow::*;
