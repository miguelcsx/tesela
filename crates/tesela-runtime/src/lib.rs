#![deny(warnings)]
#![deny(missing_docs)]

//! Ontology runtime for Tesela.
//!
//! The runtime knows about ontology metadata, policy, and store routing. It
//! does not own transports, cloud adapters, agents, workflows, or schedulers.

mod policy;
mod runtime;
#[cfg(test)]
mod runtime_tests;
mod tools;

pub use policy::AllowAllPolicy;
pub use runtime::{OntologyHandle, Runtime, RuntimeOptions};
pub use tools::{
    ActionDescribeArgs, AggregateArgs, AggregateFunctionInput, AggregateInput, EmptyArgs, GetArgs,
    ObjectSetComposeArgs, ObjectSetComposeOp, ObjectSetResolveArgs, OntologyTool,
    OntologyToolDefinition, SearchArgs, ToolApprovalPolicy, ToolSideEffect, TraverseArgs,
    ontology_tool_definitions,
};
