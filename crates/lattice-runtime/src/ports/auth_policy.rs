//! Auth and policy port traits.

use crate::query::*;
use lattice_core::{Error, Value};
use std::collections::BTreeMap;

/// Resolves an `Actor` from raw request metadata.
pub trait ActorResolver: Send + Sync {
    /// Resolve actor from request.
    fn resolve(&self, request: &RequestMeta) -> Result<Actor, Error>;
}

/// Enriches an actor with additional claims / roles.
pub trait ActorEnricher: Send + Sync {
    /// Enrich actor.
    fn enrich(&self, actor: Actor) -> Result<Actor, Error>;
}

/// Evaluates a policy request and returns a decision.
pub trait PolicyEvaluator: Send + Sync {
    /// Evaluate policy.
    fn evaluate(&self, req: &PolicyRequest) -> Result<PolicyDecision, Error>;
}

/// Evaluates a condition string (e.g., CEL expression).
pub trait ConditionEvaluator: Send + Sync {
    /// Evaluate condition in context.
    fn evaluate(&self, condition: &str, context: &BTreeMap<String, Value>) -> Result<bool, Error>;
}

/// Executes an obligation side-effect.
pub trait ObligationExecutor: Send + Sync {
    /// Execute obligation.
    fn execute(
        &self,
        obligation: &lattice_ir::Obligation,
        context: &BTreeMap<String, Value>,
    ) -> Result<(), Error>;
}
