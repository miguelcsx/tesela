//! Main Lattice runtime engine.

use crate::config::ConfigSource;
use crate::crypto::Sealer;
use crate::ports::*;
use crate::query::Actor;
use crate::ratelimit::RateLimiter;
use crate::runtime_internal::{DefaultIdGenerator, SystemClock};
use lattice_core::{lock_read, ApiName, Error};
use lattice_ir::{ObjectSet, Spec, TransformPipeline};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ---------------------------------------------------------------------------
// RuntimeOptions
// ---------------------------------------------------------------------------

/// Options used to construct a [`Runtime`].
#[derive(Default)]
pub struct RuntimeOptions {
    /// Allow development/test-only defaults for missing policy and audit ports.
    ///
    /// Production runtimes should leave this false and wire explicit policy and
    /// audit implementations. This flag exists so examples and unit tests can
    /// opt into local-only behavior without hiding missing governance wiring.
    pub allow_dev_defaults: bool,
    /// Backend registry.
    pub backend_registry: Option<Arc<dyn BackendRegistry>>,
    /// Policy evaluator.
    pub policy_evaluator: Option<Arc<dyn PolicyEvaluator>>,
    /// Audit sink.
    pub audit_sink: Option<Arc<dyn AuditSink>>,
    /// Event bus.
    pub event_bus: Option<Arc<dyn EventBus>>,
    /// Action dispatcher.
    pub action_dispatcher: Option<Arc<dyn ActionDispatcher>>,
    /// Agent runtime.
    pub agent_runtime: Option<Arc<dyn AgentRuntime>>,
    /// ID generator.
    pub id_generator: Option<Arc<dyn IdGenerator>>,
    /// Clock.
    pub clock: Option<Arc<dyn Clock>>,
    /// Object store for signed upload URLs.
    pub object_store: Option<Arc<dyn ObjectStore>>,
    /// Meta store for persisting spec versions.
    pub meta_store: Option<Arc<dyn MetaStore>>,
    /// Maximum rows a single search may return.
    pub max_query_limit: Option<i32>,
    /// Approval provider for high-risk or flagged actions.
    pub approval_provider: Option<Arc<dyn ApprovalProvider>>,
    /// CDC / streaming source.
    pub change_stream_source: Option<Arc<dyn ChangeStreamSource>>,
    /// Agent evaluator called after every completed agent run.
    pub agent_evaluator: Option<Arc<dyn AgentEvaluator>>,
    /// Obligation executor for policy side-effects.
    pub obligation_executor: Option<Arc<dyn ObligationExecutor>>,
    /// Evaluator for computed property expressions.
    pub computed_evaluator: Option<Arc<dyn ComputedEvaluator>>,
    /// Validator for quality rules declared on object types.
    pub quality_rule_evaluator: Option<Arc<dyn QualityRuleEvaluator>>,
    /// ANN vector search backend.
    pub vector_backend: Option<Arc<dyn VectorBackend>>,
    /// Runtime data-lineage store.
    pub lineage_store: Option<Arc<dyn LineageStore>>,
    /// Schema migration executor.
    pub migration_executor: Option<Arc<dyn MigrationExecutor>>,
    /// Branch / draft-spec store.
    pub branch_store: Option<Arc<dyn BranchStore>>,
    /// Query planner for aggregate push-down.
    pub query_planner: Option<Arc<dyn QueryPlanner>>,
    /// Message bus for logical events.
    pub message_bus: Option<Arc<dyn MessageBus>>,
    /// Run store for actions, jobs, and uploads.
    pub run_store: Option<Arc<dyn RunStore>>,
    /// Capability issuer/verifier.
    pub capability_issuer: Option<Arc<dyn CapabilityIssuer>>,
    /// Federated search backend.
    pub federated_backend: Option<Arc<dyn FederatedBackend>>,
    /// Transform pipeline executor.
    pub pipeline_executor: Option<Arc<dyn PipelineExecutor>>,
    /// Subscription bus for real-time push / SSE.
    pub subscription_bus: Option<Arc<dyn SubscriptionBus>>,
    /// Per-actor rate limiter (checked at the top of every operation).
    pub rate_limiter: Option<Arc<dyn RateLimiter>>,
    /// Symmetric encryption sealer for field-level encryption.
    pub sealer: Option<Arc<dyn Sealer>>,
    /// Runtime configuration source.
    pub config_source: Option<Arc<dyn ConfigSource>>,
    /// Metrics registry for counters, histograms, gauges.
    pub metrics_registry: Option<Arc<dyn lattice_telemetry::MetricsRegistry>>,
}

impl RuntimeOptions {
    /// Build options for tests and examples that intentionally run without
    /// production policy/audit infrastructure.
    pub fn dev() -> Self {
        Self {
            allow_dev_defaults: true,
            ..Self::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

/// Immutable snapshot of all ontology indexes, swapped atomically on `apply_spec`.
pub(crate) struct OntologySnapshot {
    pub spec: Arc<Spec>,
    pub object_types: HashMap<ApiName, Arc<lattice_ir::ObjectType>>,
    pub datasources: HashMap<ApiName, Arc<lattice_ir::Datasource>>,
    pub actions: HashMap<ApiName, Arc<lattice_ir::ActionType>>,
    pub agents: HashMap<ApiName, Arc<lattice_ir::Agent>>,
    pub policies: HashMap<ApiName, Arc<lattice_ir::PolicyRule>>,
    pub links: HashMap<ApiName, Arc<lattice_ir::LinkType>>,
    pub roles: HashMap<ApiName, Arc<lattice_ir::Role>>,
    pub object_sets: HashMap<ApiName, Arc<ObjectSet>>,
    pub pipelines: HashMap<ApiName, Arc<TransformPipeline>>,
    pub artifact_types: HashMap<ApiName, Arc<lattice_ir::ArtifactType>>,
    pub upload_flows: HashMap<ApiName, Arc<lattice_ir::UploadFlow>>,
    pub job_types: HashMap<ApiName, Arc<lattice_ir::JobType>>,
    pub event_types: HashMap<ApiName, Arc<lattice_ir::EventType>>,
    pub capability_grants: HashMap<ApiName, Arc<lattice_ir::CapabilityGrant>>,
    pub aggregate_views: HashMap<ApiName, Arc<lattice_ir::AggregateView>>,
}

impl OntologySnapshot {
    pub(crate) fn build(spec: Spec) -> Self {
        Self {
            object_types: Runtime::index_object_types(&spec),
            datasources: Runtime::index_datasources(&spec),
            actions: Runtime::index_actions(&spec),
            agents: Runtime::index_agents(&spec),
            policies: Runtime::index_policies(&spec),
            links: Runtime::index_links(&spec),
            roles: Runtime::index_roles(&spec),
            object_sets: Runtime::index_object_sets(&spec),
            pipelines: Runtime::index_pipelines(&spec),
            artifact_types: Runtime::index_artifact_types(&spec),
            upload_flows: Runtime::index_upload_flows(&spec),
            job_types: Runtime::index_job_types(&spec),
            event_types: Runtime::index_event_types(&spec),
            capability_grants: Runtime::index_capability_grants(&spec),
            aggregate_views: Runtime::index_aggregate_views(&spec),
            spec: Arc::new(spec),
        }
    }
}

/// The main Lattice runtime.
///
/// All ontology indexes live behind a single `RwLock<Arc<OntologySnapshot>>`
/// so that `apply_spec` swaps every index atomically. Per-request reads clone
/// the `Arc` (8 bytes) then access the snapshot without contention.
pub struct Runtime {
    pub(crate) ontology: RwLock<Arc<OntologySnapshot>>,

    pub(crate) backend_registry: Option<Arc<dyn BackendRegistry>>,
    pub(crate) policy_evaluator: Option<Arc<dyn PolicyEvaluator>>,
    pub(crate) audit_sink: Arc<dyn AuditSink>,
    pub(crate) event_bus: Arc<dyn EventBus>,
    pub(crate) action_dispatcher: Option<Arc<dyn ActionDispatcher>>,
    pub(crate) agent_runtime: Option<Arc<dyn AgentRuntime>>,
    pub(crate) id_generator: Arc<dyn IdGenerator>,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) object_store: Option<Arc<dyn ObjectStore>>,
    pub(crate) meta_store: Option<Arc<dyn MetaStore>>,
    pub(crate) max_query_limit: i32,
    pub(crate) approval_provider: Option<Arc<dyn ApprovalProvider>>,
    pub(crate) change_stream_source: Option<Arc<dyn ChangeStreamSource>>,
    pub(crate) agent_evaluator: Option<Arc<dyn AgentEvaluator>>,
    pub(crate) obligation_executor: Option<Arc<dyn ObligationExecutor>>,
    pub(crate) computed_evaluator: Option<Arc<dyn ComputedEvaluator>>,
    pub(crate) quality_rule_evaluator: Option<Arc<dyn QualityRuleEvaluator>>,
    pub(crate) vector_backend: Option<Arc<dyn VectorBackend>>,
    pub(crate) lineage_store: Option<Arc<dyn LineageStore>>,
    pub(crate) migration_executor: Option<Arc<dyn MigrationExecutor>>,
    pub(crate) branch_store: Option<Arc<dyn BranchStore>>,
    pub(crate) query_planner: Option<Arc<dyn QueryPlanner>>,
    pub(crate) message_bus: Option<Arc<dyn MessageBus>>,
    pub(crate) run_store: Option<Arc<dyn RunStore>>,
    pub(crate) capability_issuer: Option<Arc<dyn CapabilityIssuer>>,
    pub(crate) federated_backend: Option<Arc<dyn FederatedBackend>>,
    pub(crate) pipeline_executor: Option<Arc<dyn PipelineExecutor>>,
    pub(crate) subscription_bus: Option<Arc<dyn SubscriptionBus>>,
    pub(crate) rate_limiter: Option<Arc<dyn RateLimiter>>,
    pub(crate) sealer: Option<Arc<dyn Sealer>>,
    pub(crate) config_source: Option<Arc<dyn ConfigSource>>,
    pub(crate) metrics_registry: Option<Arc<dyn lattice_telemetry::MetricsRegistry>>,
}

impl Runtime {
    /// Create a new runtime from a compiled spec and options.
    pub fn new(spec: Spec, opts: RuntimeOptions) -> Result<Arc<Self>, Error> {
        if opts.allow_dev_defaults {
            tracing::warn!(
                "Runtime created with allow_dev_defaults=true; \
                 policy and audit are disabled — do not use in production"
            );
        }

        if opts.audit_sink.is_none() && !opts.allow_dev_defaults {
            return Err(Error::validation(
                "audit_sink is required; use RuntimeOptions::dev() only for local tests/examples",
            ));
        }
        if opts.policy_evaluator.is_none() && !opts.allow_dev_defaults {
            return Err(Error::validation(
                "policy_evaluator is required; use RuntimeOptions::dev() only for local tests/examples",
            ));
        }

        let snapshot = OntologySnapshot::build(spec);

        let rt = Arc::new(Self {
            ontology: RwLock::new(Arc::new(snapshot)),
            backend_registry: opts.backend_registry,
            policy_evaluator: opts.policy_evaluator,
            audit_sink: opts
                .audit_sink
                .unwrap_or_else(|| Arc::new(crate::audit::NoopAuditSink)),
            event_bus: opts
                .event_bus
                .unwrap_or_else(|| Arc::new(crate::events::NoopEventBus)),
            action_dispatcher: opts.action_dispatcher,
            agent_runtime: opts.agent_runtime,
            id_generator: opts
                .id_generator
                .unwrap_or_else(|| Arc::new(DefaultIdGenerator)),
            clock: opts.clock.unwrap_or_else(|| Arc::new(SystemClock)),
            object_store: opts.object_store,
            meta_store: opts.meta_store,
            max_query_limit: opts.max_query_limit.unwrap_or(1000),
            approval_provider: opts.approval_provider,
            change_stream_source: opts.change_stream_source,
            agent_evaluator: opts.agent_evaluator,
            obligation_executor: opts.obligation_executor,
            computed_evaluator: opts.computed_evaluator,
            quality_rule_evaluator: opts.quality_rule_evaluator,
            vector_backend: opts.vector_backend,
            lineage_store: opts.lineage_store,
            migration_executor: opts.migration_executor,
            branch_store: opts.branch_store,
            query_planner: opts.query_planner,
            message_bus: opts.message_bus,
            run_store: opts.run_store,
            capability_issuer: opts.capability_issuer,
            federated_backend: opts.federated_backend,
            pipeline_executor: opts.pipeline_executor,
            subscription_bus: opts.subscription_bus,
            rate_limiter: opts.rate_limiter,
            sealer: opts.sealer,
            config_source: opts.config_source,
            metrics_registry: opts.metrics_registry,
        });
        Ok(rt)
    }

    // -----------------------------------------------------------------------
    // Ontology snapshot accessor
    // -----------------------------------------------------------------------

    /// Cheaply clone the current ontology snapshot (`Arc` bump only).
    pub(crate) fn ontology(&self) -> Result<Arc<OntologySnapshot>, Error> {
        Ok(Arc::clone(&*lock_read(&self.ontology)?))
    }

    // -----------------------------------------------------------------------
    // Index builders
    // -----------------------------------------------------------------------

    pub(crate) fn index_object_types(spec: &Spec) -> HashMap<ApiName, Arc<lattice_ir::ObjectType>> {
        spec.object_types
            .iter()
            .map(|ot| (ot.api_name.clone(), Arc::new(ot.clone())))
            .collect()
    }

    pub(crate) fn index_datasources(spec: &Spec) -> HashMap<ApiName, Arc<lattice_ir::Datasource>> {
        spec.datasources
            .iter()
            .map(|ds| (ds.api_name.clone(), Arc::new(ds.clone())))
            .collect()
    }

    pub(crate) fn index_actions(spec: &Spec) -> HashMap<ApiName, Arc<lattice_ir::ActionType>> {
        spec.actions
            .iter()
            .map(|a| (a.api_name.clone(), Arc::new(a.clone())))
            .collect()
    }

    pub(crate) fn index_agents(spec: &Spec) -> HashMap<ApiName, Arc<lattice_ir::Agent>> {
        spec.agents
            .iter()
            .map(|a| (a.api_name.clone(), Arc::new(a.clone())))
            .collect()
    }

    pub(crate) fn index_policies(spec: &Spec) -> HashMap<ApiName, Arc<lattice_ir::PolicyRule>> {
        spec.policies
            .iter()
            .map(|p| (p.api_name.clone(), Arc::new(p.clone())))
            .collect()
    }

    pub(crate) fn index_links(spec: &Spec) -> HashMap<ApiName, Arc<lattice_ir::LinkType>> {
        spec.link_types
            .iter()
            .map(|l| (l.api_name.clone(), Arc::new(l.clone())))
            .collect()
    }

    pub(crate) fn index_roles(spec: &Spec) -> HashMap<ApiName, Arc<lattice_ir::Role>> {
        spec.roles
            .iter()
            .map(|r| (r.api_name.clone(), Arc::new(r.clone())))
            .collect()
    }

    pub(crate) fn index_object_sets(spec: &Spec) -> HashMap<ApiName, Arc<ObjectSet>> {
        spec.object_sets
            .iter()
            .map(|os| (os.api_name.clone(), Arc::new(os.clone())))
            .collect()
    }

    pub(crate) fn index_pipelines(spec: &Spec) -> HashMap<ApiName, Arc<TransformPipeline>> {
        spec.pipelines
            .iter()
            .map(|p| (p.api_name.clone(), Arc::new(p.clone())))
            .collect()
    }

    pub(crate) fn index_artifact_types(
        spec: &Spec,
    ) -> HashMap<ApiName, Arc<lattice_ir::ArtifactType>> {
        spec.artifact_types
            .iter()
            .map(|a| (a.api_name.clone(), Arc::new(a.clone())))
            .collect()
    }

    pub(crate) fn index_upload_flows(spec: &Spec) -> HashMap<ApiName, Arc<lattice_ir::UploadFlow>> {
        spec.upload_flows
            .iter()
            .map(|u| (u.api_name.clone(), Arc::new(u.clone())))
            .collect()
    }

    pub(crate) fn index_job_types(spec: &Spec) -> HashMap<ApiName, Arc<lattice_ir::JobType>> {
        spec.job_types
            .iter()
            .map(|j| (j.api_name.clone(), Arc::new(j.clone())))
            .collect()
    }

    pub(crate) fn index_event_types(spec: &Spec) -> HashMap<ApiName, Arc<lattice_ir::EventType>> {
        spec.event_types
            .iter()
            .map(|e| (e.api_name.clone(), Arc::new(e.clone())))
            .collect()
    }

    pub(crate) fn index_capability_grants(
        spec: &Spec,
    ) -> HashMap<ApiName, Arc<lattice_ir::CapabilityGrant>> {
        spec.capability_grants
            .iter()
            .map(|c| (c.api_name.clone(), Arc::new(c.clone())))
            .collect()
    }

    pub(crate) fn index_aggregate_views(
        spec: &Spec,
    ) -> HashMap<ApiName, Arc<lattice_ir::AggregateView>> {
        spec.aggregate_views
            .iter()
            .map(|a| (a.api_name.clone(), Arc::new(a.clone())))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Rate-limit enforcement
    // -----------------------------------------------------------------------

    pub(crate) fn check_rate_limit(&self, actor: &Actor, namespace: &str) -> Result<(), Error> {
        if let Some(rl) = &self.rate_limiter {
            if !rl.allow(namespace, &actor.user_id)? {
                return Err(crate::ratelimit::rate_limited_error(
                    namespace,
                    &actor.user_id,
                ));
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Config accessor
    // -----------------------------------------------------------------------

    /// Access the runtime configuration source.
    pub fn config(&self) -> Option<&dyn ConfigSource> {
        self.config_source.as_deref()
    }

    /// Access the sealer for field-level encryption / decryption.
    pub fn sealer(&self) -> Option<&dyn Sealer> {
        self.sealer.as_deref()
    }

    /// Get a counter from the metrics registry (or a no-op counter if none configured).
    pub(crate) fn metric_counter(
        &self,
        name: &str,
        desc: &str,
        labels: &[(String, String)],
    ) -> std::sync::Arc<dyn lattice_telemetry::Counter> {
        self.metrics_registry
            .as_ref()
            .map(|r| r.counter(name, desc, labels))
            .unwrap_or_else(|| std::sync::Arc::new(lattice_telemetry::NoopCounter))
    }

    /// Get a histogram from the metrics registry (or a no-op histogram if none configured).
    pub(crate) fn metric_histogram(
        &self,
        name: &str,
        desc: &str,
        labels: &[(String, String)],
    ) -> std::sync::Arc<dyn lattice_telemetry::Histogram> {
        self.metrics_registry
            .as_ref()
            .map(|r| r.histogram(name, desc, labels))
            .unwrap_or_else(|| std::sync::Arc::new(lattice_telemetry::NoopHistogram))
    }
}
