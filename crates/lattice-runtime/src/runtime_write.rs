//! Write and execution operations on the runtime.

use crate::ports::*;
use crate::query::*;
use crate::runtime::Runtime;
use crate::runtime_internal::{
    apply_redactions, index_vectors_for_mutation, mutation_to_record, record_mutation_lineage,
};
use crate::EvalContext;
use lattice_core::{ApiName, Error, Operation, Value};
use lattice_ir::{ExecutionMode, MutationResult, Page, PipelineResult};

impl Runtime {
    /// Apply a mutation to an object type.
    #[tracing::instrument(skip(self, actor, mutation), err)]
    pub fn mutate(
        &self,
        actor: &Actor,
        object_name: &ApiName,
        mutation: Mutation,
    ) -> Result<MutationResult, Error> {
        let start = std::time::Instant::now();
        self.check_rate_limit(actor, "mutate")?;
        self.authorize_with_decision(actor, Operation::Mutate, "object_type", object_name)?;
        let snap = self.ontology()?;
        let ot = snap
            .object_types
            .get(object_name)
            .cloned()
            .ok_or_else(|| Error::not_found("object_type", object_name))?;

        if let Some(qre) = &self.quality_rule_evaluator {
            let record = mutation_to_record(&mutation);
            qre.validate(&ot, &record)?;
        }

        let mut mutation = mutation;
        if let Some(sealer) = &self.sealer {
            match &mut mutation {
                Mutation::Create { values }
                | Mutation::Update { values, .. }
                | Mutation::Upsert { values } => {
                    crate::encrypt::encrypt_sensitive_fields(values, &ot, sealer.as_ref())?;
                }
                Mutation::Batch { items } => {
                    for item in items {
                        match item {
                            Mutation::Create { values }
                            | Mutation::Update { values, .. }
                            | Mutation::Upsert { values } => {
                                crate::encrypt::encrypt_sensitive_fields(
                                    values,
                                    &ot,
                                    sealer.as_ref(),
                                )?;
                            }
                            _ => {}
                        }
                    }
                }
                Mutation::Delete { .. } => {}
            }
        }

        let backend = self.acquire_backend(&ot.source.datasource)?;
        let mutator = backend
            .as_ref()
            .as_mutator()
            .ok_or_else(|| Error::unsupported("mutate"))?;
        let result = mutator.mutate(object_name, &mutation)?;

        if let Some(vb) = &self.vector_backend {
            index_vectors_for_mutation(&mutation, object_name, &ot, vb.as_ref());
        }

        if let (Some(ls), false) = (&self.lineage_store, ot.lineage.is_empty()) {
            if let Some(record) = &result.record {
                record_mutation_lineage(
                    ls.as_ref(),
                    object_name,
                    record,
                    &ot,
                    &actor.user_id,
                    &self.clock.now().to_rfc3339(),
                    &self.id_generator,
                );
            }
        }

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.metric_counter("lattice_mutate_total", "Total mutate calls", &[])
            .inc(1);
        self.metric_histogram("lattice_mutate_duration_ms", "Mutate latency", &[])
            .record(elapsed_ms);

        self.audit_and_event(
            actor,
            Operation::Mutate,
            "object_type",
            object_name,
            true,
            result.rows_affected.unwrap_or(1),
        )?;
        Ok(result)
    }

    /// Initiate a signed upload.
    #[tracing::instrument(skip(self, actor), err)]
    pub fn initiate_upload(
        &self,
        actor: &Actor,
        object_name: &ApiName,
        _format: &str,
        ttl: u64,
    ) -> Result<SignedUpload, Error> {
        self.authorize_with_decision(actor, Operation::Upload, "object_type", object_name)?;
        let store = self
            .object_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("upload"))?;
        let path = self.id_generator.new_id("upload");
        let upload = store.signed_upload_url(&path, ttl, &std::collections::BTreeMap::new())?;
        self.audit_and_event(
            actor,
            Operation::Upload,
            "object_type",
            object_name,
            true,
            0,
        )?;
        Ok(upload)
    }

    /// Rollback a bulk load.
    #[tracing::instrument(skip(self, actor), err)]
    pub fn rollback_upload(
        &self,
        actor: &Actor,
        object_name: &ApiName,
        load_id: &str,
    ) -> Result<(), Error> {
        self.authorize_with_decision(actor, Operation::Mutate, "object_type", object_name)?;
        let snap = self.ontology()?;
        let ot = snap
            .object_types
            .get(object_name)
            .cloned()
            .ok_or_else(|| Error::not_found("object_type", object_name))?;

        let backend = self.acquire_backend(&ot.source.datasource)?;
        let rollbacker = backend
            .as_ref()
            .as_rollbacker()
            .ok_or_else(|| Error::unsupported("rollback"))?;
        rollbacker.rollback(object_name, load_id)?;

        self.audit_and_event(
            actor,
            Operation::Mutate,
            "object_type",
            object_name,
            true,
            0,
        )?;
        Ok(())
    }

    /// Execute an ANN vector search.
    #[tracing::instrument(skip(self, actor, query), err)]
    pub fn vector_search(
        &self,
        actor: &Actor,
        query: VectorSearchQuery,
    ) -> Result<Vec<VectorResult>, Error> {
        self.authorize_with_decision(actor, Operation::Search, "object_type", &query.object_type)?;
        let vb = self
            .vector_backend
            .as_ref()
            .ok_or_else(|| Error::unsupported("vector_search"))?;
        let mut results = vb.vector_search(&query)?;

        let decision =
            self.evaluate_policy(actor, Operation::Search, "object_type", &query.object_type)?;
        if !decision.redactions.is_empty() {
            for vr in &mut results {
                apply_redactions(&mut vr.record, &decision.redactions);
            }
        }

        Ok(results)
    }

    /// Resolve a named object set and return its records.
    #[tracing::instrument(skip(self, actor), err)]
    pub fn resolve_object_set(&self, actor: &Actor, name: &ApiName) -> Result<Page, Error> {
        let snap = self.ontology()?;
        let os = snap
            .object_sets
            .get(name)
            .cloned()
            .ok_or_else(|| Error::not_found("object_set", name))?;

        let query = Query {
            filter: os.filter.clone(),
            limit: os.limit,
            sort: os
                .sort
                .iter()
                .map(|s| Sort {
                    property: s.property.clone(),
                    direction: match s.direction {
                        lattice_ir::SortDirection::Asc => "asc".to_string(),
                        lattice_ir::SortDirection::Desc => "desc".to_string(),
                    },
                })
                .collect(),
            ..Query::default()
        };

        self.search(actor, &os.object_type, query)
    }

    /// Compose multiple named object sets with a set operation.
    #[tracing::instrument(skip(self, actor, names), err)]
    pub fn compose_object_sets(
        &self,
        actor: &Actor,
        names: &[ApiName],
        op: lattice_ir::SetOp,
    ) -> Result<Page, Error> {
        let mut pages: Vec<Page> = Vec::with_capacity(names.len());
        for name in names {
            pages.push(self.resolve_object_set(actor, name)?);
        }

        let records = match op {
            lattice_ir::SetOp::Union => {
                let mut all = Vec::new();
                for p in pages {
                    all.extend(p.records);
                }
                all
            }
            lattice_ir::SetOp::Intersect => {
                if pages.is_empty() {
                    return Ok(Page {
                        records: Vec::new(),
                        next_cursor: None,
                    });
                }
                let mut base = pages.remove(0).records;
                for page in pages {
                    base.retain(|r| {
                        page.records
                            .iter()
                            .any(|pr| pr.primary_key == r.primary_key)
                    });
                }
                base
            }
            lattice_ir::SetOp::Subtract => {
                if pages.is_empty() {
                    return Ok(Page {
                        records: Vec::new(),
                        next_cursor: None,
                    });
                }
                let mut base = pages.remove(0).records;
                for page in pages {
                    base.retain(|r| {
                        !page
                            .records
                            .iter()
                            .any(|pr| pr.primary_key == r.primary_key)
                    });
                }
                base
            }
        };

        Ok(Page {
            records,
            next_cursor: None,
        })
    }

    /// Execute a named transform pipeline.
    #[tracing::instrument(skip(self, actor, pipeline_name), err)]
    pub fn execute_pipeline(
        &self,
        actor: &Actor,
        pipeline_name: &ApiName,
        mode: ExecutionMode,
    ) -> Result<PipelineResult, Error> {
        self.authorize_with_decision(actor, Operation::Execute, "pipeline", pipeline_name)?;
        let snap = self.ontology()?;
        let pipeline = snap
            .pipelines
            .get(pipeline_name)
            .cloned()
            .ok_or_else(|| Error::not_found("pipeline", pipeline_name))?;

        let executor = self
            .pipeline_executor
            .as_ref()
            .ok_or_else(|| Error::unsupported("pipeline_executor not configured"))?;
        let result = executor.execute(&pipeline, mode)?;

        self.audit_and_event(
            actor,
            Operation::Execute,
            "pipeline",
            pipeline_name,
            true,
            result.records_written,
        )?;
        Ok(result)
    }

    /// Return lineage edges connected to a record.
    #[tracing::instrument(skip(self, actor, pk), err)]
    pub fn get_lineage(
        &self,
        actor: &Actor,
        object_type: &ApiName,
        pk: &Value,
        depth: Option<u32>,
    ) -> Result<Vec<LineageRecord>, Error> {
        self.authorize_with_decision(actor, Operation::Read, "object_type", object_type)?;
        let ls = self
            .lineage_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("lineage_store not configured"))?;
        ls.query_lineage(object_type, pk, depth)
    }

    /// Execute a fan-out search across multiple datasources.
    #[tracing::instrument(skip(self, actor, queries), err)]
    pub fn cross_search(&self, actor: &Actor, queries: Vec<FederatedQuery>) -> Result<Page, Error> {
        if let Some(first) = queries.first() {
            self.authorize_with_decision(
                actor,
                Operation::Search,
                "object_type",
                &first.object_type,
            )?;
        }
        let fb = self
            .federated_backend
            .as_ref()
            .ok_or_else(|| Error::unsupported("federated_backend not configured"))?;
        fb.federated_search(&queries)
    }

    /// Execute an action.
    #[tracing::instrument(skip(self, actor, input), err)]
    pub fn execute_action(
        &self,
        actor: &Actor,
        action_name: &ApiName,
        input: Value,
    ) -> Result<lattice_ir::ActionResult, Error> {
        self.authorize_with_decision(actor, Operation::Execute, "action", action_name)?;
        let snap = self.ontology()?;
        let action = snap
            .actions
            .get(action_name)
            .cloned()
            .ok_or_else(|| Error::not_found("action", action_name))?;

        if action.risk_level.as_deref() == Some("high") {
            if let Some(approval) = self.approval_provider.as_deref() {
                let req = ApprovalRequest {
                    resource: action_name.to_string(),
                    actor: actor.clone(),
                    reason: "high-risk action".to_string(),
                };
                let decision = approval.request_approval(req)?;
                if !decision.approved {
                    return Err(Error::policy_denied("action denied by approval gate"));
                }
            }
        }

        let dispatcher = self
            .action_dispatcher
            .as_ref()
            .ok_or_else(|| Error::internal("no action dispatcher configured"))?;
        let handler = dispatcher
            .get_handler(action_name)
            .ok_or_else(|| Error::not_found("action_handler", action_name))?;

        let req = ActionRequest {
            action: action_name.clone(),
            input,
            actor: actor.clone(),
            run_id: Some(self.id_generator.new_id("act")),
        };

        let result = handler.execute(req)?;
        self.audit_and_event(
            actor,
            Operation::Execute,
            "action",
            action_name,
            result.status == "success",
            0,
        )?;
        Ok(result)
    }

    /// Start an agent run.
    #[tracing::instrument(skip(self, actor, input), err)]
    pub fn start_agent_run(
        &self,
        actor: &Actor,
        agent_name: &ApiName,
        input: Value,
    ) -> Result<String, Error> {
        self.authorize_with_decision(actor, Operation::Execute, "agent", agent_name)?;
        let snap = self.ontology()?;
        let agent = snap
            .agents
            .get(agent_name)
            .cloned()
            .ok_or_else(|| Error::not_found("agent", agent_name))?;

        let runtime = self
            .agent_runtime
            .as_ref()
            .ok_or_else(|| Error::internal("no agent runtime configured"))?;
        let run_id = runtime.start_run(&agent, input, actor)?;
        self.audit_and_event(actor, Operation::Execute, "agent", agent_name, true, 0)?;
        Ok(run_id)
    }

    /// Get the state of an agent run.
    #[tracing::instrument(skip(self, _actor), err)]
    pub fn get_agent_run(
        &self,
        _actor: &Actor,
        run_id: &str,
    ) -> Result<lattice_ir::AgentRun, Error> {
        let runtime = self
            .agent_runtime
            .as_ref()
            .ok_or_else(|| Error::internal("no agent runtime configured"))?;
        let mut run = runtime.get_run(run_id)?;

        if run.status == "completed" {
            if let Some(evaluator) = &self.agent_evaluator {
                let ctx = EvalContext::default();
                if let Ok(result) = evaluator.evaluate(&run, &ctx) {
                    run.eval_passed = Some(result.passed);
                    run.eval_score = result.score;
                    run.eval_notes = result.notes;
                }
            }
        }

        Ok(run)
    }
}
