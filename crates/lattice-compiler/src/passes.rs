//! Validation passes for the compiler pipeline.

use crate::compiler::Pass;
use crate::hash::HasApiName;
use lattice_core::{ApiName, DataType, Diagnostic, DiagnosticCode, LinkCardinality, Operation};
use lattice_graph::SchemaGraph;
use lattice_ir::{ObjectType, Spec};
use std::collections::{HashMap, HashSet};

/// Validate API names: regex `^[a-z][a-z0-9_]*$` and uniqueness within each collection.
pub struct NameValidationPass;

impl Pass for NameValidationPass {
    fn name(&self) -> &'static str {
        "name_validation"
    }

    fn run(&self, spec: &Spec, _graph: &SchemaGraph) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let re = regex::Regex::new(ApiName::PATTERN).expect("ApiName::PATTERN is a valid regex");

        fn check_uniqueness<T: HasApiName>(
            items: &[T],
            kind: &str,
            re: &regex::Regex,
            diags: &mut Vec<Diagnostic>,
        ) {
            let mut seen = HashSet::new();
            for item in items {
                let name = item.api_name();
                if !re.is_match(name) {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidName,
                            format!(
                                "{} name '{}' does not match pattern {}",
                                kind,
                                name,
                                ApiName::PATTERN
                            ),
                        )
                        .with_api_name(name.clone()),
                    );
                }
                if !seen.insert(name.clone()) {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidName,
                            format!("duplicate {} name '{}'", kind, name),
                        )
                        .with_api_name(name.clone()),
                    );
                }
            }
        }

        check_uniqueness(&spec.datasources, "datasource", &re, &mut diags);
        check_uniqueness(&spec.traits, "trait", &re, &mut diags);
        check_uniqueness(&spec.object_types, "object_type", &re, &mut diags);
        check_uniqueness(&spec.link_types, "link_type", &re, &mut diags);
        check_uniqueness(&spec.actions, "action", &re, &mut diags);
        check_uniqueness(&spec.roles, "role", &re, &mut diags);
        check_uniqueness(&spec.policies, "policy", &re, &mut diags);
        check_uniqueness(&spec.agents, "agent", &re, &mut diags);
        check_uniqueness(&spec.custom_tools, "custom_tool", &re, &mut diags);
        check_uniqueness(&spec.assets, "asset", &re, &mut diags);
        check_uniqueness(&spec.environments, "environment", &re, &mut diags);
        check_uniqueness(&spec.artifact_types, "artifact_type", &re, &mut diags);
        check_uniqueness(&spec.upload_flows, "upload_flow", &re, &mut diags);
        check_uniqueness(&spec.job_types, "job_type", &re, &mut diags);
        check_uniqueness(&spec.event_types, "event_type", &re, &mut diags);
        check_uniqueness(&spec.capability_grants, "capability_grant", &re, &mut diags);
        check_uniqueness(&spec.aggregate_views, "aggregate_view", &re, &mut diags);

        if !re.is_match(&spec.workspace.api_name) {
            diags.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidName,
                    format!(
                        "workspace name '{}' does not match pattern {}",
                        spec.workspace.api_name,
                        ApiName::PATTERN
                    ),
                )
                .with_api_name(spec.workspace.api_name.clone()),
            );
        }

        diags
    }
}

/// Validate cross-references: datasources exist, PKs valid, link endpoints exist, etc.
pub struct ReferenceValidationPass;

impl Pass for ReferenceValidationPass {
    fn name(&self) -> &'static str {
        "reference_validation"
    }

    fn run(&self, spec: &Spec, graph: &SchemaGraph) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let ds_names: HashSet<ApiName> = spec
            .datasources
            .iter()
            .map(|d| d.api_name.clone())
            .collect();
        let ot_names: HashSet<ApiName> = spec
            .object_types
            .iter()
            .map(|o| o.api_name.clone())
            .collect();
        let event_names: HashSet<ApiName> = spec
            .event_types
            .iter()
            .map(|e| e.api_name.clone())
            .collect();
        let role_names: HashSet<ApiName> = spec.roles.iter().map(|r| r.api_name.clone()).collect();

        for ot in &spec.object_types {
            if !ds_names.contains(&ot.source.datasource) {
                diags.push(
                    Diagnostic::error(
                        DiagnosticCode::BrokenReference,
                        format!(
                            "object_type '{}' references unknown datasource '{}'",
                            ot.api_name, ot.source.datasource
                        ),
                    )
                    .with_api_name(ot.api_name.clone()),
                );
            }

            let prop_names: HashSet<ApiName> =
                ot.properties.iter().map(|p| p.api_name.clone()).collect();
            if !prop_names.contains(&ot.primary_key) {
                diags.push(
                    Diagnostic::error(
                        DiagnosticCode::BrokenReference,
                        format!(
                            "object_type '{}' primary_key '{}' is not a defined property",
                            ot.api_name, ot.primary_key
                        ),
                    )
                    .with_api_name(ot.api_name.clone()),
                );
            }
        }

        for lt in &spec.link_types {
            if !ot_names.contains(&lt.from) {
                diags.push(
                    Diagnostic::error(
                        DiagnosticCode::BrokenReference,
                        format!(
                            "link_type '{}' references unknown source object_type '{}'",
                            lt.api_name, lt.from
                        ),
                    )
                    .with_api_name(lt.api_name.clone()),
                );
            }
            if !ot_names.contains(&lt.to) {
                diags.push(
                    Diagnostic::error(
                        DiagnosticCode::BrokenReference,
                        format!(
                            "link_type '{}' references unknown target object_type '{}'",
                            lt.api_name, lt.to
                        ),
                    )
                    .with_api_name(lt.api_name.clone()),
                );
            }

            if let (Some(from_ot), Some(to_ot)) = (
                find_object_type(spec, &lt.from),
                find_object_type(spec, &lt.to),
            ) {
                let from_props: HashSet<ApiName> = from_ot
                    .properties
                    .iter()
                    .map(|p| p.api_name.clone())
                    .collect();
                let to_props: HashSet<ApiName> = to_ot
                    .properties
                    .iter()
                    .map(|p| p.api_name.clone())
                    .collect();
                for mapping in &lt.mappings {
                    if !from_props.contains(&mapping.from_property) {
                        diags.push(
                            Diagnostic::error(
                                DiagnosticCode::BrokenReference,
                                format!(
                                    "link_type '{}' mapping references unknown property '{}' on '{}'",
                                    lt.api_name, mapping.from_property, lt.from
                                ),
                            )
                            .with_api_name(lt.api_name.clone()),
                        );
                    }
                    if !to_props.contains(&mapping.to_property) {
                        diags.push(
                            Diagnostic::error(
                                DiagnosticCode::BrokenReference,
                                format!(
                                    "link_type '{}' mapping references unknown property '{}' on '{}'",
                                    lt.api_name, mapping.to_property, lt.to
                                ),
                            )
                            .with_api_name(lt.api_name.clone()),
                        );
                    }
                }
            }
        }

        let cycle_diags = graph.detect_cycles();
        diags.extend(cycle_diags);

        for role in &spec.roles {
            for parent in &role.inherits {
                if !role_names.contains(parent) {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::BrokenReference,
                            format!(
                                "role '{}' inherits from unknown role '{}'",
                                role.api_name, parent
                            ),
                        )
                        .with_api_name(role.api_name.clone()),
                    );
                }
            }
        }

        diags.extend(detect_role_cycles(spec));

        for upload in &spec.upload_flows {
            if let Some(target) = &upload.target_object_type {
                if !ot_names.contains(target) {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::BrokenReference,
                            format!(
                                "upload_flow '{}' references unknown target object_type '{}'",
                                upload.api_name, target
                            ),
                        )
                        .with_api_name(upload.api_name.clone()),
                    );
                } else if let Some(ot) = find_object_type(spec, target) {
                    let prop_names: HashSet<ApiName> =
                        ot.properties.iter().map(|p| p.api_name.clone()).collect();
                    for mapping in &upload.mappings {
                        if !prop_names.contains(&mapping.target_property) {
                            diags.push(
                                Diagnostic::error(
                                    DiagnosticCode::BrokenReference,
                                    format!(
                                        "upload_flow '{}' mapping targets unknown property '{}' on '{}'",
                                        upload.api_name, mapping.target_property, target
                                    ),
                                )
                                .with_api_name(upload.api_name.clone()),
                            );
                        }
                    }
                }
            }
            validate_template(
                &upload.api_name,
                "upload_flow",
                &upload.path_template,
                &mut diags,
            );
        }

        for job in &spec.job_types {
            if let Some(event_name) = &job.start_event {
                if !event_names.contains(event_name) {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::BrokenReference,
                            format!(
                                "job_type '{}' references unknown start_event '{}'",
                                job.api_name, event_name
                            ),
                        )
                        .with_api_name(job.api_name.clone()),
                    );
                }
            }
            if let Some(event_name) = &job.result_event {
                if !event_names.contains(event_name) {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::BrokenReference,
                            format!(
                                "job_type '{}' references unknown result_event '{}'",
                                job.api_name, event_name
                            ),
                        )
                        .with_api_name(job.api_name.clone()),
                    );
                }
            }
        }

        for view in &spec.aggregate_views {
            if !ot_names.contains(&view.object_type) {
                diags.push(
                    Diagnostic::error(
                        DiagnosticCode::BrokenReference,
                        format!(
                            "aggregate_view '{}' references unknown object_type '{}'",
                            view.api_name, view.object_type
                        ),
                    )
                    .with_api_name(view.api_name.clone()),
                );
            } else if let Some(ot) = find_object_type(spec, &view.object_type) {
                let prop_names: HashSet<ApiName> =
                    ot.properties.iter().map(|p| p.api_name.clone()).collect();
                for group in &view.group_by {
                    if !prop_names.contains(group) {
                        diags.push(
                            Diagnostic::error(
                                DiagnosticCode::BrokenReference,
                                format!(
                                    "aggregate_view '{}' groups by unknown property '{}' on '{}'",
                                    view.api_name, group, view.object_type
                                ),
                            )
                            .with_api_name(view.api_name.clone()),
                        );
                    }
                }
                for measure in &view.measures {
                    if let Some(property) = &measure.property {
                        if !prop_names.contains(property) {
                            diags.push(
                                Diagnostic::error(
                                    DiagnosticCode::BrokenReference,
                                    format!(
                                        "aggregate_view '{}' measures unknown property '{}' on '{}'",
                                        view.api_name, property, view.object_type
                                    ),
                                )
                                .with_api_name(view.api_name.clone()),
                            );
                        }
                    }
                }
                if let Some(bucket) = &view.time_bucket {
                    if !prop_names.contains(&bucket.property) {
                        diags.push(
                            Diagnostic::error(
                                DiagnosticCode::BrokenReference,
                                format!(
                                    "aggregate_view '{}' time_bucket references unknown property '{}' on '{}'",
                                    view.api_name, bucket.property, view.object_type
                                ),
                            )
                            .with_api_name(view.api_name.clone()),
                        );
                    }
                }
                if let Some(extent) = &view.spatial_extent {
                    if !prop_names.contains(&extent.property) {
                        diags.push(
                            Diagnostic::error(
                                DiagnosticCode::BrokenReference,
                                format!(
                                    "aggregate_view '{}' spatial_extent references unknown property '{}' on '{}'",
                                    view.api_name, extent.property, view.object_type
                                ),
                            )
                            .with_api_name(view.api_name.clone()),
                        );
                    }
                }
            }
        }

        for artifact in &spec.artifact_types {
            validate_template(
                &artifact.api_name,
                "artifact_type",
                &artifact.path_template,
                &mut diags,
            );
        }

        for grant in &spec.capability_grants {
            if let Some(resource) = &grant.resource {
                let ok = match grant.resource_kind.as_str() {
                    "object_type" => ot_names.contains(resource),
                    "link_type" => spec.link_types.iter().any(|l| &l.api_name == resource),
                    "action" => spec.actions.iter().any(|a| &a.api_name == resource),
                    "agent" => spec.agents.iter().any(|a| &a.api_name == resource),
                    "artifact_type" => spec.artifact_types.iter().any(|a| &a.api_name == resource),
                    "upload_flow" => spec.upload_flows.iter().any(|u| &u.api_name == resource),
                    "job_type" => spec.job_types.iter().any(|j| &j.api_name == resource),
                    "event_type" => spec.event_types.iter().any(|e| &e.api_name == resource),
                    "aggregate_view" => {
                        spec.aggregate_views.iter().any(|a| &a.api_name == resource)
                    }
                    _ => true,
                };
                if !ok {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::BrokenReference,
                            format!(
                                "capability_grant '{}' references unknown {} '{}'",
                                grant.api_name, grant.resource_kind, resource
                            ),
                        )
                        .with_api_name(grant.api_name.clone()),
                    );
                }
            }
        }

        diags
    }
}

fn validate_template(api_name: &ApiName, kind: &str, template: &str, diags: &mut Vec<Diagnostic>) {
    let mut depth = 0i32;
    for ch in template.chars() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            break;
        }
    }
    if depth != 0 {
        diags.push(
            Diagnostic::error(
                DiagnosticCode::InvalidProperty,
                format!(
                    "{} '{}' has unbalanced path template braces",
                    kind, api_name
                ),
            )
            .with_api_name(api_name.clone()),
        );
    }
}

/// Validate properties: no duplicates, allowed-values semantics, computed expression language.
pub struct PropertyValidationPass;

impl Pass for PropertyValidationPass {
    fn name(&self) -> &'static str {
        "property_validation"
    }

    fn run(&self, spec: &Spec, _graph: &SchemaGraph) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        for tr in &spec.traits {
            let mut seen = HashSet::new();
            for prop in &tr.properties {
                if !seen.insert(prop.api_name.clone()) {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidProperty,
                            format!(
                                "trait '{}' has duplicate property '{}'",
                                tr.api_name, prop.api_name
                            ),
                        )
                        .with_api_name(tr.api_name.clone()),
                    );
                }
            }
        }

        for ot in &spec.object_types {
            let mut seen = HashSet::new();
            for prop in &ot.properties {
                if !seen.insert(prop.api_name.clone()) {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidProperty,
                            format!(
                                "object_type '{}' has duplicate property '{}'",
                                ot.api_name, prop.api_name
                            ),
                        )
                        .with_api_name(ot.api_name.clone()),
                    );
                }

                if let Some(ref vals) = prop.allowed_values {
                    if !vals.is_empty()
                        && prop.data_type != DataType::Enum
                        && prop.data_type != DataType::String
                    {
                        diags.push(
                            Diagnostic::warning(
                                DiagnosticCode::InvalidProperty,
                                format!(
                                    "property '{}' on '{}' has allowed_values but data_type is {:?}",
                                    prop.api_name, ot.api_name, prop.data_type
                                ),
                            )
                            .with_api_name(ot.api_name.clone()),
                        );
                    }
                }

                if let Some(ref computed) = prop.computed {
                    if computed.language.is_empty() {
                        diags.push(
                            Diagnostic::error(
                                DiagnosticCode::InvalidProperty,
                                format!(
                                    "computed property '{}' on '{}' has empty language",
                                    prop.api_name, ot.api_name
                                ),
                            )
                            .with_api_name(ot.api_name.clone()),
                        );
                    }
                }
            }
        }

        diags
    }
}

/// Validate policy rules: role refs exist, operations valid, resource kinds valid.
pub struct PolicyValidationPass;

impl Pass for PolicyValidationPass {
    fn name(&self) -> &'static str {
        "policy_validation"
    }

    fn run(&self, spec: &Spec, _graph: &SchemaGraph) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let role_names: HashSet<String> =
            spec.roles.iter().map(|r| r.api_name.to_string()).collect();
        let valid_resource_kinds: HashSet<&str> = [
            "object_type",
            "action",
            "link_type",
            "agent",
            "custom_tool",
            "asset",
            "artifact_type",
            "upload_flow",
            "job_type",
            "event_type",
            "capability_grant",
            "aggregate_view",
        ]
        .iter()
        .copied()
        .collect();

        for policy in &spec.policies {
            for role in &policy.roles {
                if !role_names.contains(role) {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidPolicy,
                            format!(
                                "policy '{}' references unknown role '{}'",
                                policy.api_name, role
                            ),
                        )
                        .with_api_name(policy.api_name.clone()),
                    );
                }
            }

            for op in &policy.operations {
                if !matches!(
                    op,
                    Operation::Search
                        | Operation::Read
                        | Operation::Mutate
                        | Operation::Traverse
                        | Operation::Aggregate
                        | Operation::Upload
                        | Operation::Execute
                ) {
                    diags.push(
                        Diagnostic::warning(
                            DiagnosticCode::InvalidPolicy,
                            format!(
                                "policy '{}' references unknown operation variant",
                                policy.api_name
                            ),
                        )
                        .with_api_name(policy.api_name.clone()),
                    );
                }
            }

            if let Some(ref kind) = policy.resource_kind {
                if !valid_resource_kinds.contains(kind.as_str()) {
                    diags.push(
                        Diagnostic::warning(
                            DiagnosticCode::InvalidPolicy,
                            format!(
                                "policy '{}' has unknown resource_kind '{}'",
                                policy.api_name, kind
                            ),
                        )
                        .with_api_name(policy.api_name.clone()),
                    );
                }
            }
        }

        diags
    }
}

/// Validate link types: cardinality rules, many-to-many requires junction.
pub struct LinkValidationPass;

impl Pass for LinkValidationPass {
    fn name(&self) -> &'static str {
        "link_validation"
    }

    fn run(&self, spec: &Spec, _graph: &SchemaGraph) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        for lt in &spec.link_types {
            match lt.cardinality {
                LinkCardinality::OneToOne => {
                    if lt.mappings.len() != 1 {
                        diags.push(
                            Diagnostic::warning(
                                DiagnosticCode::InvalidLink,
                                format!(
                                    "one_to_one link '{}' should have exactly 1 mapping, found {}",
                                    lt.api_name,
                                    lt.mappings.len()
                                ),
                            )
                            .with_api_name(lt.api_name.clone()),
                        );
                    }
                }
                LinkCardinality::OneToMany => {
                    if lt.mappings.is_empty() {
                        diags.push(
                            Diagnostic::error(
                                DiagnosticCode::InvalidLink,
                                format!(
                                    "one_to_many link '{}' must have at least 1 mapping",
                                    lt.api_name
                                ),
                            )
                            .with_api_name(lt.api_name.clone()),
                        );
                    }
                }
                LinkCardinality::ManyToMany => {
                    if lt.junction.is_none() {
                        diags.push(
                            Diagnostic::error(
                                DiagnosticCode::InvalidLink,
                                format!(
                                    "many_to_many link '{}' requires a junction configuration",
                                    lt.api_name
                                ),
                            )
                            .with_api_name(lt.api_name.clone()),
                        );
                    }
                    if lt.mappings.is_empty() {
                        diags.push(
                            Diagnostic::error(
                                DiagnosticCode::InvalidLink,
                                format!(
                                    "many_to_many link '{}' must have at least 1 mapping",
                                    lt.api_name
                                ),
                            )
                            .with_api_name(lt.api_name.clone()),
                        );
                    }
                }
            }
        }

        diags
    }
}

/// Normalize a spec: deterministic sort of all collections by `api_name`.
pub struct NormalizationPass;

impl Pass for NormalizationPass {
    fn name(&self) -> &'static str {
        "normalization"
    }

    fn run(&self, _spec: &Spec, _graph: &SchemaGraph) -> Vec<Diagnostic> {
        Vec::new()
    }
}

fn find_object_type<'a>(spec: &'a Spec, name: &ApiName) -> Option<&'a ObjectType> {
    spec.object_types.iter().find(|o| &o.api_name == name)
}

fn detect_role_cycles(spec: &Spec) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut adj: HashMap<ApiName, Vec<ApiName>> = HashMap::new();
    for role in &spec.roles {
        for parent in &role.inherits {
            adj.entry(role.api_name.clone())
                .or_default()
                .push(parent.clone());
        }
    }

    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    fn dfs(
        node: &ApiName,
        adj: &HashMap<ApiName, Vec<ApiName>>,
        visited: &mut HashSet<ApiName>,
        rec_stack: &mut HashSet<ApiName>,
        diags: &mut Vec<Diagnostic>,
    ) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());

        if let Some(parents) = adj.get(node) {
            for parent in parents {
                if !visited.contains(parent) {
                    dfs(parent, adj, visited, rec_stack, diags);
                } else if rec_stack.contains(parent) {
                    diags.push(
                        Diagnostic::error(
                            DiagnosticCode::BrokenReference,
                            format!(
                                "role inheritance cycle involving '{}' and '{}'",
                                node, parent
                            ),
                        )
                        .with_api_name(node.clone()),
                    );
                }
            }
        }

        rec_stack.remove(node);
    }

    for role in &spec.roles {
        if !visited.contains(&role.api_name) {
            dfs(
                &role.api_name,
                &adj,
                &mut visited,
                &mut rec_stack,
                &mut diags,
            );
        }
    }

    diags
}
