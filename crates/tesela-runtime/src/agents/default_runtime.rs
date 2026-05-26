//! DefaultAgentRuntime — the agentic loop implementation.

use super::tools;
use crate::constants::*;
use crate::ports::{
    AgentCommunicationChannel, AgentMemoryStore, AgentRuntime, ApprovalProvider, Compactor,
    IdGenerator, ModelProvider, Planner, PolicyEvaluator, SubagentRuntime,
};
use crate::query::{Actor, ApprovalRequest, Message, ModelRequest, ToolCall, ToolDef};
use tesela_core::Value;
use tesela_core::{ApiName, Error, Operation};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

// jsonschema used via path `jsonschema::validator_for` — no import needed

/// Options for constructing a [`DefaultAgentRuntime`].
pub struct DefaultAgentRuntimeOptions {
    /// Model provider.
    pub model_provider: Arc<dyn ModelProvider>,
    /// Planner.
    pub planner: Option<Arc<dyn Planner>>,
    /// Compactor.
    pub compactor: Option<Arc<dyn Compactor>>,
    /// Memory store.
    pub memory_store: Option<Arc<dyn AgentMemoryStore>>,
    /// Communication channel.
    pub channel: Option<Arc<dyn AgentCommunicationChannel>>,
    /// Subagent runtime.
    pub subagent_runtime: Option<Arc<dyn SubagentRuntime>>,
    /// Approval provider.
    pub approval_provider: Option<Arc<dyn ApprovalProvider>>,
    /// Policy evaluator.
    pub policy_evaluator: Option<Arc<dyn PolicyEvaluator>>,
    /// ID generator.
    pub id_generator: Arc<dyn IdGenerator>,
}

/// Default agent runtime implementation.
pub struct DefaultAgentRuntime {
    model_provider: Arc<dyn ModelProvider>,
    planner: Option<Arc<dyn Planner>>,
    compactor: Option<Arc<dyn Compactor>>,
    memory_store: Option<Arc<dyn AgentMemoryStore>>,
    channel: Option<Arc<dyn AgentCommunicationChannel>>,
    subagent_runtime: Option<Arc<dyn SubagentRuntime>>,
    approval_provider: Option<Arc<dyn ApprovalProvider>>,
    policy_evaluator: Option<Arc<dyn PolicyEvaluator>>,
    id_generator: Arc<dyn IdGenerator>,
    runs: Mutex<BTreeMap<String, tesela_ir::AgentRun>>,
}

impl DefaultAgentRuntime {
    /// Create a new agent runtime.
    pub fn new(opts: DefaultAgentRuntimeOptions) -> Self {
        Self {
            model_provider: opts.model_provider,
            planner: opts.planner,
            compactor: opts.compactor,
            memory_store: opts.memory_store,
            channel: opts.channel,
            subagent_runtime: opts.subagent_runtime,
            approval_provider: opts.approval_provider,
            policy_evaluator: opts.policy_evaluator,
            id_generator: opts.id_generator,
            runs: Mutex::new(BTreeMap::new()),
        }
    }

    fn build_system_prompt(&self, agent: &tesela_ir::Agent, tools: &[ToolDef]) -> String {
        let mut prompt = format!(
            "You are the agent '{}'.\nInstructions: {}\n\n",
            agent.api_name,
            agent.instructions.as_deref().unwrap_or("")
        );
        if !tools.is_empty() {
            prompt.push_str("Available tools:\n");
            for tool in tools {
                prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
            }
        }
        prompt
    }

    fn check_tool_policy(&self, tool_name: &str, actor: &Actor) -> Result<bool, Error> {
        if let Some(policy) = &self.policy_evaluator {
            let op = match tool_name {
                n if n.starts_with(TOOL_PREFIX_SEARCH) => Operation::Search,
                n if n.starts_with(TOOL_PREFIX_GET) => Operation::Read,
                n if n.starts_with(TOOL_PREFIX_MUTATE) => Operation::Mutate,
                n if n.starts_with(TOOL_PREFIX_AGGREGATE) => Operation::Aggregate,
                n if n.starts_with(TOOL_PREFIX_TRAVERSE) => Operation::Traverse,
                n if n.starts_with(TOOL_PREFIX_EXECUTE) => Operation::Execute,
                _ => Operation::Search,
            };
            let req = crate::query::PolicyRequest {
                actor: actor.clone(),
                operation: op,
                resource_kind: RESOURCE_KIND_TOOL.to_string(),
                resource: ApiName::new_unchecked(tool_name),
                context: BTreeMap::new(),
                resource_instance: None,
                request_meta: None,
                capability: None,
                operation_params: BTreeMap::new(),
            };
            let decision = policy.evaluate(&req)?;
            Ok(decision.allow)
        } else {
            Ok(true)
        }
    }

    fn assemble_tools(&self, _agent: &tesela_ir::Agent, actor: &Actor) -> Vec<ToolDef> {
        let mut all_tools = Vec::new();

        if self.memory_store.is_some() {
            all_tools.extend(tools::memory_tools());
        }
        if self.channel.is_some() {
            all_tools.extend(tools::channel_tools());
        }
        if self.subagent_runtime.is_some() {
            all_tools.extend(tools::subagent_tools());
        }

        all_tools
            .into_iter()
            .filter(|t| self.check_tool_policy(&t.name, actor).unwrap_or(true))
            .collect()
    }

    fn execute_tool(
        &self,
        tool_call: &ToolCall,
        _agent: &tesela_ir::Agent,
        _actor: &Actor,
    ) -> Result<Value, Error> {
        let args: BTreeMap<String, Value> =
            serde_json::from_str(&tool_call.arguments).unwrap_or_default();

        match tool_call.name.as_str() {
            TOOL_MEMORY_REMEMBER => {
                if let Some(store) = &self.memory_store {
                    let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    let value = args.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    store.remember("agent", key, value)?;
                    return Ok(Value::from("ok"));
                }
            }
            TOOL_MEMORY_RECALL => {
                if let Some(store) = &self.memory_store {
                    let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    let val = store.recall("agent", key)?;
                    return Ok(Value::from(val.unwrap_or_default()));
                }
            }
            TOOL_MEMORY_SEARCH => {
                if let Some(store) = &self.memory_store {
                    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                    let results = store.search_memory("agent", query)?;
                    return Ok(Value::from(
                        results.into_iter().map(Value::from).collect::<Vec<Value>>(),
                    ));
                }
            }
            TOOL_MEMORY_FORGET => {
                if let Some(store) = &self.memory_store {
                    let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
                    store.forget("agent", key)?;
                    return Ok(Value::from("ok"));
                }
            }
            TOOL_CHANNEL_SEND => {
                if let Some(ch) = &self.channel {
                    let channel = args.get("channel").and_then(|v| v.as_str()).unwrap_or("");
                    let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
                    ch.send(channel, message)?;
                    return Ok(Value::from("ok"));
                }
            }
            TOOL_CHANNEL_RECEIVE => {
                if let Some(ch) = &self.channel {
                    let channel = args.get("channel").and_then(|v| v.as_str()).unwrap_or("");
                    let timeout_ms = args
                        .get("timeout_ms")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(DEFAULT_CHANNEL_TIMEOUT_MS as i64)
                        as u64;
                    let msg = ch.receive(channel, timeout_ms)?;
                    return Ok(Value::from(msg.unwrap_or_default()));
                }
            }
            TOOL_SUBAGENT_SPAWN => {
                if let Some(_sub) = &self.subagent_runtime {
                    let agent_name = args
                        .get("agent_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let _input = args.get("input").cloned().unwrap_or_default();
                    return Ok(Value::from(format!("spawned {}", agent_name)));
                }
            }
            TOOL_SUBAGENT_WAIT => {
                if let Some(sub) = &self.subagent_runtime {
                    let run_id = args.get("run_id").and_then(|v| v.as_str()).unwrap_or("");
                    let timeout_ms = args
                        .get("timeout_ms")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(DEFAULT_SUBAGENT_TIMEOUT_MS as i64)
                        as u64;
                    let _run = sub.wait(run_id, timeout_ms)?;
                    return Ok(Value::from("done"));
                }
            }
            _ => {}
        }
        Ok(Value::from(format!("unknown tool: {}", tool_call.name)))
    }
}

impl AgentRuntime for DefaultAgentRuntime {
    fn start_run(
        &self,
        agent: &tesela_ir::Agent,
        input: Value,
        actor: &Actor,
    ) -> Result<String, Error> {
        let run_id = self.id_generator.new_id("run");
        let tools = self.assemble_tools(agent, actor);
        let system_prompt = self.build_system_prompt(agent, &tools);

        let _plan = if let Some(planner) = &self.planner {
            planner.plan(&system_prompt, &tools)?
        } else {
            None
        };

        let mut messages = vec![
            Message {
                role: ROLE_SYSTEM.to_string(),
                content: system_prompt,
                tool_calls: Vec::new(),
                tool_call_id: None,
            },
            Message {
                role: ROLE_USER.to_string(),
                content: serde_json::to_string(&input).unwrap_or_default(),
                tool_calls: Vec::new(),
                tool_call_id: None,
            },
        ];

        if agent.requires_approval == Some(true)
            && let Some(approval) = &self.approval_provider
        {
            let req = ApprovalRequest {
                resource: agent.api_name.to_string(),
                actor: actor.clone(),
                reason: APPROVAL_REASON_HIGH_RISK_AGENT.to_string(),
            };
            let decision = approval.request_approval(req)?;
            if !decision.approved {
                return Err(Error::policy_denied(
                    "agent execution denied by approval gate",
                ));
            }
        }

        let max_tool_calls = agent
            .limits
            .as_ref()
            .and_then(|l| l.max_tool_calls)
            .unwrap_or(DEFAULT_MAX_TOOL_CALLS as i32) as usize;
        let token_budget = agent.limits.as_ref().and_then(|l| l.token_budget);
        let mut tool_calls_count = 0;

        for _turn in 0..max_tool_calls {
            if let Some(compactor) = &self.compactor {
                let should_compact = match token_budget {
                    Some(budget) => {
                        let estimated: usize =
                            messages.iter().map(|m| m.content.len() / 4 + 1).sum();
                        estimated > budget as usize
                    }
                    None => messages.len() > DEFAULT_COMPACTION_THRESHOLD,
                };
                if should_compact {
                    messages = compactor.compact(&messages)?;
                }
            }

            let model_req = ModelRequest {
                system: messages
                    .iter()
                    .find(|m| m.role == ROLE_SYSTEM)
                    .map(|m| m.content.clone())
                    .unwrap_or_default(),
                messages: messages
                    .iter()
                    .filter(|m| m.role != ROLE_SYSTEM)
                    .map(|m| {
                        let mut map = BTreeMap::new();
                        map.insert("role".to_string(), Value::from(m.role.as_str()));
                        map.insert("content".to_string(), Value::from(m.content.as_str()));
                        map
                    })
                    .collect(),
                tools: tools.clone(),
                max_tokens: agent.limits.as_ref().and_then(|l| l.max_tokens),
                temperature: None,
                tool_choice: None,
                response_format: None,
                parallel_tool_calls: None,
            };

            let resp = self.model_provider.call(model_req)?;

            let assistant_msg = Message {
                role: ROLE_ASSISTANT.to_string(),
                content: resp.content.clone(),
                tool_calls: resp.tool_calls.clone(),
                tool_call_id: None,
            };
            messages.push(assistant_msg);

            if resp.tool_calls.is_empty() {
                break;
            }

            for tc in &resp.tool_calls {
                tool_calls_count += 1;
                if tool_calls_count > max_tool_calls {
                    return Err(Error::validation("max tool calls exceeded"));
                }
                let result = self.execute_tool(tc, agent, actor)?;
                messages.push(Message {
                    role: ROLE_TOOL.to_string(),
                    content: serde_json::to_string(&result).unwrap_or_default(),
                    tool_calls: Vec::new(),
                    tool_call_id: Some(tc.id.clone()),
                });
            }
        }

        let final_output = messages.last().map(|m| m.content.clone());
        if let (Some(schema), Some(output)) = (&agent.output_schema, &final_output)
            && let Ok(output_json) = serde_json::from_str::<serde_json::Value>(output)
        {
            let validator = jsonschema::validator_for(&schema.0)
                .map_err(|e| Error::validation(format!("invalid output schema: {}", e)))?;
            let validation_errors: Vec<String> = validator
                .iter_errors(&output_json)
                .map(|e| e.to_string())
                .collect();
            if !validation_errors.is_empty() {
                return Err(Error::validation(format!(
                    "agent '{}' output failed schema validation: {}",
                    agent.api_name,
                    validation_errors.join("; ")
                )));
            }
        }

        let run = tesela_ir::AgentRun {
            id: run_id.clone(),
            status: AGENT_STATUS_COMPLETED.to_string(),
            output: final_output,
            error: None,
            messages: messages
                .iter()
                .map(|m| {
                    let mut map = BTreeMap::new();
                    map.insert("role".to_string(), Value::from(m.role.as_str()));
                    map.insert("content".to_string(), Value::from(m.content.as_str()));
                    map
                })
                .collect(),
            tool_calls: Some(tool_calls_count as i32),
            tokens_used: None,
            cost_usd: None,
            eval_passed: None,
            eval_score: None,
            eval_notes: None,
        };

        self.runs
            .lock()
            .map_err(|_| Error::internal("agent runs lock poisoned"))?
            .insert(run_id.clone(), run);
        Ok(run_id)
    }

    fn get_run(&self, run_id: &str) -> Result<tesela_ir::AgentRun, Error> {
        self.runs
            .lock()
            .map_err(|_| Error::internal("agent runs lock poisoned"))?
            .get(run_id)
            .cloned()
            .ok_or_else(|| Error::not_found("agent_run", run_id))
    }
}
