//! Main ontology runtime.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tesela_core::{ApiName, Error, Operation, Value, lock_read, lock_write};
use tesela_ir::{AggregateResult, MutationResult, ObjectSet, Page, Record, Spec};
use tesela_store::{
    Actor, AggregateQuery, AuditEvent, AuditSink, EventBus, Mutation, OntologyEvent, OntologyStore,
    PolicyDecision, PolicyEngine, PolicyRequest, Query, StaticStoreRouter, StoreRouter,
    TraversalQuery,
};

use crate::AllowAllPolicy;

/// Options used to construct a [`Runtime`].
#[derive(Default)]
pub struct RuntimeOptions {
    /// Store router.
    pub store_router: Option<Arc<dyn StoreRouter>>,
    /// Policy engine.
    pub policy_engine: Option<Arc<dyn PolicyEngine>>,
    /// Optional audit sink.
    pub audit_sink: Option<Arc<dyn AuditSink>>,
    /// Optional event bus.
    pub event_bus: Option<Arc<dyn EventBus>>,
    /// Maximum rows a search may return.
    pub max_query_limit: Option<i32>,
}

impl RuntimeOptions {
    /// Local development/test options.
    pub fn dev() -> Self {
        Self {
            policy_engine: Some(Arc::new(AllowAllPolicy)),
            ..Self::default()
        }
    }
}

/// Immutable ontology indexes.
struct OntologySnapshot {
    spec: Arc<Spec>,
    object_types: HashMap<ApiName, Arc<tesela_ir::ObjectType>>,
    links: HashMap<ApiName, Arc<tesela_ir::LinkType>>,
    object_sets: HashMap<ApiName, Arc<ObjectSet>>,
}

impl OntologySnapshot {
    fn build(spec: Spec) -> Self {
        Self {
            object_types: spec
                .object_types
                .iter()
                .map(|item| (item.api_name.clone(), Arc::new(item.clone())))
                .collect(),
            links: spec
                .link_types
                .iter()
                .map(|item| (item.api_name.clone(), Arc::new(item.clone())))
                .collect(),
            object_sets: spec
                .object_sets
                .iter()
                .map(|item| (item.api_name.clone(), Arc::new(item.clone())))
                .collect(),
            spec: Arc::new(spec),
        }
    }
}

/// Shareable ontology handle for transports and agent harnesses.
#[derive(Clone)]
pub struct OntologyHandle {
    runtime: Arc<Runtime>,
}

impl OntologyHandle {
    /// Wrap a runtime.
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }

    /// Borrow the underlying runtime.
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }
}

/// Tesela ontology runtime.
pub struct Runtime {
    ontology: RwLock<Arc<OntologySnapshot>>,
    store_router: Arc<dyn StoreRouter>,
    policy_engine: Arc<dyn PolicyEngine>,
    audit_sink: Option<Arc<dyn AuditSink>>,
    event_bus: Option<Arc<dyn EventBus>>,
    max_query_limit: i32,
}

impl Runtime {
    /// Create a runtime from a spec and platform-provided ports.
    pub fn new(spec: Spec, options: RuntimeOptions) -> Result<Arc<Self>, Error> {
        let store_router = match options.store_router {
            Some(router) => router,
            None => Arc::new(StaticStoreRouter::new()),
        };
        let policy_engine = options
            .policy_engine
            .ok_or_else(|| Error::validation("policy_engine is required"))?;
        Ok(Arc::new(Self {
            ontology: RwLock::new(Arc::new(OntologySnapshot::build(spec))),
            store_router,
            policy_engine,
            audit_sink: options.audit_sink,
            event_bus: options.event_bus,
            max_query_limit: max_query_limit(options.max_query_limit),
        }))
    }

    /// Return a clone of the active spec.
    pub fn spec(&self) -> Result<Spec, Error> {
        Ok(lock_read(&self.ontology)?.spec.as_ref().clone())
    }

    /// Atomically replace the active spec.
    pub fn apply_spec(&self, spec: Spec) -> Result<(), Error> {
        *lock_write(&self.ontology)? = Arc::new(OntologySnapshot::build(spec));
        Ok(())
    }

    /// Search records of an object type.
    pub fn search(
        &self,
        actor: &Actor,
        object_name: &ApiName,
        mut query: Query,
    ) -> Result<Page, Error> {
        let decision = self.authorize(actor, Operation::Search, "object_type", object_name)?;
        let requested_limit = match query.limit {
            Some(limit) => limit,
            None => self.max_query_limit,
        };
        if requested_limit > self.max_query_limit {
            query.limit = Some(self.max_query_limit);
        }
        if let Some(row_filter) = decision.row_filter {
            query = query.and_filter(row_filter);
        }
        let mut page = self
            .store_for_object(object_name)?
            .search(object_name, &query)?;
        redact(&mut page.records, &decision.redactions);
        self.emit(
            actor,
            "search",
            "object_type",
            object_name,
            true,
            page.records.len() as i64,
        )?;
        Ok(page)
    }

    /// Get one record by primary key.
    pub fn get(&self, actor: &Actor, object_name: &ApiName, pk: &Value) -> Result<Record, Error> {
        let decision = self.authorize(actor, Operation::Read, "object_type", object_name)?;
        let mut record = self
            .store_for_object(object_name)?
            .get(object_name, pk)?
            .ok_or_else(|| Error::not_found("record", pk))?;
        redact(std::slice::from_mut(&mut record), &decision.redactions);
        self.emit(actor, "get", "object_type", object_name, true, 1)?;
        Ok(record)
    }

    /// Apply a mutation to an object type.
    pub fn mutate(
        &self,
        actor: &Actor,
        object_name: &ApiName,
        mutation: Mutation,
    ) -> Result<MutationResult, Error> {
        self.authorize(actor, Operation::Mutate, "object_type", object_name)?;
        let store = self.store_for_object(object_name)?;
        let primary_key = self.primary_key_for_object(object_name)?;
        let result = apply_mutation(store.as_ref(), object_name, &primary_key, mutation)?;
        self.emit(
            actor,
            "mutate",
            "object_type",
            object_name,
            true,
            rows_affected_count(result.rows_affected),
        )?;
        Ok(result)
    }

    /// Aggregate records of an object type.
    pub fn aggregate(
        &self,
        actor: &Actor,
        object_name: &ApiName,
        query: AggregateQuery,
    ) -> Result<AggregateResult, Error> {
        self.authorize(actor, Operation::Aggregate, "object_type", object_name)?;
        let result = self
            .store_for_object(object_name)?
            .aggregate(object_name, &query)?;
        self.emit(
            actor,
            "aggregate",
            "object_type",
            object_name,
            true,
            result.groups.len() as i64,
        )?;
        Ok(result)
    }

    /// Traverse a link type.
    pub fn traverse(
        &self,
        actor: &Actor,
        link_name: &ApiName,
        query: TraversalQuery,
    ) -> Result<Page, Error> {
        self.authorize(actor, Operation::Traverse, "link_type", link_name)?;
        let snapshot = self.snapshot()?;
        let link = snapshot
            .links
            .get(link_name)
            .ok_or_else(|| Error::not_found("link_type", link_name))?;
        let datasource = link
            .source
            .as_ref()
            .and_then(|source| source.datasource.as_ref())
            .cloned()
            .or_else(|| {
                snapshot
                    .object_types
                    .get(&link.to)
                    .map(|object_type| object_type.source.datasource.clone())
            })
            .ok_or_else(|| Error::validation(format!("link '{link_name}' has no datasource")))?;
        let page = self
            .store_router
            .store_for_datasource(&datasource)?
            .traverse(link_name, &query)?;
        self.emit(
            actor,
            "traverse",
            "link_type",
            link_name,
            true,
            page.records.len() as i64,
        )?;
        Ok(page)
    }

    /// Resolve a named object set.
    pub fn resolve_object_set(&self, actor: &Actor, name: &ApiName) -> Result<Page, Error> {
        let object_set = self
            .snapshot()?
            .object_sets
            .get(name)
            .cloned()
            .ok_or_else(|| Error::not_found("object_set", name))?;
        self.search(
            actor,
            &object_set.object_type,
            Query {
                filter: object_set.filter.clone(),
                sort: object_set
                    .sort
                    .iter()
                    .map(|item| tesela_store::Sort {
                        property: item.property.clone(),
                        direction: match item.direction {
                            tesela_ir::SortDirection::Asc => tesela_store::SortDirection::Asc,
                            tesela_ir::SortDirection::Desc => tesela_store::SortDirection::Desc,
                        },
                    })
                    .collect(),
                limit: object_set.limit,
                ..Query::default()
            },
        )
    }

    /// Compose named object sets.
    pub fn compose_object_sets(
        &self,
        actor: &Actor,
        names: &[ApiName],
        op: tesela_ir::SetOp,
    ) -> Result<Page, Error> {
        let mut pages = Vec::with_capacity(names.len());
        for name in names {
            pages.push(self.resolve_object_set(actor, name)?);
        }
        let records = compose_pages(pages, op);
        Ok(Page {
            records,
            next_cursor: None,
        })
    }

    fn snapshot(&self) -> Result<Arc<OntologySnapshot>, Error> {
        let guard = lock_read(&self.ontology)?;
        Ok(Arc::clone(&guard))
    }

    fn store_for_object(&self, object_name: &ApiName) -> Result<Arc<dyn OntologyStore>, Error> {
        let snapshot = self.snapshot()?;
        let object_type = snapshot
            .object_types
            .get(object_name)
            .ok_or_else(|| Error::not_found("object_type", object_name))?;
        self.store_router
            .store_for_datasource(&object_type.source.datasource)
    }

    fn primary_key_for_object(&self, object_name: &ApiName) -> Result<ApiName, Error> {
        let snapshot = self.snapshot()?;
        snapshot
            .object_types
            .get(object_name)
            .map(|object_type| object_type.primary_key.clone())
            .ok_or_else(|| Error::not_found("object_type", object_name))
    }

    fn authorize(
        &self,
        actor: &Actor,
        operation: Operation,
        resource_kind: &str,
        resource: &ApiName,
    ) -> Result<PolicyDecision, Error> {
        let decision = self.policy_engine.evaluate(&PolicyRequest {
            actor: actor.clone(),
            operation,
            resource_kind: resource_kind.to_string(),
            resource: resource.clone(),
            context: Default::default(),
        })?;
        if decision.allow {
            Ok(decision)
        } else {
            Err(Error::policy_denied(match decision.reason {
                Some(reason) => reason,
                None => "policy denied".to_string(),
            }))
        }
    }

    fn emit(
        &self,
        actor: &Actor,
        operation: &str,
        resource_kind: &str,
        resource: &ApiName,
        success: bool,
        result_count: i64,
    ) -> Result<(), Error> {
        if let Some(audit_sink) = &self.audit_sink {
            audit_sink.record(AuditEvent {
                actor_id: actor.user_id.clone(),
                operation: operation.to_string(),
                resource_kind: resource_kind.to_string(),
                resource: resource.to_string(),
                success,
                result_count,
            })?;
        }
        if let Some(event_bus) = &self.event_bus {
            event_bus.publish(OntologyEvent {
                kind: operation.to_string(),
                resource_kind: resource_kind.to_string(),
                resource: resource.to_string(),
                actor_id: actor.user_id.clone(),
            })?;
        }
        Ok(())
    }
}

fn apply_mutation(
    store: &dyn OntologyStore,
    object_name: &ApiName,
    primary_key_name: &ApiName,
    mutation: Mutation,
) -> Result<MutationResult, Error> {
    match mutation {
        Mutation::Create { values } => store.create(object_name, values),
        Mutation::Update {
            primary_key,
            values,
        } => store.update(object_name, &primary_key, values),
        Mutation::Delete { primary_key } => store.delete(object_name, &primary_key),
        Mutation::Upsert { values } => {
            let primary_key = values.get(primary_key_name).cloned().ok_or_else(|| {
                Error::bad_request(format!(
                    "upsert for '{}' requires primary key '{}'",
                    object_name, primary_key_name
                ))
            })?;
            match store.get(object_name, &primary_key)? {
                Some(_) => store.update(object_name, &primary_key, values),
                None => store.create(object_name, values),
            }
        }
        Mutation::Batch { items } => {
            let mut affected = 0i64;
            for item in items {
                let rows_affected =
                    apply_mutation(store, object_name, primary_key_name, item)?.rows_affected;
                affected += rows_affected_count(rows_affected);
            }
            Ok(MutationResult {
                record: None,
                rows_affected: Some(affected),
            })
        }
    }
}

fn max_query_limit(limit: Option<i32>) -> i32 {
    if let Some(limit) = limit {
        return limit;
    }
    1000
}

fn rows_affected_count(rows_affected: Option<i64>) -> i64 {
    if let Some(count) = rows_affected {
        return count;
    }
    0
}

fn redact(records: &mut [Record], redactions: &[ApiName]) {
    for record in records {
        for property in redactions {
            record.values.remove(property);
        }
    }
}

fn compose_pages(pages: Vec<Page>, op: tesela_ir::SetOp) -> Vec<Record> {
    match op {
        tesela_ir::SetOp::Union => pages.into_iter().flat_map(|page| page.records).collect(),
        tesela_ir::SetOp::Intersect => intersect_pages(pages),
        tesela_ir::SetOp::Subtract => subtract_pages(pages),
    }
}

fn intersect_pages(mut pages: Vec<Page>) -> Vec<Record> {
    if pages.is_empty() {
        return Vec::new();
    }
    let mut base = pages.remove(0).records;
    for page in pages {
        base.retain(|record| {
            page.records
                .iter()
                .any(|candidate| candidate.primary_key == record.primary_key)
        });
    }
    base
}

fn subtract_pages(mut pages: Vec<Page>) -> Vec<Record> {
    if pages.is_empty() {
        return Vec::new();
    }
    let mut base = pages.remove(0).records;
    for page in pages {
        base.retain(|record| {
            !page
                .records
                .iter()
                .any(|candidate| candidate.primary_key == record.primary_key)
        });
    }
    base
}
