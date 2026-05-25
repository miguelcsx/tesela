//! Agent end-to-end example with a mock ModelProvider.
//!
//! Demonstrates defining an agent spec, wiring it into a Runtime with a mock
//! LLM provider, and running the agent loop. The mock model makes one tool
//! call (search_product) and then returns a final answer.

use lattice::memory::{DefaultBackendRegistry, MemoryBackend};
use lattice::runtime::agents::DefaultAgentRuntime;
use lattice::runtime::ports::{AgentRuntime, IdGenerator, ModelProvider};
use lattice::runtime::query::{Actor, ModelRequest, ModelResponse, Query, ToolCall};
use lattice::runtime::runtime::{Runtime, RuntimeOptions};
use lattice::sdk::{AgentBuilder, App, ObjectTypeBuilder, PropertyBuilder};
use lattice_core::{ApiName, DataType, Error, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Mock ModelProvider
// ---------------------------------------------------------------------------

struct MockModel {
    call_count: Mutex<u32>,
}

impl MockModel {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            call_count: Mutex::new(0),
        })
    }
}

impl ModelProvider for MockModel {
    fn call(&self, _req: ModelRequest) -> Result<ModelResponse, Error> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;
        if *count == 1 {
            // First call: return a tool call to search_product.
            Ok(ModelResponse {
                content: String::new(),
                tool_calls: vec![ToolCall {
                    name: "search_product".to_string(),
                    id: "call_1".to_string(),
                    arguments: r#"{"limit":5}"#.to_string(),
                }],
                tokens_used: Some(100),
                structured_output: None,
            })
        } else {
            // Subsequent calls: return a final answer.
            Ok(ModelResponse {
                content: "I found 0 products in the catalogue.".to_string(),
                tool_calls: Vec::new(),
                tokens_used: Some(50),
                structured_output: None,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Minimal IdGenerator and Clock
// ---------------------------------------------------------------------------

struct SequentialIdGenerator(Mutex<u64>);

impl SequentialIdGenerator {
    fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(0)))
    }
}

impl IdGenerator for SequentialIdGenerator {
    fn new_id(&self, prefix: &str) -> String {
        let mut n = self.0.lock().unwrap();
        *n += 1;
        format!("{}-{}", prefix, *n)
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let app = App::new("shop")
        .object_type(
            ObjectTypeBuilder::new("product")
                .display("Product")
                .property(
                    PropertyBuilder::new("id", DataType::String)
                        .required(true)
                        .build(),
                )
                .property(PropertyBuilder::new("name", DataType::String).build())
                .build(),
        )
        .agent(
            AgentBuilder::new("catalogue_agent")
                .display("Catalogue Agent")
                .model("mock-model")
                .instructions("You help users find products.")
                .allow_tool("search_product")
                .build(),
        );

    let result = app.compile();
    assert!(result.is_valid, "compile failed: {:?}", result.diagnostics);
    let spec = result.spec.unwrap();

    // Wire memory backend.
    let registry = DefaultBackendRegistry::new();
    registry.register(ApiName::new_unchecked("memory"), MemoryBackend::new()).unwrap();
    let registry_dyn: Arc<dyn lattice::runtime::ports::BackendRegistry> = registry;

    // Build the agent runtime.
    let agent_runtime = Arc::new(DefaultAgentRuntime::new(
        lattice::runtime::agents::DefaultAgentRuntimeOptions {
            model_provider: MockModel::new(),
            planner: None,
            compactor: None,
            memory_store: None,
            channel: None,
            subagent_runtime: None,
            approval_provider: None,
            policy_evaluator: None,
            id_generator: SequentialIdGenerator::new(),
        },
    ));
    let agent_runtime_dyn: Arc<dyn AgentRuntime> = agent_runtime;

    let runtime = Runtime::new(
        spec,
        RuntimeOptions {
            backend_registry: Some(registry_dyn),
            agent_runtime: Some(agent_runtime_dyn),
            ..RuntimeOptions::dev()
        },
    )
    .unwrap();

    let actor = Actor {
        user_id: "user1".to_string(),
        roles: vec!["user".to_string()],
        claims: BTreeMap::new(),
    };

    // Start an agent run.
    let agent_name = ApiName::new_unchecked("catalogue_agent");
    let run_id = runtime
        .start_agent_run(&actor, &agent_name, Value::null())
        .unwrap();
    println!("Started agent run: {}", run_id);

    // Poll the run status.
    let run = runtime.get_agent_run(&actor, &run_id).unwrap();
    println!("Status : {}", run.status);
    println!("Output : {:?}", run.output);

    // Also verify product search works.
    let page = runtime
        .search(&actor, &ApiName::new_unchecked("product"), Query::default())
        .unwrap();
    println!("Products: {}", page.records.len());
}
