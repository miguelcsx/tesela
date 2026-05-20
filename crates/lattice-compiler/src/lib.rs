//! Compiler pipeline for Lattice ontologies.
//!
//! Provides validation passes, normalization, diff computation, and hashing.

#![deny(warnings)]
#![deny(missing_docs)]

mod compiler;
mod diff;
mod hash;
mod passes;

pub use compiler::*;
pub use diff::*;
pub use hash::*;
pub use passes::*;

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::ApiName;
    use lattice_ir::{
        ActionHandler, ActionType, Datasource, ObjectSource, ObjectType, Property, Spec, Workspace,
    };

    fn make_spec() -> Spec {
        Spec {
            version: lattice_core::Version::new(lattice_ir::SPEC_VERSION),
            workspace: Workspace::default(),
            datasources: vec![Datasource {
                api_name: ApiName::new_unchecked("db"),
                adapter_type: "memory".to_string(),
                config: None,
                secrets: None,
            }],
            traits: Vec::new(),
            object_types: vec![ObjectType {
                api_name: ApiName::new_unchecked("customer"),
                display: None,
                description: None,
                source: ObjectSource {
                    datasource: ApiName::new_unchecked("db"),
                    resource: None,
                },
                primary_key: ApiName::new_unchecked("id"),
                properties: vec![Property {
                    api_name: ApiName::new_unchecked("id"),
                    display: None,
                    description: None,
                    data_type: lattice_core::DataType::Uuid,
                    nullable: None,
                    indexed: None,
                    unique: None,
                    tags: Vec::new(),
                    markings: Vec::new(),
                    default: None,
                    computed: None,
                    source_column: None,
                    allowed_values: None,
                    sort_order: None,
                    metadata: None,
                    encrypted: None,
                    quality: Vec::new(),
                }],
                traits: Vec::new(),
                tags: Vec::new(),
                metadata: None,
                indexes: Vec::new(),
                temporal: None,
                lifecycle: None,
                scoring: None,
                classification: None,
                quality_rules: Vec::new(),
                lineage: Vec::new(),
                deprecated_at: None,
            }],
            link_types: Vec::new(),
            actions: Vec::new(),
            roles: Vec::new(),
            policies: Vec::new(),
            agents: Vec::new(),
            custom_tools: Vec::new(),
            assets: Vec::new(),
            environments: Vec::new(),
            object_sets: Vec::new(),
            pipelines: Vec::new(),
            artifact_types: Vec::new(),
            upload_flows: Vec::new(),
            job_types: Vec::new(),
            event_types: Vec::new(),
            capability_grants: Vec::new(),
            aggregate_views: Vec::new(),
        }
    }

    #[test]
    fn test_name_validation() {
        let spec = make_spec();
        let compiler = Compiler::default_pipeline();
        let result = compiler.compile(&spec);
        assert!(result.is_valid, "expected valid: {:?}", result.diagnostics);
    }

    #[test]
    fn test_reference_validation_missing_datasource() {
        let mut spec = make_spec();
        spec.datasources.clear();
        let compiler = Compiler::default_pipeline();
        let result = compiler.compile(&spec);
        assert!(!result.is_valid);
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("unknown datasource")));
    }

    #[test]
    fn test_diff_basic() {
        let old = make_spec();
        let mut new = make_spec();
        new.actions.push(ActionType {
            api_name: ApiName::new_unchecked("create_customer"),
            display: None,
            description: None,
            subject: Some(ApiName::new_unchecked("customer")),
            handler: ActionHandler {
                kind: "crud".to_string(),
                target: None,
                config: None,
            },
            input_schema: None,
            output_schema: None,
            mode: None,
            risk_level: None,
            idempotency_key: None,
            deprecated_at: None,
            metadata: None,
        });
        let diff = compute_diff(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(
            diff.added[0].api_name,
            ApiName::new_unchecked("create_customer")
        );
    }

    #[test]
    fn test_hash_deterministic() {
        let spec = make_spec();
        let h1 = hash_spec(&spec);
        let h2 = hash_spec(&spec);
        assert_eq!(h1, h2);
    }
}
