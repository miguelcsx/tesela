//! Built-in policy helpers.

use tesela_core::Error;
use tesela_store::{PolicyDecision, PolicyEngine, PolicyRequest};

/// Policy engine for trusted tests and local harnesses.
pub struct AllowAllPolicy;

impl PolicyEngine for AllowAllPolicy {
    fn evaluate(&self, _request: &PolicyRequest) -> Result<PolicyDecision, Error> {
        Ok(PolicyDecision {
            allow: true,
            ..PolicyDecision::default()
        })
    }
}
