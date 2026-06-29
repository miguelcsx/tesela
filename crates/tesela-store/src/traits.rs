//! Store, routing, policy, audit, and event traits.

use crate::{
    ActionRequest, AggregateQuery, PolicyDecision, PolicyRequest, Query, StoreCapabilities,
    TraversalQuery,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tesela_core::{ApiName, Error, Value, lock_read, lock_write};
use tesela_ir::{ActionResult, AggregateResult, MutationResult, Page, Record};

/// Backend contract for a datasource.
pub trait OntologyStore: Send + Sync {
    /// Store type name.
    fn store_type(&self) -> &str;
    /// Advertised capabilities.
    fn capabilities(&self) -> StoreCapabilities;
    /// Search records.
    fn search(&self, object_type: &ApiName, query: &Query) -> Result<Page, Error>;
    /// Fetch one record by primary key.
    fn get(&self, object_type: &ApiName, primary_key: &Value) -> Result<Option<Record>, Error>;
    /// Create one record.
    fn create(
        &self,
        object_type: &ApiName,
        values: std::collections::BTreeMap<ApiName, Value>,
    ) -> Result<MutationResult, Error>;
    /// Update one record.
    fn update(
        &self,
        object_type: &ApiName,
        primary_key: &Value,
        values: std::collections::BTreeMap<ApiName, Value>,
    ) -> Result<MutationResult, Error>;
    /// Delete one record.
    fn delete(&self, object_type: &ApiName, primary_key: &Value) -> Result<MutationResult, Error>;
    /// Execute an action.
    fn execute_action(&self, request: ActionRequest) -> Result<ActionResult, Error> {
        Err(Error::unsupported(format!(
            "execute_action {}",
            request.action
        )))
    }
    /// Aggregate records.
    fn aggregate(
        &self,
        object_type: &ApiName,
        query: &AggregateQuery,
    ) -> Result<AggregateResult, Error> {
        let _ = (object_type, query);
        Err(Error::unsupported("aggregate"))
    }
    /// Traverse a link.
    fn traverse(&self, link_type: &ApiName, query: &TraversalQuery) -> Result<Page, Error> {
        let _ = (link_type, query);
        Err(Error::unsupported("traverse"))
    }
    /// Clean up resources.
    fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}

/// Routes a datasource to its store.
pub trait StoreRouter: Send + Sync {
    /// Return the store for a datasource.
    fn store_for_datasource(&self, datasource: &ApiName) -> Result<Arc<dyn OntologyStore>, Error>;
}

/// Map-backed router for fixed platform wiring.
#[derive(Default)]
pub struct StaticStoreRouter {
    stores: RwLock<HashMap<ApiName, Arc<dyn OntologyStore>>>,
}

impl StaticStoreRouter {
    /// Create an empty router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a store for a datasource.
    pub fn register(
        &self,
        datasource: ApiName,
        store: Arc<dyn OntologyStore>,
    ) -> Result<(), Error> {
        lock_write(&self.stores)?.insert(datasource, store);
        Ok(())
    }
}

impl StoreRouter for StaticStoreRouter {
    fn store_for_datasource(&self, datasource: &ApiName) -> Result<Arc<dyn OntologyStore>, Error> {
        lock_read(&self.stores)?
            .get(datasource)
            .cloned()
            .ok_or_else(|| Error::not_found("datasource", datasource))
    }
}

/// Evaluates ontology policy.
pub trait PolicyEngine: Send + Sync {
    /// Evaluate a policy request.
    fn evaluate(&self, request: &PolicyRequest) -> Result<PolicyDecision, Error>;
}

/// Audit sink owned by the platform.
pub trait AuditSink: Send + Sync {
    /// Record an audited operation.
    fn record(&self, event: AuditEvent) -> Result<(), Error>;
}

/// Event bus owned by the platform.
pub trait EventBus: Send + Sync {
    /// Publish an ontology event.
    fn publish(&self, event: OntologyEvent) -> Result<(), Error>;
}

/// Minimal audit event emitted by the runtime.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Actor ID.
    pub actor_id: String,
    /// Operation.
    pub operation: String,
    /// Resource kind.
    pub resource_kind: String,
    /// Resource.
    pub resource: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Result count.
    pub result_count: i64,
}

/// Minimal ontology event emitted by the runtime.
#[derive(Debug, Clone)]
pub struct OntologyEvent {
    /// Operation kind.
    pub kind: String,
    /// Resource kind.
    pub resource_kind: String,
    /// Resource.
    pub resource: String,
    /// Actor ID.
    pub actor_id: String,
}

/// Policy engine that denies when no explicit platform policy is supplied.
pub struct DenyAllPolicy;

impl PolicyEngine for DenyAllPolicy {
    fn evaluate(&self, request: &PolicyRequest) -> Result<PolicyDecision, Error> {
        Ok(PolicyDecision {
            allow: false,
            reason: Some(format!(
                "no policy engine allowed {:?} on {}",
                request.operation, request.resource
            )),
            ..PolicyDecision::default()
        })
    }
}
