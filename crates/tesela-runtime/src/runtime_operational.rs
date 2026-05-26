//! Runtime operations for artifacts, capabilities, events, and jobs.

use crate::query::*;
use crate::runtime::Runtime;
use tesela_core::{ApiName, Error, Operation, Value};
use std::collections::BTreeMap;

impl Runtime {
    /// Issue a constrained capability from a declared grant.
    #[tracing::instrument(skip(self, actor, constraints), err)]
    pub fn issue_capability(
        &self,
        actor: &Actor,
        grant_name: &ApiName,
        constraints: BTreeMap<String, Value>,
    ) -> Result<CapabilityToken, Error> {
        self.authorize_with_decision(actor, Operation::Execute, "capability_grant", grant_name)?;
        let snap = self.ontology()?;
        let grant = snap
            .capability_grants
            .get(grant_name)
            .cloned()
            .ok_or_else(|| Error::not_found("capability_grant", grant_name))?;
        let issuer = self
            .capability_issuer
            .as_ref()
            .ok_or_else(|| Error::unsupported("capability_issuer not configured"))?;
        let token = issuer.issue_capability(&grant, actor, constraints)?;
        self.audit_and_event(
            actor,
            Operation::Execute,
            "capability_grant",
            grant_name,
            true,
            1,
        )?;
        Ok(token)
    }

    /// Verify an opaque capability token.
    #[tracing::instrument(skip(self, token), err)]
    pub fn verify_capability(&self, token: &str) -> Result<CapabilityToken, Error> {
        self.capability_issuer
            .as_ref()
            .ok_or_else(|| Error::unsupported("capability_issuer not configured"))?
            .verify_capability(token)
    }

    /// Revoke a previously issued capability token.
    #[tracing::instrument(skip(self, actor), err)]
    pub fn revoke_capability(&self, actor: &Actor, token_id: &str) -> Result<(), Error> {
        let resource = ApiName::new_unchecked("capability");
        self.authorize_with_decision(actor, Operation::Execute, "capability_grant", &resource)?;
        self.capability_issuer
            .as_ref()
            .ok_or_else(|| Error::unsupported("capability_issuer not configured"))?
            .revoke_capability(token_id)?;
        self.audit_and_event(
            actor,
            Operation::Execute,
            "capability_grant",
            &resource,
            true,
            1,
        )
    }

    /// Authorize and obtain a read locator for an artifact.
    #[tracing::instrument(skip(self, actor, params), err)]
    pub fn authorize_artifact_read(
        &self,
        actor: &Actor,
        artifact_name: &ApiName,
        params: BTreeMap<String, Value>,
        ttl: u64,
    ) -> Result<ArtifactLocator, Error> {
        self.authorize_artifact_read_with_context(actor, artifact_name, params, ttl, None, None)
    }

    /// Authorize and obtain a read locator for an artifact with request/capability context.
    #[tracing::instrument(skip(self, actor, params, request_meta), err)]
    pub fn authorize_artifact_read_with_context(
        &self,
        actor: &Actor,
        artifact_name: &ApiName,
        params: BTreeMap<String, Value>,
        ttl: u64,
        request_meta: Option<RequestMeta>,
        capability_token: Option<String>,
    ) -> Result<ArtifactLocator, Error> {
        let capability = match capability_token {
            Some(token) => Some(self.verify_capability(&token)?),
            None => None,
        };
        self.authorize_request(PolicyRequest {
            actor: actor.clone(),
            operation: Operation::Read,
            resource_kind: "artifact_type".to_string(),
            resource: artifact_name.clone(),
            context: BTreeMap::new(),
            resource_instance: Some(ResourceContext {
                id: params.get("id").cloned(),
                values: params.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                relationships: BTreeMap::new(),
            }),
            request_meta,
            capability,
            operation_params: params.clone(),
        })?;
        let snap = self.ontology()?;
        let artifact = snap
            .artifact_types
            .get(artifact_name)
            .cloned()
            .ok_or_else(|| Error::not_found("artifact_type", artifact_name))?;
        let path = render_template(&artifact.path_template, &params)?;
        let mut store_params = params.clone();
        store_params.insert(
            "_store".to_string(),
            Value::string(artifact.store.to_string()),
        );
        let store = self
            .object_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("object_store not configured"))?;
        let mut locator = store.signed_read_url(&path, ttl, &store_params)?;
        if locator.media_type.is_none() {
            locator.media_type.clone_from(&artifact.media_type);
        }
        self.audit_and_event(
            actor,
            Operation::Read,
            "artifact_type",
            artifact_name,
            true,
            1,
        )?;
        Ok(locator)
    }

    /// Initiate an upload through a declared upload flow.
    #[tracing::instrument(skip(self, actor, params), err)]
    pub fn initiate_upload_flow(
        &self,
        actor: &Actor,
        flow_name: &ApiName,
        params: BTreeMap<String, Value>,
        ttl: u64,
    ) -> Result<SignedUpload, Error> {
        self.initiate_upload_flow_with_context(actor, flow_name, params, ttl, None, None)
    }

    /// Initiate an upload through a declared upload flow with request/capability context.
    #[tracing::instrument(skip(self, actor, params, request_meta), err)]
    pub fn initiate_upload_flow_with_context(
        &self,
        actor: &Actor,
        flow_name: &ApiName,
        params: BTreeMap<String, Value>,
        ttl: u64,
        request_meta: Option<RequestMeta>,
        capability_token: Option<String>,
    ) -> Result<SignedUpload, Error> {
        let capability = match capability_token {
            Some(token) => Some(self.verify_capability(&token)?),
            None => None,
        };
        self.authorize_request(PolicyRequest {
            actor: actor.clone(),
            operation: Operation::Upload,
            resource_kind: "upload_flow".to_string(),
            resource: flow_name.clone(),
            context: BTreeMap::new(),
            resource_instance: Some(ResourceContext {
                id: params.get("id").cloned(),
                values: params.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                relationships: BTreeMap::new(),
            }),
            request_meta,
            capability,
            operation_params: params.clone(),
        })?;
        let snap = self.ontology()?;
        let flow = snap
            .upload_flows
            .get(flow_name)
            .cloned()
            .ok_or_else(|| Error::not_found("upload_flow", flow_name))?;
        let path = render_template(&flow.path_template, &params)?;
        let mut store_params = params.clone();
        store_params.insert("_store".to_string(), Value::string(flow.store.to_string()));
        let store = self
            .object_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("object_store not configured"))?;
        let upload = store.signed_upload_url(&path, ttl, &store_params)?;
        self.audit_and_event(actor, Operation::Upload, "upload_flow", flow_name, true, 0)?;
        Ok(upload)
    }

    /// Complete an upload flow by validating that the object exists and returning metadata.
    #[tracing::instrument(skip(self, actor), err)]
    pub fn complete_upload_flow(
        &self,
        actor: &Actor,
        flow_name: &ApiName,
        path: &str,
    ) -> Result<ObjectMetadata, Error> {
        self.authorize_with_decision(actor, Operation::Upload, "upload_flow", flow_name)?;
        let snap = self.ontology()?;
        let _flow = snap
            .upload_flows
            .get(flow_name)
            .cloned()
            .ok_or_else(|| Error::not_found("upload_flow", flow_name))?;
        let store = self
            .object_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("object_store not configured"))?;
        let metadata = store.stat(path)?;
        self.audit_and_event(actor, Operation::Upload, "upload_flow", flow_name, true, 1)?;
        Ok(metadata)
    }

    /// Load records through the target backend declared by an upload flow.
    #[tracing::instrument(skip(self, actor, records), err)]
    pub fn load_upload_flow_records(
        &self,
        actor: &Actor,
        flow_name: &ApiName,
        records: Vec<tesela_ir::Record>,
        load_id: Option<String>,
    ) -> Result<tesela_ir::UploadResult, Error> {
        self.authorize_with_decision(actor, Operation::Upload, "upload_flow", flow_name)?;
        let snap = self.ontology()?;
        let flow = snap
            .upload_flows
            .get(flow_name)
            .cloned()
            .ok_or_else(|| Error::not_found("upload_flow", flow_name))?;
        let object_type = flow
            .target_object_type
            .as_ref()
            .ok_or_else(|| Error::validation("upload_flow has no target_object_type"))?;
        let ot = snap
            .object_types
            .get(object_type)
            .cloned()
            .ok_or_else(|| Error::not_found("object_type", object_type))?;
        let backend = self.acquire_backend(&ot.source.datasource)?;
        let loader = backend
            .as_ref()
            .as_bulk_loader()
            .ok_or_else(|| Error::unsupported("bulk_load"))?;
        let load_id = load_id.unwrap_or_else(|| self.id_generator.new_id("load"));
        let rows_loaded = loader.bulk_load(object_type, records, &load_id)?;
        self.audit_and_event(
            actor,
            Operation::Upload,
            "upload_flow",
            flow_name,
            true,
            rows_loaded,
        )?;
        Ok(tesela_ir::UploadResult {
            run_id: None,
            load_id: Some(load_id),
            rows_loaded,
            rows_skipped: 0,
            skipped_rows: Vec::new(),
            quality: Vec::new(),
        })
    }

    /// Roll back records loaded through an upload flow.
    #[tracing::instrument(skip(self, actor), err)]
    pub fn rollback_upload_flow(
        &self,
        actor: &Actor,
        flow_name: &ApiName,
        load_id: &str,
    ) -> Result<(), Error> {
        self.authorize_with_decision(actor, Operation::Upload, "upload_flow", flow_name)?;
        let snap = self.ontology()?;
        let flow = snap
            .upload_flows
            .get(flow_name)
            .cloned()
            .ok_or_else(|| Error::not_found("upload_flow", flow_name))?;
        let object_type = flow
            .target_object_type
            .as_ref()
            .ok_or_else(|| Error::validation("upload_flow has no target_object_type"))?;
        let ot = snap
            .object_types
            .get(object_type)
            .cloned()
            .ok_or_else(|| Error::not_found("object_type", object_type))?;
        let backend = self.acquire_backend(&ot.source.datasource)?;
        let rollbacker = backend
            .as_ref()
            .as_rollbacker()
            .ok_or_else(|| Error::unsupported("rollback"))?;
        rollbacker.rollback(object_type, load_id)?;
        self.audit_and_event(actor, Operation::Upload, "upload_flow", flow_name, true, 0)
    }

    /// Start a declared asynchronous job.
    #[tracing::instrument(skip(self, actor, input), err)]
    pub fn start_job(
        &self,
        actor: &Actor,
        job_name: &ApiName,
        input: BTreeMap<String, Value>,
        idempotency_key: Option<String>,
    ) -> Result<RunRecord, Error> {
        self.authorize_request(PolicyRequest {
            actor: actor.clone(),
            operation: Operation::Execute,
            resource_kind: "job_type".to_string(),
            resource: job_name.clone(),
            context: BTreeMap::new(),
            resource_instance: None,
            request_meta: None,
            capability: None,
            operation_params: input.clone(),
        })?;
        let snap = self.ontology()?;
        let job = snap
            .job_types
            .get(job_name)
            .cloned()
            .ok_or_else(|| Error::not_found("job_type", job_name))?;
        let now = self.clock.now().to_rfc3339();
        let run = RunRecord {
            id: self.id_generator.new_id("run"),
            kind: "job".to_string(),
            resource: job_name.clone(),
            status: job
                .states
                .first()
                .cloned()
                .unwrap_or_else(|| "queued".to_string()),
            actor_user_id: actor.user_id.clone(),
            idempotency_key,
            correlation_id: Some(self.id_generator.new_id("corr")),
            input,
            output: BTreeMap::new(),
            steps: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        };
        let run = match &self.run_store {
            Some(store) => store.create_or_reuse(run)?,
            None => run,
        };
        let work = WorkItem {
            kind: "job".to_string(),
            job_type: Some(job_name.clone()),
            run_id: Some(run.id.clone()),
            idempotency_key: run.idempotency_key.clone(),
            correlation_id: run.correlation_id.clone(),
            payload: run.input.clone(),
        };
        let _message_id = if let Some(queue) = &self.message_bus {
            if let Some(event_name) = &job.start_event {
                let event = Event {
                    id: self.id_generator.new_id("evt"),
                    kind: "job_started".to_string(),
                    workspace: snap.spec.workspace.api_name.to_string(),
                    object_type: None,
                    actor_user_id: actor.user_id.clone(),
                    occurred_at: self.clock.now().to_rfc3339(),
                    payload: run.input.clone(),
                    event_type: Some(event_name.clone()),
                    topic: None,
                    correlation_id: run.correlation_id.clone(),
                    causation_id: Some(run.id.clone()),
                };
                Some(queue.publish_message(event_name, event)?)
            } else {
                None
            }
        } else {
            None
        };
        let _ = work;
        self.audit_and_event(actor, Operation::Execute, "job_type", job_name, true, 1)?;
        Ok(run)
    }

    /// Get a run record.
    #[tracing::instrument(skip(self), err)]
    pub fn get_run(&self, run_id: &str) -> Result<RunRecord, Error> {
        self.run_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("run_store not configured"))?
            .get_run(run_id)?
            .ok_or_else(|| Error::not_found("run", run_id))
    }

    /// Publish a declared event.
    #[tracing::instrument(skip(self, actor, payload), err)]
    pub fn publish_event_type(
        &self,
        actor: &Actor,
        event_name: &ApiName,
        payload: BTreeMap<String, Value>,
        correlation_id: Option<String>,
    ) -> Result<String, Error> {
        self.authorize_with_decision(actor, Operation::Execute, "event_type", event_name)?;
        let snap = self.ontology()?;
        let event_type = snap
            .event_types
            .get(event_name)
            .cloned()
            .ok_or_else(|| Error::not_found("event_type", event_name))?;
        let event = Event {
            id: self.id_generator.new_id("evt"),
            kind: event_name.to_string(),
            workspace: snap.spec.workspace.api_name.to_string(),
            object_type: None,
            actor_user_id: actor.user_id.clone(),
            occurred_at: self.clock.now().to_rfc3339(),
            payload,
            event_type: Some(event_name.clone()),
            topic: Some(event_type.topic.clone()),
            correlation_id,
            causation_id: None,
        };
        let id = self
            .message_bus
            .as_ref()
            .ok_or_else(|| Error::unsupported("message_bus not configured"))?
            .publish_message(event_name, event)?;
        self.audit_and_event(actor, Operation::Execute, "event_type", event_name, true, 1)?;
        Ok(id)
    }

    /// Resolve a named aggregate view and execute it through the normal aggregate path.
    #[tracing::instrument(skip(self, actor), err)]
    pub fn aggregate_view(
        &self,
        actor: &Actor,
        view_name: &ApiName,
    ) -> Result<tesela_ir::AggregateResult, Error> {
        let snap = self.ontology()?;
        let view = snap
            .aggregate_views
            .get(view_name)
            .cloned()
            .ok_or_else(|| Error::not_found("aggregate_view", view_name))?;
        let query = AggregateQuery {
            filter: view.filter.clone(),
            group_by: view.group_by.clone(),
            aggregations: view
                .measures
                .iter()
                .map(|m| Aggregation {
                    function: match m.function {
                        tesela_ir::AggregateFunction::Count => "count",
                        tesela_ir::AggregateFunction::Sum => "sum",
                        tesela_ir::AggregateFunction::Avg => "avg",
                        tesela_ir::AggregateFunction::Min => "min",
                        tesela_ir::AggregateFunction::Max => "max",
                        tesela_ir::AggregateFunction::CountDistinct => "count_distinct",
                        tesela_ir::AggregateFunction::SpatialExtent => "spatial_extent",
                    }
                    .to_string(),
                    property: m.property.clone(),
                    alias: m.alias.clone(),
                })
                .collect(),
            time_bucket: view.time_bucket.clone(),
            spatial_extent: view.spatial_extent.clone(),
            require_pushdown: view.require_pushdown,
        };
        if view.require_pushdown && self.query_planner.is_none() {
            return Err(Error::unsupported(
                "aggregate_view requires pushdown but no query_planner is configured",
            ));
        }
        self.aggregate(actor, &view.object_type, query)
    }
}

fn render_template(template: &str, params: &BTreeMap<String, Value>) -> Result<String, Error> {
    let mut rendered = template.to_string();
    for (key, value) in params {
        let value = match value.as_str() {
            Some(s) => s.to_string(),
            None => value.to_string(),
        };
        rendered = rendered.replace(&format!("{{{}}}", key), &value);
    }
    if rendered.contains('{') || rendered.contains('}') {
        return Err(Error::validation(format!(
            "unresolved template placeholders in '{}'",
            template
        )));
    }
    Ok(rendered)
}
