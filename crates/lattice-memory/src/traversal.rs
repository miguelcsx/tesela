//! Aggregator, Traverser, BulkLoader, Rollbacker, and SearchExplainer implementations.

use crate::backend::MemoryBackend;
use crate::{aggregate, apply_sort, filter, resolve_offset};
use lattice_core::{ApiName, Error, Value};
use lattice_ir::{AggregateResult, ExplainPlan, Filter, Page, Record};
use lattice_runtime::{
    ports::{Aggregator, BulkLoader, Rollbacker, SearchExplainer, Traverser},
    query::{AggregateQuery, Query, TraversalQuery},
};
use std::collections::BTreeMap;

impl Aggregator for MemoryBackend {
    fn aggregate(
        &self,
        object_type: &ApiName,
        query: &AggregateQuery,
    ) -> Result<AggregateResult, Error> {
        let store = self.store.read().unwrap_or_else(|e| e.into_inner());
        let records: Vec<Record> = store
            .get(object_type)
            .map(|t| {
                t.values()
                    .filter(|r| {
                        query
                            .filter
                            .as_ref()
                            .is_none_or(|f| filter::evaluate(f, &r.values))
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        Ok(aggregate::compute(
            &records,
            &query.group_by,
            &query.aggregations,
        ))
    }
}

impl Traverser for MemoryBackend {
    fn traverse(&self, link_type: &ApiName, query: &TraversalQuery) -> Result<Page, Error> {
        let spec_guard = self.spec.read().unwrap_or_else(|e| e.into_inner());
        let link = spec_guard
            .as_ref()
            .and_then(|s| s.link_types.iter().find(|l| l.api_name == *link_type))
            .cloned();

        let link = match link {
            Some(l) => l,
            None => return Err(Error::not_found("link_type", link_type)),
        };

        let target_type = link.to.clone();
        let mapping = link.mappings.first();
        let (from_prop, to_prop) = match mapping {
            Some(m) => (m.from_property.clone(), m.to_property.clone()),
            None => {
                return Ok(Page {
                    records: Vec::new(),
                    next_cursor: None,
                })
            }
        };

        let store = self.store.read().unwrap_or_else(|e| e.into_inner());
        let source_type = &link.from;
        let source_pk_key = MemoryBackend::pk_key(&query.source_pk);

        let source_field_val = store
            .get(source_type)
            .and_then(|t| t.get(&source_pk_key))
            .and_then(|r| r.values.get(&from_prop))
            .cloned();

        let source_val = match source_field_val {
            Some(v) => v,
            None => {
                return Ok(Page {
                    records: Vec::new(),
                    next_cursor: None,
                })
            }
        };

        let join_filter = Filter::eq(to_prop, source_val);
        let combined_filter = match &query.filter {
            Some(user_filter) => Filter::and(vec![join_filter, user_filter.clone()]),
            None => join_filter,
        };

        let mut records: Vec<Record> = store
            .get(&target_type)
            .map(|t| {
                t.values()
                    .filter(|r| filter::evaluate(&combined_filter, &r.values))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        if !query.sort.is_empty() {
            apply_sort(&mut records, &query.sort);
        }

        let offset = resolve_offset(&None, query.offset);
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

impl BulkLoader for MemoryBackend {
    fn bulk_load(
        &self,
        object_type: &ApiName,
        records: Vec<Record>,
        load_id: &str,
    ) -> Result<i64, Error> {
        let count = records.len() as i64;
        let mut store = self.store.write().unwrap_or_else(|e| e.into_inner());
        let mut log = self.load_log.write().unwrap_or_else(|e| e.into_inner());
        let type_store = store.entry(object_type.clone()).or_default();
        let load_entries = log.entry(load_id.to_string()).or_default();

        for record in records {
            let pk_key = record
                .primary_key
                .as_ref()
                .map(MemoryBackend::pk_key)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            load_entries.push((object_type.clone(), pk_key.clone()));
            type_store.insert(pk_key, record);
        }
        Ok(count)
    }
}

impl Rollbacker for MemoryBackend {
    fn rollback(&self, _object_type: &ApiName, load_id: &str) -> Result<(), Error> {
        let mut store = self.store.write().unwrap_or_else(|e| e.into_inner());
        let mut log = self.load_log.write().unwrap_or_else(|e| e.into_inner());
        if let Some(entries) = log.remove(load_id) {
            for (obj_type, pk_key) in entries {
                if let Some(type_store) = store.get_mut(&obj_type) {
                    type_store.remove(&pk_key);
                }
            }
        }
        Ok(())
    }
}

impl SearchExplainer for MemoryBackend {
    fn explain_search(&self, object_type: &ApiName, query: &Query) -> Result<ExplainPlan, Error> {
        let store = self.store.read().unwrap_or_else(|e| e.into_inner());
        let total_records = store.get(object_type).map(|t| t.len()).unwrap_or(0);
        let mut steps: Vec<BTreeMap<String, Value>> = Vec::new();

        let mut scan = BTreeMap::new();
        scan.insert("op".to_string(), Value::string("full_scan"));
        scan.insert(
            "object_type".to_string(),
            Value::string(object_type.to_string()),
        );
        scan.insert(
            "total_records".to_string(),
            Value::integer(total_records as i64),
        );
        steps.push(scan);

        if query.filter.is_some() {
            let mut f = BTreeMap::new();
            f.insert("op".to_string(), Value::string("filter"));
            f.insert(
                "filter_json".to_string(),
                Value::string(serde_json::to_string(&query.filter).unwrap_or_default()),
            );
            steps.push(f);
        }

        if !query.sort.is_empty() {
            let mut s = BTreeMap::new();
            s.insert("op".to_string(), Value::string("sort"));
            s.insert(
                "columns".to_string(),
                Value::string(
                    query
                        .sort
                        .iter()
                        .map(|s| format!("{} {}", s.property, s.direction))
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            );
            steps.push(s);
        }

        if let Some(limit) = query.limit {
            let mut l = BTreeMap::new();
            l.insert("op".to_string(), Value::string("limit"));
            l.insert("n".to_string(), Value::integer(limit as i64));
            steps.push(l);
        }

        Ok(ExplainPlan { steps })
    }
}
