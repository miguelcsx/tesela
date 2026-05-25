//! Branch management, ontology lifecycle, and system introspection.

use crate::query::Actor;
use crate::runtime::Runtime;
use crate::runtime_internal::{lock_r, lock_w};
use lattice_core::{ApiName, Error, Value};
use lattice_graph::{GraphBuilder, SchemaGraph};
use lattice_ir::{Branch, BranchStatus, Capabilities, HealthStatus, Spec};
use std::collections::BTreeMap;
use std::sync::Arc;

impl Runtime {
    /// Create a new draft branch from the current spec.
    #[tracing::instrument(skip(self, actor), err)]
    pub fn create_branch(&self, actor: &Actor, label: &str) -> Result<Branch, Error> {
        let bs = self
            .branch_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("branch_store not configured"))?;
        let spec = Arc::clone(&*lock_r(&self.spec)?);
        bs.create_branch(&spec, label, &actor.user_id)
    }

    /// Update the draft spec on an open branch.
    #[tracing::instrument(skip(self, _actor, spec), err)]
    pub fn update_branch_spec(
        &self,
        _actor: &Actor,
        branch_id: &str,
        spec: Spec,
    ) -> Result<(), Error> {
        let bs = self
            .branch_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("branch_store not configured"))?;
        bs.update_draft(branch_id, spec)
    }

    /// Merge a branch into the live spec.
    #[tracing::instrument(skip(self, _actor), err)]
    pub fn merge_branch(
        &self,
        _actor: &Actor,
        branch_id: &str,
    ) -> Result<lattice_compiler::Diff, Error> {
        let bs = self
            .branch_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("branch_store not configured"))?;

        let branch = bs
            .get_branch(branch_id)?
            .ok_or_else(|| Error::not_found("branch", branch_id))?;

        if branch.status != BranchStatus::Draft && branch.status != BranchStatus::Review {
            return Err(Error::validation(format!(
                "branch '{}' cannot be merged (status: {:?})",
                branch_id, branch.status
            )));
        }

        let diff = self.apply_spec(branch.draft_spec)?;
        bs.set_status(branch_id, BranchStatus::Merged)?;
        Ok(diff)
    }

    /// List all branches.
    #[tracing::instrument(skip(self))]
    pub fn list_branches(&self) -> Result<Vec<Branch>, Error> {
        let bs = self
            .branch_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("branch_store not configured"))?;
        bs.list_branches()
    }

    /// Discard (delete) a branch.
    #[tracing::instrument(skip(self, _actor), err)]
    pub fn discard_branch(&self, _actor: &Actor, branch_id: &str) -> Result<(), Error> {
        let bs = self
            .branch_store
            .as_ref()
            .ok_or_else(|| Error::unsupported("branch_store not configured"))?;
        bs.set_status(branch_id, BranchStatus::Discarded)?;
        bs.delete_branch(branch_id)
    }

    /// Apply a new spec, returning the diff.
    #[tracing::instrument(skip(self, new_spec), err)]
    pub fn apply_spec(&self, new_spec: Spec) -> Result<lattice_compiler::Diff, Error> {
        let old_spec = Arc::clone(&*lock_r(&self.spec)?);
        let diff = lattice_compiler::compute_diff(&old_spec, &new_spec);

        let new_ot = Self::index_object_types(&new_spec);
        let new_ds = Self::index_datasources(&new_spec);
        let new_actions = Self::index_actions(&new_spec);
        let new_agents = Self::index_agents(&new_spec);
        let new_policies = Self::index_policies(&new_spec);
        let new_links = Self::index_links(&new_spec);
        let new_roles = Self::index_roles(&new_spec);
        let new_object_sets = Self::index_object_sets(&new_spec);
        let new_pipelines = Self::index_pipelines(&new_spec);
        let new_artifact_types = Self::index_artifact_types(&new_spec);
        let new_upload_flows = Self::index_upload_flows(&new_spec);
        let new_job_types = Self::index_job_types(&new_spec);
        let new_event_types = Self::index_event_types(&new_spec);
        let new_capability_grants = Self::index_capability_grants(&new_spec);
        let new_aggregate_views = Self::index_aggregate_views(&new_spec);

        if let Some(meta) = &self.meta_store {
            let hash = lattice_compiler::hash_spec(&new_spec);
            meta.store_spec(&new_spec, &hash)?;
        }

        *lock_w(&self.spec)? = Arc::new(new_spec);
        *lock_w(&self.object_types)? = new_ot;
        *lock_w(&self.datasources)? = new_ds;
        *lock_w(&self.actions)? = new_actions;
        *lock_w(&self.agents)? = new_agents;
        *lock_w(&self.policies)? = new_policies;
        *lock_w(&self.links)? = new_links;
        *lock_w(&self.roles)? = new_roles;
        *lock_w(&self.object_sets)? = new_object_sets;
        *lock_w(&self.pipelines)? = new_pipelines;
        *lock_w(&self.artifact_types)? = new_artifact_types;
        *lock_w(&self.upload_flows)? = new_upload_flows;
        *lock_w(&self.job_types)? = new_job_types;
        *lock_w(&self.event_types)? = new_event_types;
        *lock_w(&self.capability_grants)? = new_capability_grants;
        *lock_w(&self.aggregate_views)? = new_aggregate_views;

        Ok(diff)
    }

    /// Apply a new spec with optional migration execution.
    #[tracing::instrument(skip(self, new_spec), err)]
    pub fn apply_spec_with_migration(
        &self,
        new_spec: Spec,
    ) -> Result<lattice_compiler::Diff, Error> {
        if let Some(me) = &self.migration_executor {
            let old_spec = Arc::clone(&*lock_r(&self.spec)?);
            let diff = lattice_compiler::compute_diff(&old_spec, &new_spec);
            let plan = me.plan(&diff)?;

            if let Some(registry) = &self.backend_registry {
                for ds in &new_spec.datasources {
                    if let Ok(backend) = registry.acquire(&ds.api_name) {
                        let _ = me.execute(&plan, backend.as_ref());
                    }
                }
            }
        }
        self.apply_spec(new_spec)
    }

    /// Return a cloned snapshot of the current spec.
    #[tracing::instrument(skip(self), err)]
    pub fn spec(&self) -> Result<Spec, Error> {
        Ok(Spec::clone(&*lock_r(&self.spec)?))
    }

    /// Build the schema graph for the current spec.
    #[tracing::instrument(skip(self), err)]
    pub fn schema_graph(&self) -> Result<SchemaGraph, Error> {
        let spec = Arc::clone(&*lock_r(&self.spec)?);
        Ok(GraphBuilder::build(&spec))
    }

    /// Health check.
    #[tracing::instrument(skip(self), err)]
    pub fn health(&self) -> Result<HealthStatus, Error> {
        let spec = self.spec()?;
        Ok(HealthStatus {
            status: "healthy".to_string(),
            spec_version: spec.version.to_string(),
            workspace: spec.workspace.api_name.to_string(),
        })
    }

    /// Capability advertisement.
    #[tracing::instrument(skip(self))]
    pub fn capabilities(&self) -> Capabilities {
        let mut values = BTreeMap::new();
        values.insert("search".to_string(), Value::from(true));
        values.insert("get".to_string(), Value::from(true));
        values.insert("mutate".to_string(), Value::from(true));
        values.insert("aggregate".to_string(), Value::from(true));
        values.insert("traverse".to_string(), Value::from(true));
        values.insert(
            "vector_search".to_string(),
            Value::from(self.vector_backend.is_some()),
        );
        values.insert(
            "federated_search".to_string(),
            Value::from(self.federated_backend.is_some()),
        );
        values.insert(
            "lineage".to_string(),
            Value::from(self.lineage_store.is_some()),
        );
        values.insert(
            "branches".to_string(),
            Value::from(self.branch_store.is_some()),
        );
        values.insert(
            "pipelines".to_string(),
            Value::from(self.pipeline_executor.is_some()),
        );
        values.insert(
            "actions".to_string(),
            Value::from(self.action_dispatcher.is_some()),
        );
        values.insert(
            "agents".to_string(),
            Value::from(self.agent_runtime.is_some()),
        );
        values.insert(
            "upload".to_string(),
            Value::from(self.object_store.is_some()),
        );
        values.insert(
            "artifacts".to_string(),
            Value::from(self.object_store.is_some()),
        );
        values.insert(
            "message_bus".to_string(),
            Value::from(self.message_bus.is_some()),
        );
        values.insert(
            "run_store".to_string(),
            Value::from(self.run_store.is_some()),
        );
        values.insert(
            "capability_issuer".to_string(),
            Value::from(self.capability_issuer.is_some()),
        );
        Capabilities { values }
    }

    /// Subscribe to real-time domain events for `object_type`.
    #[tracing::instrument(skip(self), err)]
    pub fn subscribe(
        &self,
        object_type: Option<&ApiName>,
    ) -> Result<std::sync::mpsc::Receiver<crate::query::Event>, Error> {
        self.subscription_bus
            .as_ref()
            .ok_or_else(|| Error::unsupported("subscription_bus not configured"))?
            .subscribe(object_type)
    }

    /// Subscribe to change events from the configured CDC source.
    #[tracing::instrument(skip(self), err)]
    pub fn subscribe_changes(
        &self,
        object_type: &ApiName,
    ) -> Result<std::sync::mpsc::Receiver<crate::ports::ChangeEvent>, Error> {
        self.change_stream_source
            .as_ref()
            .ok_or_else(|| Error::unsupported("change_stream_source"))?
            .subscribe(object_type)
    }
}
