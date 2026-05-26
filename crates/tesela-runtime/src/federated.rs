//! Federated search across multiple backends.

use crate::ports::{BackendRegistry, FederatedBackend, FederatedQuery};
use crate::query::Sort;
use std::sync::Arc;
use tesela_core::Error;
use tesela_ir::Page;

/// Executes fan-out searches across registered backends and merges results.
///
/// Each [`FederatedQuery`] is dispatched to the backend registered for its
/// datasource.  Results are collected, re-sorted by the first sort directive
/// of the winning query (if any), and returned as a single merged `Page`.
pub struct FederatedExecutor {
    registry: Arc<dyn BackendRegistry>,
}

impl FederatedExecutor {
    /// Create a new federated executor backed by the given registry.
    pub fn new(registry: Arc<dyn BackendRegistry>) -> Self {
        Self { registry }
    }
}

impl FederatedBackend for FederatedExecutor {
    fn federated_search(&self, queries: &[FederatedQuery]) -> Result<Page, Error> {
        let mut all_records = Vec::new();

        for fq in queries {
            let backend = self.registry.acquire(&fq.datasource)?;
            let searcher = backend
                .as_searcher()
                .ok_or_else(|| Error::unsupported("search on federated datasource"))?;
            let page = searcher.search(&fq.object_type, &fq.query)?;
            all_records.extend(page.records);
        }

        // Re-sort merged results by the sort directive of the first query.
        if let Some(first_sort) = queries.first().and_then(|q| q.query.sort.first()) {
            sort_records(&mut all_records, first_sort);
        }

        // Apply the limit of the first query (conservative).
        let limit = queries
            .first()
            .and_then(|q| q.query.limit)
            .unwrap_or(i32::MAX) as usize;
        all_records.truncate(limit);

        Ok(Page {
            records: all_records,
            next_cursor: None,
        })
    }
}

/// Sort records in-place by a single sort directive.
fn sort_records(records: &mut [tesela_ir::Record], sort: &Sort) {
    let asc = sort.direction.to_lowercase() != "desc";
    records.sort_by(|a, b| {
        let av = a.values.get(&sort.property);
        let bv = b.values.get(&sort.property);
        let ord = av.cmp(&bv);
        if asc { ord } else { ord.reverse() }
    });
}
