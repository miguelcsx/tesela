//! Runtime lineage store implementation.

use crate::ports::{LineageKind, LineageRecord, LineageStore};
use tesela_core::{ApiName, Error, Value};
use std::sync::RwLock;

/// In-memory lineage store backed by a `Vec`.
///
/// Lineage records are never discarded and grow unboundedly; this is suitable
/// for development and testing.  Production deployments should implement
/// `LineageStore` over a persistent backend (e.g., a dedicated graph DB table).
pub struct MemoryLineageStore {
    records: RwLock<Vec<LineageRecord>>,
}

impl MemoryLineageStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            records: RwLock::new(Vec::new()),
        }
    }
}

impl Default for MemoryLineageStore {
    fn default() -> Self {
        Self::new()
    }
}

impl LineageStore for MemoryLineageStore {
    fn record(&self, edge: LineageRecord) -> Result<(), Error> {
        self.records
            .write()
            .map_err(|_| Error::internal("lineage store lock poisoned"))?
            .push(edge);
        Ok(())
    }

    fn query_lineage(
        &self,
        object_type: &ApiName,
        pk: &Value,
        depth: Option<u32>,
    ) -> Result<Vec<LineageRecord>, Error> {
        let max_depth = depth.unwrap_or(u32::MAX);
        let records = self
            .records
            .read()
            .map_err(|_| Error::internal("lineage store lock poisoned"))?;

        // BFS over the lineage graph up to `max_depth` hops.
        let mut result: Vec<LineageRecord> = Vec::new();
        let mut frontier: Vec<(ApiName, Value)> = vec![(object_type.clone(), pk.clone())];
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();

        for _ in 0..max_depth {
            if frontier.is_empty() {
                break;
            }
            let mut next_frontier = Vec::new();
            for (ot, key) in &frontier {
                for rec in records.iter() {
                    let key_str = format!("{}:{}", ot, key);
                    if visited.contains(&key_str) {
                        continue;
                    }
                    let matches_source = &rec.source_object_type == ot && &rec.source_pk == key;
                    let matches_target = &rec.target_object_type == ot && &rec.target_pk == key;
                    if matches_source || matches_target {
                        result.push(rec.clone());
                        // Queue the other end for the next hop.
                        if matches_source {
                            next_frontier
                                .push((rec.target_object_type.clone(), rec.target_pk.clone()));
                        } else {
                            next_frontier
                                .push((rec.source_object_type.clone(), rec.source_pk.clone()));
                        }
                    }
                }
                visited.insert(format!("{}:{}", ot, key));
            }
            frontier = next_frontier;
        }

        Ok(result)
    }
}

/// Parameters for building a `Produces` lineage edge.
#[derive(Debug, Clone)]
pub struct ProducesEdgeParams {
    /// Record id.
    pub id: String,
    /// Source object type.
    pub source_object_type: ApiName,
    /// Source primary key.
    pub source_pk: Value,
    /// Target object type.
    pub target_object_type: ApiName,
    /// Target primary key.
    pub target_pk: Value,
    /// Actor user id.
    pub actor_user_id: String,
    /// Occurred-at timestamp.
    pub occurred_at: String,
    /// Optional pipeline name.
    pub pipeline: Option<ApiName>,
}

/// Build a lineage record for a write operation.
///
/// `source` is the entity that produced `target`, linked by a `Produces` edge.
pub fn build_produces_edge(params: ProducesEdgeParams) -> LineageRecord {
    LineageRecord {
        id: params.id,
        source_object_type: params.source_object_type,
        source_pk: params.source_pk,
        target_object_type: params.target_object_type,
        target_pk: params.target_pk,
        edge_kind: LineageKind::Produces,
        actor_user_id: params.actor_user_id,
        occurred_at: params.occurred_at,
        pipeline: params.pipeline,
    }
}
