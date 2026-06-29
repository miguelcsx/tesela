//! Procedural macros for declarative Tesela ontology definitions.
//!
//! Re-exported through the `tesela` facade crate under the `macros` feature.
//! Do not depend on this crate directly; use `tesela` instead.
//!
//! # Derive macros
//!
//! * `ObjectType` — derive an `tesela_ir::ObjectType` from a struct.
//! * `LinkType` — derive an `tesela_ir::LinkType` from a struct.
//! * `TraitDef` — derive an `tesela_ir::Trait` from a struct.
//!
//! # Attribute macros
//!
//! * `action` — derive an `tesela_ir::ActionType` from a free function.
//! * `policy` — derive an `tesela_ir::PolicyRule` from a free function.

mod action_macro;
mod helpers;
mod link_macro;
mod object_type_macro;
mod policy_macro;
mod trait_macro;

use proc_macro::TokenStream;

/// Derive `tesela_object_type()` for a struct.
///
/// Struct-level:
/// `#[tesela(datasource = Datasource::Memory, primary_key = Field::Id, display = "...")]`
/// Field-level:
/// `#[tesela(indexed, unique, nullable, data_type = tesela::DataType::TimestampTz)]`
#[proc_macro_derive(ObjectType, attributes(tesela))]
pub fn derive_object_type(input: TokenStream) -> TokenStream {
    object_type_macro::expand(input)
}

/// Marks a free function as a Tesela `ActionType` and generates a
/// companion struct with a `tesela_action_type()` associated function.
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

/// Derive `tesela_link_type()` for a struct.
///
/// Struct-level: `#[tesela(from = "...", to = "...", cardinality = "one_to_many", display = "...")]`
#[proc_macro_derive(LinkType, attributes(tesela))]
pub fn derive_link_type(input: TokenStream) -> TokenStream {
    link_macro::expand(input)
}

/// Marks a free function as a Tesela `PolicyRule` and generates a
/// companion struct with a `tesela_policy_rule()` associated function.
///
/// # Arguments
///
/// * `effect` — `"allow"` (default) or `"deny"`.
/// * `roles` — comma-separated role names.
/// * `operations` — comma-separated operations: `"read"`, `"mutate"`, etc.
/// * `resource_kind` — optional resource kind filter.
/// * `resource` — optional resource api_name filter.
/// * `description` — rule description.
/// * `priority` — numeric priority.
#[proc_macro_attribute]
pub fn policy(args: TokenStream, input: TokenStream) -> TokenStream {
    policy_macro::expand(args, input)
}

/// Derive `tesela_trait()` for a struct.
///
/// Struct-level: `#[tesela(display = "...")]`
/// Field-level: `#[tesela(description = "...", nullable)]`
#[proc_macro_derive(TraitDef, attributes(tesela))]
pub fn derive_trait_def(input: TokenStream) -> TokenStream {
    trait_macro::expand(input)
}
