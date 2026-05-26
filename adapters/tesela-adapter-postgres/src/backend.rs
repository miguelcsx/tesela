use sqlx_postgres::PgPool;
use std::sync::Arc;
use tesela_core::{ApiName, Error, Value};
use tesela_ir::{MutationResult, Page, Record};
use tesela_runtime::{
    ports::{Backend, Getter, Mutator, Searcher},
    query::{BackendCapabilities, Mutation, Query},
};

/// PostgreSQL-backed implementation of the Tesela `Backend` trait.
pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    /// Connect to a PostgreSQL database.
    pub async fn connect(url: &str) -> Result<Arc<Self>, Error> {
        let pool = PgPool::connect(url)
            .await
            .map_err(|e| Error::adapter(format!("postgres connect: {e}")))?;
        Ok(Arc::new(Self { pool }))
    }

    /// Create from an existing connection pool.
    pub fn from_pool(pool: PgPool) -> Arc<Self> {
        Arc::new(Self { pool })
    }

    /// Access the underlying connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl Backend for PostgresBackend {
    fn backend_type(&self) -> &str {
        "postgres"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            search: true,
            get: true,
            mutate: true,
            aggregate: true,
            traverse: true,
            bulk_load: true,
            rollback: false,
            explain: true,
        }
    }

    fn close(&self) -> Result<(), Error> {
        Ok(())
    }

    fn as_searcher(&self) -> Option<&dyn Searcher> {
        Some(self)
    }
    fn as_getter(&self) -> Option<&dyn Getter> {
        Some(self)
    }
    fn as_mutator(&self) -> Option<&dyn Mutator> {
        Some(self)
    }
    fn as_aggregator(&self) -> Option<&dyn tesela_runtime::ports::Aggregator> {
        None
    }
    fn as_traverser(&self) -> Option<&dyn tesela_runtime::ports::Traverser> {
        None
    }
    fn as_bulk_loader(&self) -> Option<&dyn tesela_runtime::ports::BulkLoader> {
        None
    }
    fn as_rollbacker(&self) -> Option<&dyn tesela_runtime::ports::Rollbacker> {
        None
    }
    fn as_explainer(&self) -> Option<&dyn tesela_runtime::ports::SearchExplainer> {
        None
    }
}

impl Searcher for PostgresBackend {
    fn search(&self, _object_type: &ApiName, _query: &Query) -> Result<Page, Error> {
        Err(Error::unsupported("postgres search not yet implemented"))
    }
}

impl Getter for PostgresBackend {
    fn get(&self, _object_type: &ApiName, _pk: &Value) -> Result<Option<Record>, Error> {
        Err(Error::unsupported("postgres get not yet implemented"))
    }
}

impl Mutator for PostgresBackend {
    fn mutate(
        &self,
        _object_type: &ApiName,
        _mutation: &Mutation,
    ) -> Result<MutationResult, Error> {
        Err(Error::unsupported("postgres mutate not yet implemented"))
    }
}
