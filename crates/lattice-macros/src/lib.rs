//! Procedural macros for declarative Lattice ontology definitions.
//!
//! Re-exported through the `lattice` facade crate under the `macros` feature.
//! Do not depend on this crate directly; use `lattice` instead.
//!
//! # Derive macros
//!
//! * `ObjectType` — derive an `lattice_ir::ObjectType` from a struct.
//! * `Agent` — derive an `lattice_ir::Agent` from a struct.
//!
//! # Attribute macros
//!
//! * `action` — derive an `lattice_ir::ActionType` from a free function.

mod action_macro;
mod agent_macro;
mod helpers;
mod object_type_macro;

use proc_macro::TokenStream;

/// Derive `lattice_object_type()` for a struct.
///
/// Struct-level: `#[lattice(datasource = "memory", primary_key = "id", display = "...")]`
/// Field-level: `#[lattice(indexed, unique, nullable, description = "...", source_column = "...", encrypted)]`
#[proc_macro_derive(ObjectType, attributes(lattice))]
pub fn derive_object_type(input: TokenStream) -> TokenStream {
    object_type_macro::expand(input)
}

/// Marks a free function as a Lattice `ActionType` and generates a
/// companion struct with a `lattice_action_type()` associated function.
///
/// # Arguments
///
/// * `risk` — risk level: `"low"` (default), `"medium"`, or `"high"`.
/// * `handler` — handler kind (default: `"callback"`).
/// * `display` — human-readable label (default: function name).
/// * `description` — action description.
#[proc_macro_attribute]
pub fn action(args: TokenStream, input: TokenStream) -> TokenStream {
    action_macro::expand(args, input)
}

/// Derive `lattice_agent()` for a struct.
///
/// Struct-level: `#[lattice(model = "...", display = "...")]`
#[proc_macro_derive(Agent, attributes(lattice))]
pub fn derive_agent(input: TokenStream) -> TokenStream {
    agent_macro::expand(input)
}
