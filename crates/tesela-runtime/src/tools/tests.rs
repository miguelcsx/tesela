use super::OntologyTool;

#[test]
fn ontology_tool_names_round_trip_through_constant_time_lookup() {
    for tool in OntologyTool::ALL.iter().copied() {
        assert_eq!(OntologyTool::from_name(tool.name()), Some(tool));
    }
}

#[test]
fn unknown_ontology_tool_name_is_rejected() {
    assert_eq!(OntologyTool::from_name("tesela.unknown"), None);
}

#[test]
fn ontology_tool_schemas_declare_no_host_scope() {
    // tesela is host-agnostic: scope vocabulary belongs to the embedding
    // application, which adds its own parameters to these schemas.
    for tool in OntologyTool::ALL.iter().copied() {
        let schema = tool.input_schema().expect("tool schema");
        let Some(properties) = schema["properties"].as_object() else {
            continue;
        };
        for reserved in ["scenario_id", "world_id", "locale_id", "tenant_id"] {
            assert!(
                !properties.contains_key(reserved),
                "{} must not declare host scope {reserved}",
                tool.name()
            );
            assert!(!tool.prompt_hint().contains(reserved));
        }
    }
}
