//! Canonical intermediate representation (IR) for Lattice ontologies.
//!
//! This crate defines the language-neutral JSON spec format (`lattice.spec.v1`).
//! Every builder SDK compiles to this structure.
//! The runtime consumes only this representation.

#![deny(warnings)]
#![deny(missing_docs)]

mod action;
mod agent;
mod asset;
mod data;
mod link;
mod migration;
mod object_set;
mod object_type;
mod operational;
mod pipeline;
mod policy;
mod spec;

pub use action::*;
pub use agent::*;
pub use asset::*;
pub use data::*;
pub use link::*;
pub use migration::*;
pub use object_set::*;
pub use object_type::*;
pub use operational::*;
pub use pipeline::*;
pub use policy::*;
pub use spec::*;
