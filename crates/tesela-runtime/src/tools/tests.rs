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
