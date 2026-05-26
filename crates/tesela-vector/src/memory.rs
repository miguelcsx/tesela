//! In-memory brute-force vector backend.

use std::collections::HashMap;
use std::sync::RwLock;
use tesela_core::{ApiName, Error, Value};
use tesela_ir::Record;
use tesela_runtime::ports::{VectorBackend, VectorResult, VectorSearchQuery};

/// A single indexed vector entry.
struct Entry {
    pk: Value,
    vector: Vec<f32>,
    record: Record,
}

/// Per-object-type index holding all vectors in memory.
struct ObjectIndex {
    entries: Vec<Entry>,
    dimension: u32,
}

impl ObjectIndex {
    fn new(dimension: u32) -> Self {
        Self {
            entries: Vec::new(),
            dimension,
        }
    }

    fn upsert(&mut self, pk: Value, vector: Vec<f32>, record: Record) {
        if let Some(pos) = self.entries.iter().position(|e| e.pk == pk) {
            self.entries[pos] = Entry { pk, vector, record };
        } else {
            self.entries.push(Entry { pk, vector, record });
        }
    }

    fn delete(&mut self, pk: &Value) {
        self.entries.retain(|e| &e.pk != pk);
    }

    /// Brute-force cosine k-NN: returns `(distance, &Record)` pairs sorted
    /// ascending by distance (closest first).
    fn search(&self, query: &[f32], top_k: usize) -> Vec<(f32, &Record)> {
        let query_norm = l2_norm(query);
        let mut scored: Vec<(f32, &Record)> = self
            .entries
            .iter()
            .map(|e| (cosine_distance(query, &e.vector, query_norm), &e.record))
            .collect();
        scored.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

/// In-memory vector backend; one brute-force index per object type.
pub struct MemoryVectorBackend {
    indices: RwLock<HashMap<ApiName, ObjectIndex>>,
    default_dimension: u32,
}

impl MemoryVectorBackend {
    /// Create a new in-memory backend.  `default_dimension` is used when
    /// an index is created implicitly on the first `index_vector` call.
    pub fn new(default_dimension: u32) -> Self {
        Self {
            indices: RwLock::new(HashMap::new()),
            default_dimension,
        }
    }
}

impl VectorBackend for MemoryVectorBackend {
    fn vector_search(&self, query: &VectorSearchQuery) -> Result<Vec<VectorResult>, Error> {
        let indices = self
            .indices
            .read()
            .map_err(|_| Error::internal("vector index lock poisoned"))?;

        let index = match indices.get(&query.object_type) {
            Some(idx) => idx,
            None => return Ok(Vec::new()),
        };

        if query.query_vector.len() as u32 != index.dimension {
            return Err(Error::validation(format!(
                "query vector dimension {} does not match index dimension {}",
                query.query_vector.len(),
                index.dimension
            )));
        }

        Ok(index
            .search(&query.query_vector, query.top_k)
            .into_iter()
            .map(|(distance, record)| VectorResult {
                record: record.clone(),
                distance,
            })
            .collect())
    }

    fn index_vector(&self, object_type: &ApiName, pk: &Value, vector: &[f32]) -> Result<(), Error> {
        let mut indices = self
            .indices
            .write()
            .map_err(|_| Error::internal("vector index lock poisoned"))?;

        let dim = self.default_dimension;
        let index = indices
            .entry(object_type.clone())
            .or_insert_with(|| ObjectIndex::new(dim));

        if vector.len() as u32 != index.dimension {
            return Err(Error::validation(format!(
                "vector dimension {} does not match index dimension {}",
                vector.len(),
                index.dimension
            )));
        }

        let record = Record {
            primary_key: None,
            values: std::collections::BTreeMap::new(),
        };
        index.upsert(pk.clone(), vector.to_vec(), record);
        Ok(())
    }

    fn delete_vector(&self, object_type: &ApiName, pk: &Value) -> Result<(), Error> {
        let mut indices = self
            .indices
            .write()
            .map_err(|_| Error::internal("vector index lock poisoned"))?;

        if let Some(index) = indices.get_mut(object_type) {
            index.delete(pk);
        }
        Ok(())
    }
}

fn l2_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

fn cosine_distance(a: &[f32], b: &[f32], a_norm: f32) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let b_norm = l2_norm(b);
    let denom = a_norm * b_norm;
    if denom == 0.0 {
        1.0
    } else {
        1.0 - (dot / denom)
    }
}
