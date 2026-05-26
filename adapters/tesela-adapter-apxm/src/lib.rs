#![deny(warnings)]
#![deny(missing_docs)]

//! APXM agent runtime adapter for Tesela.
//!
//! Bridges Tesela's [`tesela_runtime::ports::AgentRuntime`] port trait to an
//! APXM service over HTTP. This process-boundary approach keeps Tesela on
//! stable Rust while APXM runs independently with its nightly/MLIR toolchain.
//!
//! # Execution model
//!
//! Tesela agents map to APXM skills:
//!
//! - `start_run` → `POST /v1/skills/{skill_id}/execute`
//! - `get_run` → `GET /v1/executions/{execution_id}`
//!
//! The skill ID is resolved from `agent.metadata["apxm_skill_id"]`, falling
//! back to the agent's `api_name`.
//!
//! # Usage
//!
//! ```rust,ignore
//! use tesela_adapter_apxm::ApxmAgentRuntime;
//!
//! let apxm = ApxmAgentRuntime::new("http://localhost:8081");
//! runtime_opts.agent_runtime = Some(Arc::new(apxm));
//! ```

mod runtime;

pub use runtime::ApxmAgentRuntime;
