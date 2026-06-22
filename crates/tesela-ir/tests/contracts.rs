use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};
use tesela_ir::{
    ActionProposal, ActionRun, AgentDefinition, AgentRun, DecisionRecord, DomainEvent,
    EvidenceNode, FunctionDefinition, FunctionRun, LayerArtifact, LayerDefinition, LayerInstance,
    LayerRun, LineageEdge, Spec, ToolCall, WorkflowDefinition, WorkflowRun, WorkflowStep,
    WorkflowStepKind, WorkflowStepRun, WorkflowTrigger,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ContractBundle {
    action_proposal: ActionProposal,
    action_run: ActionRun,
    layer_definition: LayerDefinition,
    layer_instance: LayerInstance,
    layer_run: LayerRun,
    function_definition: FunctionDefinition,
    function_run: FunctionRun,
    workflow_definition: WorkflowDefinition,
    workflow_run: WorkflowRun,
    agent_definition: AgentDefinition,
    agent_run: AgentRun,
    tool_call: ToolCall,
    evidence_node: EvidenceNode,
    decision_record: DecisionRecord,
    lineage_edge: LineageEdge,
    domain_event: DomainEvent,
}

fn api(name: &str) -> ApiName {
    ApiName::new_unchecked(name)
}

fn object_value(key: &str, value: &str) -> Value {
    Value::new(serde_json::json!({ key: value }))
}

fn layer_definition(name: &str) -> LayerDefinition {
    LayerDefinition {
        api_name: api(name),
        display: Some("Layer".to_string()),
        description: Some("Reusable layer".to_string()),
        kind: "collection".to_string(),
        input_schema: Some(object_value("type", "object")),
        output_schema: None,
        dependencies: vec![api("base_layer")],
        config: Some(BTreeMap::from([(
            "mode".to_string(),
            Value::string("materialized"),
        )])),
        metadata: None,
    }
}

fn function_definition(name: &str, handler: &str) -> FunctionDefinition {
    FunctionDefinition {
        api_name: api(name),
        display: Some("Function".to_string()),
        description: None,
        runtime: "wasm".to_string(),
        handler: handler.to_string(),
        input_schema: None,
        output_schema: Some(object_value("type", "object")),
        deterministic: Some(true),
        timeout_seconds: Some(30),
        metadata: None,
    }
}

fn workflow_definition(name: &str) -> WorkflowDefinition {
    WorkflowDefinition {
        api_name: api(name),
        display: Some("Workflow".to_string()),
        description: None,
        steps: vec![WorkflowStep {
            api_name: api("invoke_function"),
            kind: WorkflowStepKind::Function,
            target: api("normalize"),
            depends_on: Vec::new(),
            when: Some("input.enabled".to_string()),
            input: Some(object_value("source", "request")),
            metadata: None,
        }],
        triggers: vec![WorkflowTrigger {
            kind: "manual".to_string(),
            event_type: None,
            schedule: None,
            metadata: None,
        }],
        input_schema: None,
        output_schema: None,
        metadata: None,
    }
}

fn agent_definition() -> AgentDefinition {
    AgentDefinition {
        api_name: api("generic_agent"),
        display: Some("Generic agent".to_string()),
        description: None,
        model: Some("test-model".to_string()),
        model_provider: Some("test-provider".to_string()),
        instructions: Some("Use the available contracts.".to_string()),
        allowed_tools: vec![api("normalize")],
        custom_tools: Vec::new(),
        context_sources: Vec::new(),
        memory: None,
        limits: None,
        requires_approval: Some(false),
        output_schema: None,
        output_object_type: None,
        capabilities: vec!["summarize".to_string()],
        deprecated_at: None,
        metadata: None,
    }
}

fn contract_bundle() -> ContractBundle {
    let artifact = LayerArtifact {
        id: "artifact_1".to_string(),
        kind: "json".to_string(),
        uri: Some("memory://artifact_1".to_string()),
        value: None,
        media_type: Some("application/json".to_string()),
        produced_by: Some("layer_run_1".to_string()),
        metadata: None,
    };

    ContractBundle {
        action_proposal: ActionProposal {
            id: "proposal_1".to_string(),
            action: api("approve_request"),
            subject: Some(api("request")),
            input: Some(object_value("id", "request_1")),
            proposed_by: Some("agent".to_string()),
            reason: Some("Policy requires review".to_string()),
            requires_approval: Some(true),
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            metadata: None,
        },
        action_run: ActionRun {
            id: "action_run_1".to_string(),
            action: api("approve_request"),
            proposal_id: Some("proposal_1".to_string()),
            status: "completed".to_string(),
            input: Some(object_value("id", "request_1")),
            output: Some(object_value("result", "approved")),
            error: None,
            started_at: Some("2026-01-01T00:00:01Z".to_string()),
            completed_at: Some("2026-01-01T00:00:02Z".to_string()),
            metadata: None,
        },
        layer_definition: layer_definition("curated_layer"),
        layer_instance: LayerInstance {
            id: "layer_instance_1".to_string(),
            layer: api("curated_layer"),
            state: "ready".to_string(),
            inputs: BTreeMap::from([("source".to_string(), Value::string("input"))]),
            outputs: BTreeMap::from([("rows".to_string(), Value::integer(3))]),
            created_at: Some("2026-01-01T00:00:00Z".to_string()),
            updated_at: Some("2026-01-01T00:00:03Z".to_string()),
            metadata: None,
        },
        layer_run: LayerRun {
            id: "layer_run_1".to_string(),
            layer: api("curated_layer"),
            instance_id: Some("layer_instance_1".to_string()),
            status: "completed".to_string(),
            input: Some(object_value("source", "input")),
            output: Some(object_value("rows", "3")),
            artifacts: vec![artifact],
            error: None,
            started_at: Some("2026-01-01T00:00:00Z".to_string()),
            completed_at: Some("2026-01-01T00:00:03Z".to_string()),
            metadata: None,
        },
        function_definition: function_definition("normalize", "normalize.handler"),
        function_run: FunctionRun {
            id: "function_run_1".to_string(),
            function: api("normalize"),
            status: "completed".to_string(),
            input: Some(object_value("value", "A")),
            output: Some(object_value("value", "a")),
            error: None,
            started_at: None,
            completed_at: None,
            metadata: None,
        },
        workflow_definition: workflow_definition("review_workflow"),
        workflow_run: WorkflowRun {
            id: "workflow_run_1".to_string(),
            workflow: api("review_workflow"),
            status: "completed".to_string(),
            input: Some(object_value("enabled", "true")),
            output: Some(object_value("status", "done")),
            steps: vec![WorkflowStepRun {
                step: api("invoke_function"),
                status: "completed".to_string(),
                run_id: Some("function_run_1".to_string()),
                output: Some(object_value("value", "a")),
                error: None,
                metadata: None,
            }],
            error: None,
            started_at: None,
            completed_at: None,
            metadata: None,
        },
        agent_definition: agent_definition(),
        agent_run: AgentRun {
            id: "agent_run_1".to_string(),
            status: "completed".to_string(),
            output: Some("done".to_string()),
            error: None,
            messages: Vec::new(),
            tool_calls: Some(1),
            tokens_used: Some(10),
            cost_usd: None,
            eval_passed: Some(true),
            eval_score: None,
            eval_notes: None,
        },
        tool_call: ToolCall {
            id: "tool_call_1".to_string(),
            tool: "normalize".to_string(),
            arguments: Some(object_value("value", "A")),
            status: Some("completed".to_string()),
            result: Some(object_value("value", "a")),
            error: None,
            started_at: None,
            completed_at: None,
            metadata: None,
        },
        evidence_node: EvidenceNode {
            id: "evidence_1".to_string(),
            kind: "document".to_string(),
            reference: Some("memory://evidence_1".to_string()),
            payload: None,
            produced_by: Some(api("review_workflow")),
            occurred_at: Some("2026-01-01T00:00:04Z".to_string()),
            metadata: None,
        },
        decision_record: DecisionRecord {
            id: "decision_1".to_string(),
            status: "accepted".to_string(),
            decision: "Proceed".to_string(),
            decided_by: Some("reviewer".to_string()),
            rationale: Some("Evidence satisfied policy".to_string()),
            evidence: vec!["evidence_1".to_string()],
            alternatives: vec!["defer".to_string()],
            decided_at: Some("2026-01-01T00:00:05Z".to_string()),
            metadata: None,
        },
        lineage_edge: LineageEdge {
            source: api("raw_input"),
            kind: "derives_from".to_string(),
            metadata: None,
        },
        domain_event: DomainEvent {
            id: "event_1".to_string(),
            event_type: api("review_completed"),
            subject: Some(api("request")),
            payload: Some(object_value("status", "completed")),
            emitted_by: Some("workflow".to_string()),
            correlation_id: Some("correlation_1".to_string()),
            occurred_at: Some("2026-01-01T00:00:06Z".to_string()),
            metadata: None,
        },
    }
}

#[test]
fn generic_contracts_serde_roundtrip() {
    let original = contract_bundle();
    let json = serde_json::to_vec(&original).unwrap();
    let decoded: ContractBundle = serde_json::from_slice(&json).unwrap();

    assert_eq!(decoded, original);
}

#[test]
fn spec_hash_changes_when_layer_changes() {
    let mut first = Spec::default();
    first.upsert_layer(layer_definition("curated_layer"));

    let mut second = first.clone();
    second.layers[0].kind = "view".to_string();

    assert_ne!(first.hash(), second.hash());
}

#[test]
fn spec_hash_changes_when_function_changes() {
    let mut first = Spec::default();
    first.upsert_function(function_definition("normalize", "normalize.handler"));

    let mut second = first.clone();
    second.functions[0].handler = "normalize.v2".to_string();

    assert_ne!(first.hash(), second.hash());
}

#[test]
fn spec_hash_changes_when_workflow_changes() {
    let mut first = Spec::default();
    first.upsert_workflow(workflow_definition("review_workflow"));

    let mut second = first.clone();
    second.workflows[0].steps[0].target = api("normalize_v2");

    assert_ne!(first.hash(), second.hash());
}
