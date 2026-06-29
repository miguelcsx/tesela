//! In-memory [`OntologyStore`] implementation.

use crate::{AggregateQuery, OntologyStore, Query, StoreCapabilities, TraversalQuery};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};
use tesela_core::{ApiName, Error, FilterOp, Value, lock_read, lock_write};
use tesela_ir::{AggregateResult, MutationResult, Page, Record, Spec};

/// Thread-safe in-memory store for tests and local development.
pub struct MemoryStore {
    records: RwLock<HashMap<ApiName, BTreeMap<String, Record>>>,
    spec: RwLock<Option<Spec>>,
}

impl MemoryStore {
    /// Create an empty memory store.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            records: RwLock::new(HashMap::new()),
            spec: RwLock::new(None),
        })
    }

    /// Update the spec used for link traversal.
    pub fn set_spec(&self, spec: Spec) -> Result<(), Error> {
        *lock_write(&self.spec)? = Some(spec);
        Ok(())
    }

    fn pk_key(primary_key: &Value) -> String {
        primary_key.to_string()
    }

    fn extract_pk(
        &self,
        object_type: &ApiName,
        values: &BTreeMap<ApiName, Value>,
    ) -> Result<Value, Error> {
        if let Some(spec) = lock_read(&self.spec)?.as_ref()
            && let Some(object) = spec
                .object_types
                .iter()
                .find(|object| object.api_name == *object_type)
        {
            return values.get(&object.primary_key).cloned().ok_or_else(|| {
                Error::bad_request(format!(
                    "create for '{}' requires primary key '{}'",
                    object_type, object.primary_key
                ))
            });
        }

        let id = ApiName::new("id").map_err(|error| {
            Error::bad_request(format!("invalid fallback primary key: {error}"))
        })?;
        match values.get(&id).cloned() {
            Some(value) => Ok(value),
            None => Ok(Value::string(uuid::Uuid::new_v4().to_string())),
        }
    }
}

impl OntologyStore for MemoryStore {
    fn store_type(&self) -> &str {
        "memory"
    }

    fn capabilities(&self) -> StoreCapabilities {
        StoreCapabilities {
            search: true,
            get: true,
            create: true,
            update: true,
            delete: true,
            aggregate: true,
            traverse: true,
            execute_action: false,
        }
    }

    fn search(&self, object_type: &ApiName, query: &Query) -> Result<Page, Error> {
        let records = lock_read(&self.records)?;
        let mut rows: Vec<Record> = match records.get(object_type) {
            Some(store) => store
                .values()
                .filter(|record| {
                    query
                        .filter
                        .as_ref()
                        .is_none_or(|filter| evaluate_filter(filter, &record.values))
                })
                .cloned()
                .collect(),
            None => Vec::new(),
        };

        apply_sort(&mut rows, &query.sort);
        let offset = resolve_offset(query);
        let limit = query_limit(query);
        let total = rows.len();
        let page_records = rows.into_iter().skip(offset).take(limit).collect();
        let next_cursor = (offset + limit < total).then(|| (offset + limit).to_string());

        Ok(Page {
            records: page_records,
            next_cursor,
        })
    }

    fn get(&self, object_type: &ApiName, primary_key: &Value) -> Result<Option<Record>, Error> {
        Ok(lock_read(&self.records)?
            .get(object_type)
            .and_then(|store| store.get(&Self::pk_key(primary_key)))
            .cloned())
    }

    fn create(
        &self,
        object_type: &ApiName,
        values: BTreeMap<ApiName, Value>,
    ) -> Result<MutationResult, Error> {
        let primary_key = self.extract_pk(object_type, &values)?;
        let key = Self::pk_key(&primary_key);
        let mut records = lock_write(&self.records)?;
        let store = records.entry(object_type.clone()).or_default();
        if store.contains_key(&key) {
            return Err(Error::conflict(format!(
                "record '{}' already exists in '{}'",
                key, object_type
            )));
        }
        let record = Record {
            primary_key: Some(primary_key),
            values,
        };
        store.insert(key, record.clone());
        Ok(MutationResult {
            record: Some(record),
            rows_affected: Some(1),
        })
    }

    fn update(
        &self,
        object_type: &ApiName,
        primary_key: &Value,
        values: BTreeMap<ApiName, Value>,
    ) -> Result<MutationResult, Error> {
        let mut records = lock_write(&self.records)?;
        let store = records.entry(object_type.clone()).or_default();
        let key = Self::pk_key(primary_key);
        let record = store
            .get_mut(&key)
            .ok_or_else(|| Error::not_found("record", primary_key))?;
        for (field, value) in values {
            record.values.insert(field, value);
        }
        Ok(MutationResult {
            record: Some(record.clone()),
            rows_affected: Some(1),
        })
    }

    fn delete(&self, object_type: &ApiName, primary_key: &Value) -> Result<MutationResult, Error> {
        let removed = lock_write(&self.records)?
            .get_mut(object_type)
            .and_then(|store| store.remove(&Self::pk_key(primary_key)))
            .is_some();
        Ok(MutationResult {
            record: None,
            rows_affected: Some(i64::from(removed)),
        })
    }

    fn aggregate(
        &self,
        object_type: &ApiName,
        query: &AggregateQuery,
    ) -> Result<AggregateResult, Error> {
        let page = self.search(
            object_type,
            &Query {
                filter: query.filter.clone(),
                limit: Some(i32::MAX),
                ..Query::default()
            },
        )?;
        Ok(compute_aggregate(
            &page.records,
            &query.group_by,
            &query.aggregations,
        ))
    }

    fn traverse(&self, link_type: &ApiName, query: &TraversalQuery) -> Result<Page, Error> {
        let spec = lock_read(&self.spec)?
            .clone()
            .ok_or_else(|| Error::validation("memory traversal requires a spec"))?;
        let link = spec
            .link_types
            .iter()
            .find(|item| &item.api_name == link_type)
            .ok_or_else(|| Error::not_found("link_type", link_type))?;
        let mapping = link
            .mappings
            .first()
            .ok_or_else(|| Error::unsupported("link traversal without mapping"))?;
        let target_filter =
            tesela_ir::Filter::eq(mapping.to_property.clone(), query.source_pk.clone());
        self.search(
            &link.to,
            &Query {
                filter: Some(match &query.filter {
                    Some(filter) => tesela_ir::Filter::and(vec![target_filter, filter.clone()]),
                    None => target_filter,
                }),
                sort: query.sort.clone(),
                limit: query.limit,
                offset: query.offset,
                cursor: None,
            },
        )
    }
}

fn resolve_offset(query: &Query) -> usize {
    let parsed_cursor = query
        .cursor
        .as_deref()
        .and_then(|cursor| cursor.parse::<usize>().ok());
    match parsed_cursor {
        Some(value) => value,
        None => match query.offset {
            Some(offset) => offset.max(0) as usize,
            None => 0,
        },
    }
}

fn query_limit(query: &Query) -> usize {
    if let Some(limit) = query.limit {
        return limit.max(0) as usize;
    }
    1000
}

fn apply_sort(records: &mut [Record], sort: &[crate::Sort]) {
    records.sort_by(|left, right| {
        for item in sort {
            let ordering = match (
                left.values.get(&item.property),
                right.values.get(&item.property),
            ) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (Some(left_value), Some(right_value)) => left_value.cmp(right_value),
            };
            let ordering = match item.direction {
                crate::SortDirection::Asc => ordering,
                crate::SortDirection::Desc => ordering.reverse(),
            };
            if !ordering.is_eq() {
                return ordering;
            }
        }
        std::cmp::Ordering::Equal
    });
}

fn evaluate_filter(filter: &tesela_ir::Filter, record: &BTreeMap<ApiName, Value>) -> bool {
    match filter.op {
        FilterOp::And => filter.args.iter().all(|item| evaluate_filter(item, record)),
        FilterOp::Or => filter.args.iter().any(|item| evaluate_filter(item, record)),
        FilterOp::Not => filter
            .args
            .first()
            .is_none_or(|item| !evaluate_filter(item, record)),
        _ => filter
            .field
            .as_ref()
            .is_none_or(|field| evaluate_scalar(filter, record.get(field))),
    }
}

fn evaluate_scalar(filter: &tesela_ir::Filter, field_value: Option<&Value>) -> bool {
    match filter.op {
        FilterOp::IsNull => field_value.is_none_or(Value::is_null),
        FilterOp::IsNotNull => field_value.is_some_and(|value| !value.is_null()),
        FilterOp::Eq => compare_one(field_value, filter.value.as_ref(), |left, right| {
            left == right
        }),
        FilterOp::Ne => compare_one(field_value, filter.value.as_ref(), |left, right| {
            left != right
        }),
        FilterOp::Lt => compare_one(field_value, filter.value.as_ref(), |left, right| {
            left < right
        }),
        FilterOp::Lte => compare_one(field_value, filter.value.as_ref(), |left, right| {
            left <= right
        }),
        FilterOp::Gt => compare_one(field_value, filter.value.as_ref(), |left, right| {
            left > right
        }),
        FilterOp::Gte => compare_one(field_value, filter.value.as_ref(), |left, right| {
            left >= right
        }),
        FilterOp::In => field_value.is_some_and(|value| filter.values.contains(value)),
        FilterOp::NotIn => field_value.is_none_or(|value| !filter.values.contains(value)),
        FilterOp::Contains => {
            string_match(field_value, filter.value.as_ref(), |actual, expected| {
                actual.contains(expected)
            })
        }
        FilterOp::StartsWith => {
            string_match(field_value, filter.value.as_ref(), |actual, expected| {
                actual.starts_with(expected)
            })
        }
        FilterOp::Between => {
            field_value.is_some_and(
                |value| match (filter.values.first(), filter.values.get(1)) {
                    (Some(left), Some(right)) => value >= left && value <= right,
                    _ => false,
                },
            )
        }
        FilterOp::Like => string_match(field_value, filter.value.as_ref(), like_match),
        FilterOp::And | FilterOp::Or | FilterOp::Not => true,
    }
}

fn compare_one(
    field_value: Option<&Value>,
    filter_value: Option<&Value>,
    compare: impl Fn(&Value, &Value) -> bool,
) -> bool {
    match filter_value {
        Some(expected) => field_value.is_some_and(|actual| compare(actual, expected)),
        None => field_value.is_none_or(Value::is_null),
    }
}

fn string_match(
    field_value: Option<&Value>,
    filter_value: Option<&Value>,
    compare: impl Fn(&str, &str) -> bool,
) -> bool {
    match (
        field_value.and_then(Value::as_str),
        filter_value.and_then(Value::as_str),
    ) {
        (Some(actual), Some(expected)) => compare(actual, expected),
        _ => false,
    }
}

fn like_match(text: &str, pattern: &str) -> bool {
    if pattern == "%" {
        return true;
    }
    let parts: Vec<&str> = pattern.split('%').collect();
    if parts.len() == 1 {
        return text == pattern;
    }
    let mut cursor = 0usize;
    for part in parts.into_iter().filter(|part| !part.is_empty()) {
        let Some(found) = text[cursor..].find(part) else {
            return false;
        };
        cursor += found + part.len();
    }
    true
}

fn compute_aggregate(
    records: &[Record],
    group_by: &[ApiName],
    aggregations: &[crate::Aggregation],
) -> AggregateResult {
    let mut groups: BTreeMap<Vec<Value>, Vec<&Record>> = BTreeMap::new();
    for record in records {
        let key = group_by
            .iter()
            .map(|field| match record.values.get(field) {
                Some(value) => value.clone(),
                None => Value::null(),
            })
            .collect();
        groups.entry(key).or_default().push(record);
    }
    if groups.is_empty() && group_by.is_empty() {
        groups.insert(Vec::new(), Vec::new());
    }
    let mut rows = Vec::new();
    for (key, group_records) in groups {
        let mut row = BTreeMap::new();
        for (field, value) in group_by.iter().zip(key) {
            row.insert(field.to_string(), value);
        }
        for aggregation in aggregations {
            row.insert(
                aggregation.alias.clone(),
                aggregate_value(
                    aggregation.function,
                    aggregation.property.as_ref(),
                    &group_records,
                ),
            );
        }
        rows.push(row);
    }
    AggregateResult { groups: rows }
}

fn aggregate_value(
    function: crate::AggregationFunction,
    property: Option<&ApiName>,
    records: &[&Record],
) -> Value {
    match function {
        crate::AggregationFunction::Count => Value::integer(records.len() as i64),
        crate::AggregationFunction::Sum => Value::float(numeric_values(property, records).sum()),
        crate::AggregationFunction::Avg => {
            let values: Vec<f64> = numeric_values(property, records).collect();
            if values.is_empty() {
                Value::null()
            } else {
                Value::float(values.iter().sum::<f64>() / values.len() as f64)
            }
        }
        crate::AggregationFunction::Min => extrema(property, records, Iterator::min),
        crate::AggregationFunction::Max => extrema(property, records, Iterator::max),
    }
}

fn numeric_values<'a>(
    property: Option<&'a ApiName>,
    records: &'a [&Record],
) -> impl Iterator<Item = f64> + 'a {
    records
        .iter()
        .filter_map(move |record| property.and_then(|field| record.values.get(field)))
        .filter_map(Value::as_f64)
}

fn extrema<'a>(
    property: Option<&'a ApiName>,
    records: &'a [&Record],
    select: impl Fn(std::vec::IntoIter<Value>) -> Option<Value>,
) -> Value {
    let values: Vec<Value> = records
        .iter()
        .filter_map(|record| property.and_then(|field| record.values.get(field)).cloned())
        .collect();
    match select(values.into_iter()) {
        Some(value) => value,
        None => Value::null(),
    }
}
