#![deny(warnings)]
#![deny(missing_docs)]

//! Fluent builder API for defining Tesela ontologies in code.
//!
//! The primary entry point is [`App`], which accepts chained builder calls to
//! define object types, link types, actions, roles, policies, agents, and
//! more. When the definition is complete, call [`App::compile`] to run the
//! full compiler pipeline, or [`App::into_runtime`] to produce a live
//! `Runtime` (requires the `native-runtime` feature).
//!
//! # Example
//!
//! ```rust
//! use tesela_sdk::{App, DatasourceBuilder, ObjectTypeBuilder, PropertyBuilder};
//! use tesela_core::DataType;
//!
//! let result = App::new("my_workspace")
//!     .datasource(DatasourceBuilder::new("memory", "memory").build())
//!     .object_type(
//!         ObjectTypeBuilder::new("user")
//!             .display("User")
//!             .property(PropertyBuilder::new("id", DataType::String).required(true).build())
//!             .property(PropertyBuilder::new("name", DataType::String).build())
//!             .datasource("memory")
//!             .build()
//!     )
//!     .compile();
//!
//! assert!(result.is_valid);
//! ```

mod app;
mod builders;

pub use app::App;
pub use builders::{
    ActionBuilder, AgentBuilder, ArtifactTypeBuilder, DatasourceBuilder, JobTypeBuilder,
    LinkBuilder, ObjectTypeBuilder, PolicyBuilder, PropertyBuilder, RoleBuilder, UploadFlowBuilder,
    aggregate_view, capability_grant, event_type, measure, spatial_extent, time_bucket,
};

pub use tesela_compiler::{CompileResult, Compiler};
pub use tesela_core::{ApiName, DataType, Error, LinkCardinality, Operation, PolicyEffect, Value};
pub use tesela_ir::{
    ActionHandler, ActionType, Agent, AgentLimits, AgentMemory, AggregateFunction,
    AggregateMeasure, AggregateView, ArtifactType, Asset, AssetSink, CapabilityGrant,
    ContextSource, CustomTool, Datasource, Environment, EventType, Filter, JobType, LinkMapping,
    LinkSource, LinkType, ObjectSource, ObjectType, Obligation, PolicyRule, Property, Role, Spec,
    Trait, UploadFlow, Workspace,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_basic_compile() {
        let result = App::new("test_ws")
            .datasource(DatasourceBuilder::new("memory", "memory").build())
            .object_type(
                ObjectTypeBuilder::new("user")
                    .display("User")
                    .property(
                        PropertyBuilder::new("id", DataType::String)
                            .required(true)
                            .build(),
                    )
                    .property(PropertyBuilder::new("name", DataType::String).build())
                    .build(),
            )
            .compile();

        assert!(result.is_valid, "diagnostics: {:?}", result.diagnostics);
    }

    #[test]
    fn test_link_builder() {
        let link = LinkBuilder::new("user_orders", "user", "order")
            .display("User Orders")
            .cardinality(LinkCardinality::OneToMany)
            .mapping("id", "user_id")
            .build();
        assert_eq!(link.api_name.as_ref(), "user_orders");
        assert_eq!(link.mappings.len(), 1);
    }

    #[test]
    fn test_action_builder() {
        let action = ActionBuilder::new("create_user")
            .description("Create a new user.")
            .risk_level("low")
            .build();
        assert_eq!(action.api_name.as_ref(), "create_user");
        assert_eq!(action.risk_level.as_deref(), Some("low"));
    }

    #[test]
    fn test_agent_builder() {
        let agent = AgentBuilder::new("my_agent")
            .model("claude-sonnet-4-6")
            .instructions("You are helpful.")
            .allow_tool("search_user")
            .build();
        assert_eq!(agent.allowed_tools.len(), 1);
    }

    #[test]
    fn test_compile_json() {
        let json = App::new("ws")
            .datasource(DatasourceBuilder::new("memory", "memory").build())
            .object_type(
                ObjectTypeBuilder::new("item")
                    .property(
                        PropertyBuilder::new("id", DataType::String)
                            .required(true)
                            .build(),
                    )
                    .build(),
            )
            .compile_json();
        assert!(json.is_ok());
        let s = json.unwrap();
        assert!(s.contains("item"));
    }

    #[test]
    fn test_policy_builder() {
        let policy = PolicyBuilder::new("allow_read", PolicyEffect::Allow)
            .role("viewer")
            .operation(Operation::Read)
            .priority(100)
            .build();
        assert_eq!(policy.effect, PolicyEffect::Allow);
        assert_eq!(policy.priority, Some(100));
    }
}
