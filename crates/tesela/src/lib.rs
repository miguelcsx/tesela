#![deny(warnings)]
#![deny(missing_docs)]

//! Tesela facade.
//!
//! Tesela is a library for ontology-driven platforms. It provides primitives,
//! ontology IR, store contracts, and a runtime handle. Protocols, cloud
//! adapters, SDKs, agents, and application services are owned by the platform.

pub use tesela_core as core;
pub use tesela_ir as ir;
pub use tesela_runtime as runtime;
pub use tesela_store as store;

#[cfg(feature = "macros")]
pub use tesela_macros::{ObjectType, TraitDef, action, policy};

/// Common imports for platform code.
pub mod prelude {
    pub use tesela_core::{
        ApiName, ApiNameSource, DataType, Error, FilterOp, LinkCardinality, Operation, Value,
    };
    pub use tesela_ir::{
        ActionResult, ActionType, AggregateResult, Datasource, Filter, LinkMapping, LinkSource,
        LinkType, MutationResult, ObjectSet, ObjectSource, ObjectType, ObjectTypeDefinition, Page,
        Property, Record, Role, SetOp, Spec, StaticIndex, StaticObjectType, StaticProperty,
    };
    pub use tesela_runtime::{
        ActionDescribeArgs, AggregateArgs, AggregateFunctionInput, AggregateInput, AllowAllPolicy,
        EmptyArgs, GetArgs, ObjectSetComposeArgs, ObjectSetComposeOp, ObjectSetResolveArgs,
        OntologyHandle, OntologyTool, OntologyToolDefinition, Runtime, RuntimeOptions, SearchArgs,
        ToolApprovalPolicy, ToolSideEffect as TeselaToolSideEffect, TraverseArgs,
        ontology_tool_definitions,
    };
    pub use tesela_store::{
        Actor, AggregateQuery, Aggregation, AggregationFunction, AuditEvent, AuditSink, EventBus,
        MemoryStore, Mutation, OntologyEvent, OntologyStore, PolicyDecision, PolicyEngine,
        PolicyRequest, Query, SnapshotDefault, Sort, SortDirection, StaticStoreRouter,
        StoreCapabilities, StoreRouter, TraversalQuery, VersionedRecordSchema, next_version,
        restore_values, snapshot_record,
    };
}

pub use prelude::*;
