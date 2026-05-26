//! Fluent builder types for ontology elements.

mod prelude {
    pub(super) use tesela_core::{
        ApiName, DataType, LinkCardinality, Operation, PolicyEffect, Value,
    };
    pub(super) use tesela_ir::{
        ActionHandler, ActionType, Agent, AgentLimits, AgentMemory, AggregateFunction,
        AggregateMeasure, AggregateView, ArtifactType, CapabilityGrant, ClassificationConfig,
        Computed, ContextSource, Datasource, EventType, Index, JobType, JunctionConfig,
        LifecycleConfig, LinkMapping, LinkSource, LinkType, ObjectSource, ObjectType, PolicyRule,
        Property, QualityRule, QualityRuleRef, ScoringConfig, SpatialExtent, TemporalConfig,
        TimeBucket, UploadFlow,
    };
}

mod action;
mod agent;
mod datasource;
mod link;
mod object;
mod operations;
mod policy;
mod property;
mod role;

pub use action::ActionBuilder;
pub use agent::AgentBuilder;
pub use datasource::DatasourceBuilder;
pub use link::LinkBuilder;
pub use object::ObjectTypeBuilder;
pub use operations::{
    aggregate_view, capability_grant, event_type, measure, spatial_extent, time_bucket,
    ArtifactTypeBuilder, JobTypeBuilder, UploadFlowBuilder,
};
pub use policy::PolicyBuilder;
pub use property::PropertyBuilder;
pub use role::RoleBuilder;
