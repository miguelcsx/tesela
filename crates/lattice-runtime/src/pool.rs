//! Multi-workspace runtime pool.

use crate::ports::WorkspaceFactory;
use crate::Runtime;
use lattice_core::Error;
use lattice_ir;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A pool of [`Runtime`] instances keyed by workspace ID.
///
/// Enables multi-tenant deployments where a single Lattice process serves
/// multiple isolated workspaces.  The HTTP server layer extracts the workspace
/// ID from the request (e.g., the `X-Workspace-Id` header or a URL prefix)
/// and delegates to the appropriate runtime.
pub struct RuntimePool {
    runtimes: RwLock<HashMap<String, Arc<Runtime>>>,
    factory: Option<Arc<dyn WorkspaceFactory>>,
}

impl RuntimePool {
    /// Create an empty pool without an automatic factory.
    pub fn new() -> Self {
        Self {
            runtimes: RwLock::new(HashMap::new()),
            factory: None,
        }
    }

    /// Create a pool backed by a factory that can instantiate workspaces on demand.
    pub fn with_factory(factory: Arc<dyn WorkspaceFactory>) -> Self {
        Self {
            runtimes: RwLock::new(HashMap::new()),
            factory: Some(factory),
        }
    }

    /// Register a pre-built runtime under `workspace_id`.
    pub fn register(&self, workspace_id: &str, runtime: Arc<Runtime>) -> Result<(), Error> {
        self.runtimes
            .write()
            .map_err(|_| Error::internal("runtime pool lock poisoned"))?
            .insert(workspace_id.to_string(), runtime);
        Ok(())
    }

    /// Look up the runtime for `workspace_id`.
    ///
    /// Returns `None` when the workspace is not registered and no factory is
    /// configured; returns `Err` when the factory fails to instantiate.
    ///
    /// When a factory *is* configured and the workspace is unknown, the factory
    /// is called with an empty `Spec` and the resulting runtime is cached for
    /// future calls.
    pub fn get(&self, workspace_id: &str) -> Result<Option<Arc<Runtime>>, Error> {
        {
            let map = self
                .runtimes
                .read()
                .map_err(|_| Error::internal("runtime pool lock poisoned"))?;
            if let Some(rt) = map.get(workspace_id) {
                return Ok(Some(rt.clone()));
            }
        }

        // On-demand creation via factory, if configured.
        if let Some(factory) = &self.factory {
            let rt = factory.create(workspace_id, lattice_ir::Spec::default())?;
            self.runtimes
                .write()
                .map_err(|_| Error::internal("runtime pool lock poisoned"))?
                .insert(workspace_id.to_string(), rt.clone());
            return Ok(Some(rt));
        }

        Ok(None)
    }

    /// Remove and return the runtime for `workspace_id`, if present.
    pub fn remove(&self, workspace_id: &str) -> Result<Option<Arc<Runtime>>, Error> {
        Ok(self
            .runtimes
            .write()
            .map_err(|_| Error::internal("runtime pool lock poisoned"))?
            .remove(workspace_id))
    }

    /// List all registered workspace IDs.
    pub fn workspace_ids(&self) -> Result<Vec<String>, Error> {
        Ok(self
            .runtimes
            .read()
            .map_err(|_| Error::internal("runtime pool lock poisoned"))?
            .keys()
            .cloned()
            .collect())
    }
}

impl Default for RuntimePool {
    fn default() -> Self {
        Self::new()
    }
}
