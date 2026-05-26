#![deny(warnings)]
#![deny(missing_docs)]

//! In-memory backend adapter for Tesela.
//!
//! Provides a fully-functional in-memory `Backend` implementation suitable for
//! testing, examples, and development workflows. All data is stored in a
//! `RwLock`-protected `HashMap` keyed by object type and serialized primary key.

mod aggregate;
pub(crate) mod filter;

mod backend;
mod registry;
mod traversal;

pub use backend::MemoryBackend;
pub use registry::{DefaultBackendRegistry, MemoryBackendFactory, memory_capabilities};

use tesela_ir::Record;
use tesela_runtime::query::Sort;

pub(crate) fn apply_sort(records: &mut [Record], sort: &[Sort]) {
    records.sort_by(|a, b| {
        for s in sort {
            let av = a.values.get(&s.property);
            let bv = b.values.get(&s.property);
            let cmp = match (av, bv) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (Some(av), Some(bv)) => av.cmp(bv),
            };
            let cmp = if s.direction.to_lowercase() == "desc" {
                cmp.reverse()
            } else {
                cmp
            };
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }
        std::cmp::Ordering::Equal
    });
}

pub(crate) fn resolve_offset(cursor: &Option<String>, offset: Option<i32>) -> usize {
    if let Some(c) = cursor
        && let Ok(n) = c.parse::<usize>()
    {
        return n;
    }
    offset.unwrap_or(0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesela_core::{ApiName, Value};
    use tesela_ir::{Filter, Record};
    use tesela_runtime::{
        ports::{Aggregator, BulkLoader, Getter, Mutator, Rollbacker, Searcher},
        query::{AggregateQuery, Aggregation, Mutation, Query, Sort},
    };
    use std::collections::BTreeMap;

    fn pk_record(id: i64, extra: &[(&str, serde_json::Value)]) -> Record {
        let pk_val = Value::integer(id);
        let mut values: BTreeMap<ApiName, Value> = extra
            .iter()
            .map(|(k, v)| (ApiName::new_unchecked(k), Value::new(v.clone())))
            .collect();
        values.insert(ApiName::new_unchecked("id"), pk_val.clone());
        Record {
            primary_key: Some(pk_val),
            values,
        }
    }

    #[test]
    fn test_create_and_get() {
        let b = MemoryBackend::new();
        let obj = ApiName::new_unchecked("user");
        let mut vals = BTreeMap::new();
        vals.insert(ApiName::new_unchecked("id"), Value::string("u1"));
        vals.insert(ApiName::new_unchecked("name"), Value::string("Alice"));

        b.mutate(&obj, &Mutation::Create { values: vals }).unwrap();

        let got = b.get(&obj, &Value::string("u1")).unwrap().unwrap();
        assert_eq!(
            got.values.get(&ApiName::new_unchecked("name")),
            Some(&Value::string("Alice"))
        );
    }

    #[test]
    fn test_search_eq_filter() {
        let b = MemoryBackend::new();
        let obj = ApiName::new_unchecked("user");

        for (id, name) in [("u1", "Alice"), ("u2", "Bob"), ("u3", "Alice")] {
            let mut vals = BTreeMap::new();
            vals.insert(ApiName::new_unchecked("id"), Value::string(id));
            vals.insert(ApiName::new_unchecked("name"), Value::string(name));
            b.mutate(&obj, &Mutation::Upsert { values: vals }).unwrap();
        }

        let q = Query {
            filter: Some(Filter::eq(
                ApiName::new_unchecked("name"),
                Value::string("Alice"),
            )),
            ..Default::default()
        };
        let page = b.search(&obj, &q).unwrap();
        assert_eq!(page.records.len(), 2);
    }

    #[test]
    fn test_update_and_delete() {
        let b = MemoryBackend::new();
        let obj = ApiName::new_unchecked("item");
        let mut vals = BTreeMap::new();
        vals.insert(ApiName::new_unchecked("id"), Value::string("i1"));
        vals.insert(ApiName::new_unchecked("count"), Value::integer(5));
        b.mutate(&obj, &Mutation::Create { values: vals }).unwrap();

        let mut upd = BTreeMap::new();
        upd.insert(ApiName::new_unchecked("count"), Value::integer(10));
        b.mutate(
            &obj,
            &Mutation::Update {
                primary_key: Value::string("i1"),
                values: upd,
            },
        )
        .unwrap();

        let got = b.get(&obj, &Value::string("i1")).unwrap().unwrap();
        assert_eq!(
            got.values.get(&ApiName::new_unchecked("count")),
            Some(&Value::integer(10))
        );

        b.mutate(
            &obj,
            &Mutation::Delete {
                primary_key: Value::string("i1"),
            },
        )
        .unwrap();
        assert!(b.get(&obj, &Value::string("i1")).unwrap().is_none());
    }

    #[test]
    fn test_bulk_load_and_rollback() {
        let b = MemoryBackend::new();
        let obj = ApiName::new_unchecked("product");
        let records: Vec<Record> = (0..3i64)
            .map(|i| pk_record(i, &[("name", serde_json::json!(format!("p{}", i)))]))
            .collect();

        b.bulk_load(&obj, records, "load1").unwrap();
        assert_eq!(b.search(&obj, &Query::default()).unwrap().records.len(), 3);

        b.rollback(&obj, "load1").unwrap();
        assert_eq!(b.search(&obj, &Query::default()).unwrap().records.len(), 0);
    }

    #[test]
    fn test_sort_ascending() {
        let b = MemoryBackend::new();
        let obj = ApiName::new_unchecked("score");
        for n in [3i64, 1, 4, 1, 5] {
            let mut vals = BTreeMap::new();
            vals.insert(
                ApiName::new_unchecked("id"),
                Value::string(format!("id{}", n)),
            );
            vals.insert(ApiName::new_unchecked("n"), Value::integer(n));
            b.mutate(&obj, &Mutation::Upsert { values: vals }).unwrap();
        }

        let q = Query {
            sort: vec![Sort {
                property: ApiName::new_unchecked("n"),
                direction: "asc".to_string(),
            }],
            ..Default::default()
        };
        let page = b.search(&obj, &q).unwrap();
        let ns: Vec<i64> = page
            .records
            .iter()
            .filter_map(|r| {
                r.values
                    .get(&ApiName::new_unchecked("n"))
                    .and_then(|v| v.as_i64())
            })
            .collect();
        assert!(ns.windows(2).all(|w| w[0] <= w[1]), "not sorted: {:?}", ns);
    }

    #[test]
    fn test_aggregate_count_and_avg() {
        let b = MemoryBackend::new();
        let obj = ApiName::new_unchecked("order");
        for (i, (dept, amount)) in [("eng", 100.0), ("eng", 200.0), ("hr", 50.0)]
            .iter()
            .enumerate()
        {
            let mut vals = BTreeMap::new();
            vals.insert(ApiName::new_unchecked("id"), Value::integer(i as i64));
            vals.insert(ApiName::new_unchecked("dept"), Value::string(*dept));
            vals.insert(ApiName::new_unchecked("amount"), Value::float(*amount));
            b.mutate(&obj, &Mutation::Upsert { values: vals }).unwrap();
        }

        let q = AggregateQuery {
            filter: None,
            group_by: vec![ApiName::new_unchecked("dept")],
            aggregations: vec![
                Aggregation {
                    function: "count".to_string(),
                    property: None,
                    alias: "n".to_string(),
                },
                Aggregation {
                    function: "avg".to_string(),
                    property: Some(ApiName::new_unchecked("amount")),
                    alias: "avg_amt".to_string(),
                },
            ],
            time_bucket: None,
            spatial_extent: None,
            require_pushdown: false,
        };
        let result = b.aggregate(&obj, &q).unwrap();
        assert_eq!(result.groups.len(), 2);
        let eng = result
            .groups
            .iter()
            .find(|g| g.get("dept").and_then(|v| v.as_str()) == Some("eng"))
            .unwrap();
        assert_eq!(eng.get("n").and_then(|v| v.as_i64()), Some(2));
        let avg = eng.get("avg_amt").and_then(|v| v.as_f64()).unwrap();
        assert!((avg - 150.0).abs() < 0.001, "avg was {}", avg);
    }

    #[test]
    fn test_cursor_pagination() {
        let b = MemoryBackend::new();
        let obj = ApiName::new_unchecked("item");
        for i in 0..10i64 {
            let mut vals = BTreeMap::new();
            vals.insert(ApiName::new_unchecked("id"), Value::integer(i));
            b.mutate(&obj, &Mutation::Upsert { values: vals }).unwrap();
        }

        let q1 = Query {
            limit: Some(4),
            ..Default::default()
        };
        let page1 = b.search(&obj, &q1).unwrap();
        assert_eq!(page1.records.len(), 4);
        let cursor = page1.next_cursor.unwrap();

        let q2 = Query {
            limit: Some(4),
            cursor: Some(cursor),
            ..Default::default()
        };
        let page2 = b.search(&obj, &q2).unwrap();
        assert_eq!(page2.records.len(), 4);
        assert!(page2.next_cursor.is_some());
    }
}
