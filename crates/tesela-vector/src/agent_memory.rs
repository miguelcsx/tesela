//! Vector-backed semantic agent memory store.

use crate::Embedder;
use tesela_core::Error;
use tesela_runtime::ports::{AgentMemoryStore, VectorBackend, VectorSearchQuery};
use std::sync::Arc;

/// Object type name used for the vector memory index.
const MEMORY_OBJECT_TYPE: &str = "agent_memory";

/// Agent memory store that indexes memories as vectors for semantic recall.
///
/// On `remember`, the text is embedded and the vector is inserted into the
/// vector index.  On `search_memory`, the query is embedded and the top-k
/// nearest neighbours are returned as strings.
pub struct VectorAgentMemoryStore {
    vector_backend: Arc<dyn VectorBackend>,
    embedder: Arc<dyn Embedder>,
}

impl VectorAgentMemoryStore {
    /// Create a new store backed by the given vector index and embedder.
    pub fn new(vector_backend: Arc<dyn VectorBackend>, embedder: Arc<dyn Embedder>) -> Self {
        Self {
            vector_backend,
            embedder,
        }
    }

    fn memory_key(namespace: &str, key: &str) -> tesela_core::Value {
        tesela_core::Value(serde_json::Value::String(format!("{}:{}", namespace, key)))
    }

    fn memory_object_type() -> tesela_core::ApiName {
        tesela_core::ApiName::new_unchecked(MEMORY_OBJECT_TYPE)
    }
}

impl AgentMemoryStore for VectorAgentMemoryStore {
    fn remember(&self, namespace: &str, key: &str, value: &str) -> Result<(), Error> {
        let vector = self.embedder.embed(value)?;
        let pk = Self::memory_key(namespace, key);
        self.vector_backend
            .index_vector(&Self::memory_object_type(), &pk, &vector)
    }

    fn recall(&self, _namespace: &str, _key: &str) -> Result<Option<String>, Error> {
        // Point recall is not supported by the vector store — exact-key lookup
        // requires a scalar companion store.  Return None to indicate a miss.
        Ok(None)
    }

    fn search_memory(&self, _namespace: &str, query: &str) -> Result<Vec<String>, Error> {
        let vector = self.embedder.embed(query)?;
        let search = VectorSearchQuery {
            object_type: Self::memory_object_type(),
            query_vector: vector,
            top_k: 10,
            ef: 50,
            filter: None,
        };
        let results = self.vector_backend.vector_search(&search)?;
        Ok(results
            .into_iter()
            .filter_map(|r| {
                r.record
                    .values
                    .values()
                    .next()
                    .and_then(|v| v.as_str().map(String::from))
            })
            .collect())
    }

    fn forget(&self, namespace: &str, key: &str) -> Result<(), Error> {
        let pk = Self::memory_key(namespace, key);
        self.vector_backend
            .delete_vector(&Self::memory_object_type(), &pk)
    }
}
