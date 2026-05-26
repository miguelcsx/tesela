//! Policy evaluation implementation.

use crate::ports::{ConditionEvaluator, ObligationExecutor, PolicyEvaluator};
use crate::query::{Actor, PolicyDecision, PolicyRequest};
use tesela_core::{ApiName, Error, PolicyEffect};
use tesela_ir::PolicyRule;
use std::collections::{BTreeMap, HashSet};

/// A simple rules-based policy evaluator.
///
/// Evaluates rules in priority order. Deny short-circuits.
/// Allow accumulates row filters (AND-merge), redactions (union), and obligations (append).
/// Role inheritance is resolved via transitive closure.
pub struct StaticRules<C, O>
where
    C: ConditionEvaluator,
    O: ObligationExecutor,
{
    /// All policy rules.
    rules: Vec<PolicyRule>,
    /// Role inheritance map: role -> parent roles.
    role_inheritance: BTreeMap<ApiName, Vec<ApiName>>,
    /// Condition evaluator.
    condition_eval: C,
    /// Obligation executor.
    obligation_exec: O,
}

impl<C, O> StaticRules<C, O>
where
    C: ConditionEvaluator,
    O: ObligationExecutor,
{
    /// Create a new static rules evaluator.
    pub fn new(
        rules: Vec<PolicyRule>,
        role_inheritance: BTreeMap<ApiName, Vec<ApiName>>,
        condition_eval: C,
        obligation_exec: O,
    ) -> Self {
        Self {
            rules,
            role_inheritance,
            condition_eval,
            obligation_exec,
        }
    }

    /// Resolve all roles for an actor including inherited ones.
    fn resolve_roles(&self, actor: &Actor) -> HashSet<String> {
        let mut resolved = HashSet::new();
        let mut queue: Vec<String> = actor.roles.clone();
        while let Some(role_str) = queue.pop() {
            if resolved.insert(role_str.clone())
                && let Ok(role_name) = role_str.parse::<ApiName>()
                && let Some(parents) = self.role_inheritance.get(&role_name)
            {
                for parent in parents {
                    queue.push(parent.to_string());
                }
            }
        }
        resolved
    }
}

impl<C, O> PolicyEvaluator for StaticRules<C, O>
where
    C: ConditionEvaluator,
    O: ObligationExecutor,
{
    fn evaluate(&self, req: &PolicyRequest) -> Result<PolicyDecision, Error> {
        let actor_roles = self.resolve_roles(&req.actor);
        let mut decision = PolicyDecision {
            allow: false,
            reason: None,
            row_filter: None,
            redactions: Vec::new(),
            obligations: Vec::new(),
        };

        // Sort rules by priority (higher first).
        let mut sorted_rules = self.rules.clone();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in &sorted_rules {
            // Check role match.
            let role_match = rule.roles.is_empty()
                || rule
                    .roles
                    .iter()
                    .any(|r| actor_roles.contains(&r.to_string()));
            if !role_match {
                continue;
            }

            // Check operation match.
            let op_match = rule.operations.is_empty() || rule.operations.contains(&req.operation);
            if !op_match {
                continue;
            }

            // Check resource match against optional single-field filter.
            let resource_match = rule.resource.as_ref().is_none_or(|r| r == &req.resource);
            let resource_kind_match = rule
                .resource_kind
                .as_ref()
                .is_none_or(|k| k == &req.resource_kind);
            if !resource_match || !resource_kind_match {
                continue;
            }

            // Check condition if present.
            if let Some(cond) = &rule.condition {
                let mut ctx = req.context.clone();
                ctx.insert(
                    "actor".to_string(),
                    serde_json::to_value(&req.actor).unwrap_or_default().into(),
                );
                if let Some(resource_instance) = &req.resource_instance {
                    ctx.insert(
                        "resource_instance".to_string(),
                        serde_json::to_value(resource_instance)
                            .unwrap_or_default()
                            .into(),
                    );
                }
                if let Some(request_meta) = &req.request_meta {
                    ctx.insert(
                        "request_meta".to_string(),
                        serde_json::to_value(request_meta)
                            .unwrap_or_default()
                            .into(),
                    );
                }
                if let Some(capability) = &req.capability {
                    ctx.insert(
                        "capability".to_string(),
                        serde_json::to_value(capability).unwrap_or_default().into(),
                    );
                }
                if !req.operation_params.is_empty() {
                    ctx.insert(
                        "operation_params".to_string(),
                        serde_json::to_value(&req.operation_params)
                            .unwrap_or_default()
                            .into(),
                    );
                }
                match self.condition_eval.evaluate(cond, &ctx) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(e) => {
                        return Err(Error::policy_denied(format!(
                            "condition evaluation error: {}",
                            e
                        )));
                    }
                }
            }

            match rule.effect {
                PolicyEffect::Deny => {
                    decision.allow = false;
                    decision.reason = Some(format!("denied by policy rule '{}'", rule.api_name));
                    return Ok(decision);
                }
                PolicyEffect::Allow => {
                    decision.allow = true;
                    if let Some(filter) = &rule.row_filter {
                        // AND-merge: if existing, wrap in And with new filter.
                        decision.row_filter =
                            Some(merge_and_filter(decision.row_filter.take(), filter.clone()));
                    }
                    for redaction in &rule.redactions {
                        if !decision.redactions.contains(redaction) {
                            decision.redactions.push(redaction.clone());
                        }
                    }
                    for obligation in &rule.obligations {
                        decision.obligations.push(obligation.clone());
                    }
                }
            }
        }

        if !decision.allow && decision.reason.is_none() {
            decision.reason = Some("no matching allow rule".to_string());
        }

        // Execute obligations on an allow.
        if decision.allow {
            for obligation in &decision.obligations {
                if let Err(e) = self.obligation_exec.execute(obligation, &req.context) {
                    return Err(Error::policy_denied(format!(
                        "obligation execution failed: {}",
                        e
                    )));
                }
            }
        }

        Ok(decision)
    }
}

/// Merge two filters with And.
fn merge_and_filter(
    left: Option<tesela_ir::Filter>,
    right: tesela_ir::Filter,
) -> tesela_ir::Filter {
    match left {
        Some(l) => tesela_ir::Filter {
            op: tesela_core::FilterOp::And,
            field: None,
            value: None,
            values: Vec::new(),
            args: vec![l, right],
        },
        None => right,
    }
}

/// A condition evaluator that always returns true (for simple setups).
pub struct AlwaysTrueConditionEvaluator;

impl ConditionEvaluator for AlwaysTrueConditionEvaluator {
    fn evaluate(
        &self,
        _condition: &str,
        _context: &BTreeMap<String, tesela_core::Value>,
    ) -> Result<bool, Error> {
        Ok(true)
    }
}

/// An obligation executor that does nothing.
pub struct NoopObligationExecutor;

impl ObligationExecutor for NoopObligationExecutor {
    fn execute(
        &self,
        _obligation: &tesela_ir::Obligation,
        _context: &BTreeMap<String, tesela_core::Value>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
