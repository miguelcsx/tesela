#![deny(warnings)]
#![deny(missing_docs)]

//! Store contracts for Tesela ontology runtimes.
//!
//! Tesela owns the ontology contract. Platform teams own the concrete stores.

mod memory;
mod query;
mod traits;
mod versioning;

pub use memory::MemoryStore;
pub use query::*;
pub use traits::*;
pub use versioning::*;
