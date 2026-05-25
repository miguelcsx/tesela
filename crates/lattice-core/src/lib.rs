//! Core primitives for the Lattice ontology runtime.
//!
//! This crate defines identifiers, errors, diagnostics, domain enums, and value types
//! used across the entire Lattice ecosystem. It contains no business logic — only
//! strongly-typed building blocks.

#![deny(warnings)]
#![deny(missing_docs)]

pub mod diagnostics;
pub mod error;
pub mod ident;
pub mod sync;
pub mod types;
pub mod value;

pub use diagnostics::*;
pub use error::*;
pub use ident::*;
pub use sync::*;
pub use types::*;
pub use value::*;
