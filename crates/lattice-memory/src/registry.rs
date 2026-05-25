//! DefaultBackendRegistry, MemoryBackendFactory, and capabilities helper.

use crate::backend::MemoryBackend;
use lattice_core::{ApiName, Error, Value};
use lattice_ir::Capabilities;
use lattice_runtime::{
    ports::{
        Aggregator, Backend, BackendFactory, BackendRegistry, BulkLoader, Getter, Mutator,
        Rollbacker, SearchExplainer, Searcher, Traverser,
    },
    query::BackendCapabilities,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

/// A default backend registry backed by a `RwLock<HashMap>`.
pub struct DefaultBackendRegistry {
    backends: RwLock<HashMap<ApiName, Arc<dyn Backend>>>,
}

impl DefaultBackendRegistry {
    /// Create an empty registry.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            backends: RwLock::new(HashMap::new()),
        })
    }

    /// Register a backend under a datasource name.
    pub fn register(&self, ds_name: ApiName, backend: Arc<dyn Backend>) {
        self.backends
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(ds_name, backend);
    }
}

impl Default for DefaultBackendRegistry {
    fn default() -> Self {
        Self {
            backends: RwLock::new(HashMap::new()),
        }
    }
}

impl BackendRegistry for DefaultBackendRegistry {
    fn acquire(&self, ds_name: &ApiName) -> Result<Box<dyn Backend>, Error> {
        let backends = self.backends.read().unwrap_or_else(|e| e.into_inner());
        let backend = backends
            .get(ds_name)
            .cloned()
            .ok_or_else(|| Error::not_found("datasource", ds_name))?;
        Ok(Box::new(ArcBackendWrapper(backend)))
    }

    fn register_factory(
        &self,
        _ds_name: ApiName,
        _factory: Box<dyn BackendFactory>,
    ) -> Result<(), Error> {
        Err(Error::unsupported("factory_registration"))
    }
}

struct ArcBackendWrapper(Arc<dyn Backend>);

impl Backend for ArcBackendWrapper {
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

/// Factory that opens a fresh in-memory backend for any datasource.
pub struct MemoryBackendFactory;

impl BackendFactory for MemoryBackendFactory {
    fn factory_type(&self) -> &str {
        "memory"
    }

    fn open(&self, _ds: &lattice_ir::Datasource) -> Result<Box<dyn Backend>, Error> {
        Ok(Box::new(ArcBackendWrapper(MemoryBackend::new())))
    }
}

/// Build a capabilities map advertising all memory backend features.
pub fn memory_capabilities() -> Capabilities {
    let mut values = BTreeMap::new();
    values.insert("backend".to_string(), Value::string("memory"));
    values.insert("search".to_string(), Value::bool(true));
    values.insert("get".to_string(), Value::bool(true));
    values.insert("mutate".to_string(), Value::bool(true));
    values.insert("aggregate".to_string(), Value::bool(true));
    values.insert("traverse".to_string(), Value::bool(true));
    values.insert("bulk_load".to_string(), Value::bool(true));
    values.insert("rollback".to_string(), Value::bool(true));
    values.insert("explain".to_string(), Value::bool(true));
    Capabilities { values }
}
