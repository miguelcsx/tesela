#![deny(warnings)]
#![deny(missing_docs)]

//! Canonical intermediate representation for Tesela ontologies.

mod action;
mod data;
mod declarative;
mod link;
mod object_set;
mod object_type;
mod policy;
mod spec;

pub use action::*;
pub use data::*;
pub use declarative::*;
pub use link::*;
pub use object_set::*;
pub use object_type::*;
pub use policy::*;
pub use spec::*;
