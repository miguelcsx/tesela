//! Core backend traits and default registry.

use crate::query::*;
use tesela_core::{ApiName, Error, Value};
use tesela_ir::{AggregateResult, Datasource, ExplainPlan, MutationResult, Page, Record};

/// Core backend trait.
pub trait Backend: Send + Sync {
    /// Backend type name.
    fn backend_type(&self) -> &str;
    /// Advertised capabilities.
    fn capabilities(&self) -> BackendCapabilities;
    /// Clean up resources.
    fn close(&self) -> Result<(), Error>;
    /// Cast to searcher capability.
    fn as_searcher(&self) -> Option<&dyn Searcher> {
        None
    }
    /// Cast to getter capability.
    fn as_getter(&self) -> Option<&dyn Getter> {
        None
    }
    /// Cast to mutator capability.
    fn as_mutator(&self) -> Option<&dyn Mutator> {
        None
    }
    /// Cast to aggregator capability.
    fn as_aggregator(&self) -> Option<&dyn Aggregator> {
        None
    }
    /// Cast to traverser capability.
    fn as_traverser(&self) -> Option<&dyn Traverser> {
        None
    }
    /// Cast to bulk loader capability.
    fn as_bulk_loader(&self) -> Option<&dyn BulkLoader> {
        None
    }
    /// Cast to rollbacker capability.
    fn as_rollbacker(&self) -> Option<&dyn Rollbacker> {
        None
    }
    /// Cast to explainer capability.
    fn as_explainer(&self) -> Option<&dyn SearchExplainer> {
        None
    }
}

/// Search capability.
pub trait Searcher: Backend {
    /// Execute a search query.
    fn search(&self, object_type: &ApiName, query: &Query) -> Result<Page, Error>;
}

/// Get-by-PK capability.
pub trait Getter: Backend {
    /// Fetch a single record by primary key.
    fn get(&self, object_type: &ApiName, pk: &Value) -> Result<Option<Record>, Error>;
}

/// Mutation capability.
pub trait Mutator: Backend {
    /// Apply a mutation.
    fn mutate(&self, object_type: &ApiName, mutation: &Mutation) -> Result<MutationResult, Error>;
}

/// Aggregation capability.
pub trait Aggregator: Backend {
    /// Execute an aggregate query.
    fn aggregate(
        &self,
        object_type: &ApiName,
        query: &AggregateQuery,
    ) -> Result<AggregateResult, Error>;
}

/// Link traversal capability.
pub trait Traverser: Backend {
    /// Traverse a link from a source record.
    fn traverse(&self, link_type: &ApiName, query: &TraversalQuery) -> Result<Page, Error>;
}

/// Bulk load capability.
pub trait BulkLoader: Backend {
    /// Bulk load records with a load ID for rollback.
    fn bulk_load(
        &self,
        object_type: &ApiName,
        records: Vec<Record>,
        load_id: &str,
    ) -> Result<i64, Error>;
}

/// Rollback capability.
pub trait Rollbacker: Backend {
    /// Rollback a bulk load by load ID.
    fn rollback(&self, object_type: &ApiName, load_id: &str) -> Result<(), Error>;
}

/// Explain capability.
pub trait SearchExplainer: Backend {
    /// Return an explain plan for a search query.
    fn explain_search(&self, object_type: &ApiName, query: &Query) -> Result<ExplainPlan, Error>;
}

/// Factory that opens a backend for a datasource.
pub trait BackendFactory: Send + Sync {
    /// Factory type name.
    fn factory_type(&self) -> &str;
    /// Open a backend for the given datasource config.
    fn open(&self, ds: &Datasource) -> Result<Box<dyn Backend>, Error>;
}

/// Registry of open backends keyed by datasource name.
pub trait BackendRegistry: Send + Sync {
    /// Get or open the backend for a datasource.
    fn acquire(&self, ds_name: &ApiName) -> Result<Box<dyn Backend>, Error>;
    /// Register a factory for a datasource.
    fn register_factory(
        &self,
        ds_name: ApiName,
        factory: Box<dyn BackendFactory>,
    ) -> Result<(), Error>;
}

/// A simple `HashMap`-backed [`BackendRegistry`].
pub struct DefaultBackendRegistry {
    backends: std::sync::RwLock<std::collections::HashMap<ApiName, std::sync::Arc<dyn Backend>>>,
}

impl DefaultBackendRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            backends: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Register a pre-opened backend under `ds_name`.
    pub fn register(&self, ds_name: ApiName, backend: std::sync::Arc<dyn Backend>) {
        if let Ok(mut map) = self.backends.write() {
            map.insert(ds_name, backend);
        }
    }
}

impl Default for DefaultBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendRegistry for DefaultBackendRegistry {
    fn acquire(&self, ds_name: &ApiName) -> Result<Box<dyn Backend>, Error> {
        self.backends
            .read()
            .map_err(|_| Error::internal("backend registry lock poisoned"))?
            .get(ds_name)
            .map(|arc| Box::new(ArcBackendRef(arc.clone())) as Box<dyn Backend>)
            .ok_or_else(|| Error::not_found("datasource", ds_name))
    }

    fn register_factory(
        &self,
        _ds_name: ApiName,
        _factory: Box<dyn BackendFactory>,
    ) -> Result<(), Error> {
        Err(Error::unsupported(
            "factory registration — use register() directly",
        ))
    }
}

struct ArcBackendRef(std::sync::Arc<dyn Backend>);

impl Backend for ArcBackendRef {
    fn backend_type(&self) -> &str {
        self.0.backend_type()
    }
    fn capabilities(&self) -> BackendCapabilities {
        self.0.capabilities()
    }
    fn close(&self) -> Result<(), Error> {
        self.0.close()
    }
    fn as_searcher(&self) -> Option<&dyn Searcher> {
        self.0.as_searcher()
    }
    fn as_getter(&self) -> Option<&dyn Getter> {
        self.0.as_getter()
    }
    fn as_mutator(&self) -> Option<&dyn Mutator> {
        self.0.as_mutator()
    }
    fn as_aggregator(&self) -> Option<&dyn Aggregator> {
        self.0.as_aggregator()
    }
    fn as_traverser(&self) -> Option<&dyn Traverser> {
        self.0.as_traverser()
    }
    fn as_bulk_loader(&self) -> Option<&dyn BulkLoader> {
        self.0.as_bulk_loader()
    }
    fn as_rollbacker(&self) -> Option<&dyn Rollbacker> {
        self.0.as_rollbacker()
    }
    fn as_explainer(&self) -> Option<&dyn SearchExplainer> {
        self.0.as_explainer()
    }
}
