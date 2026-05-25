#![deny(warnings)]
#![deny(missing_docs)]

//! APXM agent runtime adapter for Lattice.
//!
//! Bridges Lattice's [`AgentRuntime`] and [`ModelProvider`] port traits to
//! an APXM service over HTTP. This process-boundary approach keeps Lattice
//! on stable Rust while APXM runs independently with its nightly/MLIR
//! toolchain.
//!
//! # Usage
//!
//! ```rust,ignore
//! use lattice_adapter_apxm::ApxmAgentRuntime;
//!
//! let apxm = ApxmAgentRuntime::new("http://localhost:8081");
//! runtime_opts.agent_runtime = Some(Arc::new(apxm));
//! ```

mod runtime;

pub use runtime::ApxmAgentRuntime;
