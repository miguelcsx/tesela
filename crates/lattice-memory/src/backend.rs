//! MemoryBackend struct, Backend, Searcher, Getter, and Mutator implementations.

use crate::{apply_sort, filter, resolve_offset};
use lattice_core::{lock_read, lock_write, ApiName, Error, Value};
use lattice_ir::{MutationResult, Page, Record, Spec};
use lattice_runtime::{
    ports::{Backend, Getter, Mutator, Searcher},
    query::{BackendCapabilities, Mutation, Query},
};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

/// A thread-safe, in-memory implementation of the Lattice `Backend` trait.
pub struct MemoryBackend {
    pub(crate) store: RwLock<HashMap<ApiName, BTreeMap<String, Record>>>,
    pub(crate) load_log: RwLock<HashMap<String, Vec<(ApiName, String)>>>,
    pub(crate) spec: RwLock<Option<Spec>>,
}

impl MemoryBackend {
    /// Create a new empty in-memory backend.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: RwLock::new(HashMap::new()),
            load_log: RwLock::new(HashMap::new()),
            spec: RwLock::new(None),
        })
    }

    /// Update the spec used for link traversal mappings.
    pub fn set_spec(&self, spec: Spec) -> Result<(), Error> {
        *lock_write(&self.spec)? = Some(spec);
        Ok(())
    }

    pub(crate) fn pk_key(pk: &Value) -> String {
        pk.to_string()
    }

    pub(crate) fn extract_pk(values: &BTreeMap<ApiName, Value>) -> Value {
        let id_key = ApiName::new_unchecked("id");
        values
            .get(&id_key)
            .cloned()
            .or_else(|| values.values().next().cloned())
            .unwrap_or_else(|| Value::string(uuid::Uuid::new_v4().to_string()))
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            load_log: RwLock::new(HashMap::new()),
            spec: RwLock::new(None),
        }
    }
}

impl Backend for MemoryBackend {
    fn backend_type(&self) -> &str {
        "memory"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            search: true,
            get: true,
            mutate: true,
            aggregate: true,
            traverse: true,
            bulk_load: true,
            rollback: true,
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
    fn as_aggregator(&self) -> Option<&dyn lattice_runtime::ports::Aggregator> {
        Some(self)
    }
    fn as_traverser(&self) -> Option<&dyn lattice_runtime::ports::Traverser> {
        Some(self)
    }
    fn as_bulk_loader(&self) -> Option<&dyn lattice_runtime::ports::BulkLoader> {
        Some(self)
    }
    fn as_rollbacker(&self) -> Option<&dyn lattice_runtime::ports::Rollbacker> {
        Some(self)
    }
    fn as_explainer(&self) -> Option<&dyn lattice_runtime::ports::SearchExplainer> {
        Some(self)
    }
}

impl Searcher for MemoryBackend {
    fn search(&self, object_type: &ApiName, query: &Query) -> Result<Page, Error> {
        let store = lock_read(&self.store)?;
        let type_store = match store.get(object_type) {
            Some(s) => s,
            None => {
                return Ok(Page {
                    records: Vec::new(),
                    next_cursor: None,
                })
            }
        };

        let mut records: Vec<Record> = type_store
            .values()
            .filter(|r| {
                query
                    .filter
                    .as_ref()
                    .is_none_or(|f| filter::evaluate(f, &r.values))
            })
            .cloned()
            .collect();

        if !query.sort.is_empty() {
            apply_sort(&mut records, &query.sort);
        }

        let offset = resolve_offset(&query.cursor, query.offset);
        let limit = query.limit.unwrap_or(1000) as usize;
        let total = records.len();
        let page_records: Vec<Record> = records.into_iter().skip(offset).take(limit).collect();

        let next_cursor = if offset + limit < total {
            Some((offset + limit).to_string())
        } else {
            None
        };

        Ok(Page {
            records: page_records,
            next_cursor,
        })
    }
}

impl Getter for MemoryBackend {
    fn get(&self, object_type: &ApiName, pk: &Value) -> Result<Option<Record>, Error> {
        let store = lock_read(&self.store)?;
        let key = Self::pk_key(pk);
        Ok(store.get(object_type).and_then(|t| t.get(&key)).cloned())
    }
}

impl Mutator for MemoryBackend {
    fn mutate(&self, object_type: &ApiName, mutation: &Mutation) -> Result<MutationResult, Error> {
        match mutation {
            Mutation::Create { values } => {
                let pk_val = Self::extract_pk(values);
                let pk_key = Self::pk_key(&pk_val);
                let record = Record {
                    primary_key: Some(pk_val),
                    values: values.clone(),
                };

                let mut store = lock_write(&self.store)?;
                let type_store = store.entry(object_type.clone()).or_default();

                if type_store.contains_key(&pk_key) {
                    return Err(Error::conflict(format!(
                        "record '{}' already exists in '{}'",
                        pk_key, object_type
                    )));
                }
                type_store.insert(pk_key, record.clone());
                Ok(MutationResult {
                    record: Some(record),
                    rows_affected: Some(1),
                })
            }

            Mutation::Update {
                primary_key,
                values,
            } => {
                let pk_key = Self::pk_key(primary_key);
                let mut store = lock_write(&self.store)?;
                let type_store = store.entry(object_type.clone()).or_default();

                match type_store.get_mut(&pk_key) {
                    None => Err(Error::not_found("record", primary_key)),
                    Some(existing) => {
                        for (k, v) in values {
                            existing.values.insert(k.clone(), v.clone());
                        }
                        Ok(MutationResult {
                            record: Some(existing.clone()),
                            rows_affected: Some(1),
                        })
                    }
                }
            }

            Mutation::Delete { primary_key } => {
                let pk_key = Self::pk_key(primary_key);
                let mut store = lock_write(&self.store)?;
                let removed = store
                    .get_mut(object_type)
                    .and_then(|t| t.remove(&pk_key))
                    .is_some();
                Ok(MutationResult {
                    record: None,
                    rows_affected: Some(if removed { 1 } else { 0 }),
                })
            }

            Mutation::Upsert { values } => {
                let pk_val = Self::extract_pk(values);
                let pk_key = Self::pk_key(&pk_val);
                let record = Record {
                    primary_key: Some(pk_val),
                    values: values.clone(),
                };

                let mut store = lock_write(&self.store)?;
                store
                    .entry(object_type.clone())
                    .or_default()
                    .insert(pk_key, record.clone());
                Ok(MutationResult {
                    record: Some(record),
                    rows_affected: Some(1),
                })
            }

            Mutation::Batch { items } => {
                let mut total_affected = 0i64;
                for item in items {
                    let result = self.mutate(object_type, item)?;
                    total_affected += result.rows_affected.unwrap_or(0);
                }
                Ok(MutationResult {
                    record: None,
                    rows_affected: Some(total_affected),
                })
            }
        }
    }
}
