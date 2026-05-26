//! Internal helpers, free functions, and default infrastructure implementations.

use crate::ports::*;
use crate::query::*;
use crate::runtime::Runtime;
use tesela_core::{ApiName, Error, Operation, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

impl Runtime {
    /// Evaluate policy and return the full decision.
    pub(crate) fn evaluate_policy(
        &self,
        actor: &Actor,
        operation: Operation,
        resource_kind: &str,
        resource: &ApiName,
    ) -> Result<PolicyDecision, Error> {
        match &self.policy_evaluator {
            Some(policy) => {
                let req = PolicyRequest {
                    actor: actor.clone(),
                    operation,
                    resource_kind: resource_kind.to_string(),
                    resource: resource.clone(),
                    context: BTreeMap::new(),
                    resource_instance: None,
                    request_meta: None,
                    capability: None,
                    operation_params: BTreeMap::new(),
                };
                policy.evaluate(&req)
            }
            None => Ok(PolicyDecision {
                allow: true,
                ..Default::default()
            }),
        }
    }

    /// Evaluate policy with fully populated request context.
    pub(crate) fn evaluate_policy_request(
        &self,
        req: PolicyRequest,
    ) -> Result<PolicyDecision, Error> {
        match &self.policy_evaluator {
            Some(policy) => policy.evaluate(&req),
            None => Ok(PolicyDecision {
                allow: true,
                ..Default::default()
            }),
        }
    }

    /// Authorize a fully populated request.
    pub(crate) fn authorize_request(&self, req: PolicyRequest) -> Result<PolicyDecision, Error> {
        let actor = req.actor.clone();
        let operation = req.operation;
        let resource_kind = req.resource_kind.clone();
        let resource = req.resource.clone();
        let decision = self.evaluate_policy_request(req)?;
        if !decision.allow {
            self.audit_and_event(&actor, operation, &resource_kind, &resource, false, 0)?;
            return Err(Error::policy_denied(
                decision
                    .reason
                    .unwrap_or_else(|| "policy denied".to_string()),
            ));
        }
        Ok(decision)
    }

    /// Authorize an operation; return the full policy decision for further use.
    pub(crate) fn authorize_with_decision(
        &self,
        actor: &Actor,
        operation: Operation,
        resource_kind: &str,
        resource: &ApiName,
    ) -> Result<PolicyDecision, Error> {
        let decision = self.evaluate_policy(actor, operation, resource_kind, resource)?;
        if !decision.allow {
            self.audit_and_event(actor, operation, resource_kind, resource, false, 0)?;
            return Err(Error::policy_denied(
                decision
                    .reason
                    .unwrap_or_else(|| "policy denied".to_string()),
            ));
        }
        Ok(decision)
    }

    /// Execute all obligations from a policy decision.
    pub(crate) fn run_obligations(
        &self,
        _actor: &Actor,
        decision: &PolicyDecision,
        context: &BTreeMap<String, Value>,
    ) -> Result<(), Error> {
        if decision.obligations.is_empty() {
            return Ok(());
        }
        let exec = match &self.obligation_executor {
            Some(e) => e,
            None => return Ok(()),
        };
        for obligation in &decision.obligations {
            exec.execute(obligation, context)?;
        }
        Ok(())
    }

    pub(crate) fn acquire_backend(&self, ds_name: &ApiName) -> Result<Box<dyn Backend>, Error> {
        let registry = self
            .backend_registry
            .as_ref()
            .ok_or_else(|| Error::internal("no backend registry configured"))?;
        registry.acquire(ds_name)
    }

    pub(crate) fn audit_and_event(
        &self,
        actor: &Actor,
        operation: Operation,
        resource_kind: &str,
        resource: &ApiName,
        success: bool,
        result_count: i64,
    ) -> Result<(), Error> {
        let record = AuditRecord {
            id: self.id_generator.new_id("audit"),
            occurred_at: self.clock.now().to_rfc3339(),
            actor_user_id: actor.user_id.clone(),
            operation: format!("{:?}", operation),
            resource_kind: resource_kind.to_string(),
            resource: resource.to_string(),
            decision: if success {
                "allow".to_string()
            } else {
                "deny".to_string()
            },
            result_count: Some(result_count),
            error_code: None,
            metadata: BTreeMap::new(),
        };
        self.audit_sink.write_audit(record)?;

        let workspace_name = self
            .ontology()
            .map(|snap| snap.spec.workspace.api_name.to_string())
            .unwrap_or_default();
        let event = Event {
            id: self.id_generator.new_id("evt"),
            kind: format!("{:?}", operation),
            workspace: workspace_name,
            object_type: Some(resource.to_string()),
            actor_user_id: actor.user_id.clone(),
            occurred_at: self.clock.now().to_rfc3339(),
            payload: BTreeMap::new(),
            event_type: None,
            topic: None,
            correlation_id: None,
            causation_id: None,
        };
        self.event_bus.publish(event)?;
        Ok(())
    }
}

/// Null out redacted fields in a record.
pub(crate) fn apply_redactions(record: &mut tesela_ir::Record, fields: &[ApiName]) {
    for field in fields {
        record.values.insert(field.clone(), Value::null());
    }
}

/// Extract a representative `Record` from a mutation for quality rule checks.
pub(crate) fn mutation_to_record(mutation: &Mutation) -> tesela_ir::Record {
    let values = match mutation {
        Mutation::Create { values } | Mutation::Upsert { values } => values.clone(),
        Mutation::Update { values, .. } => values.clone(),
        Mutation::Delete { .. } | Mutation::Batch { .. } => BTreeMap::new(),
    };
    tesela_ir::Record {
        primary_key: None,
        values,
    }
}

/// Auto-index vector-typed properties after a successful write.
pub(crate) fn index_vectors_for_mutation(
    mutation: &Mutation,
    object_name: &ApiName,
    ot: &tesela_ir::ObjectType,
    vb: &dyn VectorBackend,
) {
    use tesela_core::DataType;

    let (pk, values) = match mutation {
        Mutation::Create { values } | Mutation::Upsert { values } => {
            let pk = ot
                .properties
                .iter()
                .find(|p| p.api_name == ot.primary_key)
                .and_then(|p| values.get(&p.api_name))
                .cloned()
                .unwrap_or_default();
            (pk, values)
        }
        Mutation::Update {
            primary_key,
            values,
        } => (primary_key.clone(), values),
        _ => return,
    };

    for prop in &ot.properties {
        if let DataType::Vector(_dim) = prop.data_type
            && let Some(val) = values.get(&prop.api_name)
            && let Some(arr) = val.as_array()
        {
            let floats: Vec<f32> = arr
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            if !floats.is_empty() {
                let _ = vb.index_vector(object_name, &pk, &floats);
            }
        }
    }
}

/// Record a lineage `Produces` edge for each declared lineage entry.
pub(crate) fn record_mutation_lineage(
    ls: &dyn LineageStore,
    object_name: &ApiName,
    record: &tesela_ir::Record,
    ot: &tesela_ir::ObjectType,
    actor_user_id: &str,
    occurred_at: &str,
    id_gen: &Arc<dyn IdGenerator>,
) {
    let pk = record
        .primary_key
        .clone()
        .unwrap_or_else(|| Value::string("unknown"));

    for edge in &ot.lineage {
        let lineage_record =
            crate::lineage::build_produces_edge(crate::lineage::ProducesEdgeParams {
                id: id_gen.new_id("lin"),
                source_object_type: edge.source.clone(),
                source_pk: pk.clone(),
                target_object_type: object_name.clone(),
                target_pk: pk.clone(),
                actor_user_id: actor_user_id.to_string(),
                occurred_at: occurred_at.to_string(),
                pipeline: None,
            });
        let _ = ls.record(lineage_record);
    }
}

/// Default ID generator using UUID v4.
pub struct DefaultIdGenerator;

impl IdGenerator for DefaultIdGenerator {
    fn new_id(&self, prefix: &str) -> String {
        format!("{}_{}", prefix, uuid::Uuid::new_v4())
    }
}

/// System clock.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}
