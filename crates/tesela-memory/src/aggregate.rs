use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};
use tesela_ir::{AggregateResult, Record};
use tesela_runtime::query::Aggregation;

/// Compute aggregations over a filtered set of records.
///
/// Groups records by `group_by` properties, then applies each aggregation function.
pub fn compute(
    records: &[Record],
    group_by: &[ApiName],
    aggregations: &[Aggregation],
) -> AggregateResult {
    // Build groups: group key (ordered values) -> records.
    let mut groups: BTreeMap<Vec<Value>, Vec<&Record>> = BTreeMap::new();

    for record in records {
        let key: Vec<Value> = group_by
            .iter()
            .map(|col| record.values.get(col).cloned().unwrap_or_default())
            .collect();
        groups.entry(key).or_default().push(record);
    }

    let mut result_groups: Vec<BTreeMap<String, Value>> = Vec::new();

    if groups.is_empty() {
        // No records: emit a single zero-row group for aggregate-only queries.
        if group_by.is_empty() {
            let mut row: BTreeMap<String, Value> = BTreeMap::new();
            for agg in aggregations {
                let val = match agg.function.as_str() {
                    "count" => Value::integer(0),
                    "sum" | "avg" | "min" | "max" => Value::null(),
                    _ => Value::null(),
                };
                row.insert(agg.alias.clone(), val);
            }
            result_groups.push(row);
        }
        return AggregateResult {
            groups: result_groups,
        };
    }

    for (group_key, recs) in &groups {
        let mut row: BTreeMap<String, Value> = BTreeMap::new();

        // Insert group-by values.
        for (col, val) in group_by.iter().zip(group_key.iter()) {
            row.insert(col.to_string(), val.clone());
        }

        // Compute each aggregation.
        for agg in aggregations {
            let val = compute_agg(&agg.function, agg.property.as_ref(), recs);
            row.insert(agg.alias.clone(), val);
        }

        result_groups.push(row);
    }

    AggregateResult {
        groups: result_groups,
    }
}

fn compute_agg(function: &str, property: Option<&ApiName>, records: &[&Record]) -> Value {
    match function {
        "count" => Value::integer(records.len() as i64),

        "sum" => {
            let prop = match property {
                Some(p) => p,
                None => return Value::null(),
            };
            let sum: f64 = records
                .iter()
                .filter_map(|r| r.values.get(prop))
                .filter_map(|v| v.as_f64())
                .sum();
            Value::float(sum)
        }

        "avg" => {
            let prop = match property {
                Some(p) => p,
                None => return Value::null(),
            };
            let vals: Vec<f64> = records
                .iter()
                .filter_map(|r| r.values.get(prop))
                .filter_map(|v| v.as_f64())
                .collect();
            if vals.is_empty() {
                return Value::null();
            }
            let avg = vals.iter().sum::<f64>() / vals.len() as f64;
            Value::float(avg)
        }

        "min" => {
            let prop = match property {
                Some(p) => p,
                None => return Value::null(),
            };
            records
                .iter()
                .filter_map(|r| r.values.get(prop))
                .min()
                .cloned()
                .unwrap_or_default()
        }

        "max" => {
            let prop = match property {
                Some(p) => p,
                None => return Value::null(),
            };
            records
                .iter()
                .filter_map(|r| r.values.get(prop))
                .max()
                .cloned()
                .unwrap_or_default()
        }

        _ => Value::null(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(pairs: &[(&str, serde_json::Value)]) -> Record {
        Record {
            primary_key: None,
            values: pairs
                .iter()
                .map(|(k, v)| (ApiName::new_unchecked(k), Value::new(v.clone())))
                .collect(),
        }
    }

    #[test]
    fn test_count() {
        let records = vec![
            make_record(&[
                ("dept", serde_json::json!("eng")),
                ("salary", serde_json::json!(100.0)),
            ]),
            make_record(&[
                ("dept", serde_json::json!("eng")),
                ("salary", serde_json::json!(200.0)),
            ]),
            make_record(&[
                ("dept", serde_json::json!("hr")),
                ("salary", serde_json::json!(150.0)),
            ]),
        ];

        let group_by = vec![ApiName::new_unchecked("dept")];
        let aggs = vec![
            Aggregation {
                function: "count".to_string(),
                property: None,
                alias: "n".to_string(),
            },
            Aggregation {
                function: "avg".to_string(),
                property: Some(ApiName::new_unchecked("salary")),
                alias: "avg_sal".to_string(),
            },
        ];

        let result = compute(&records, &group_by, &aggs);
        assert_eq!(result.groups.len(), 2);

        let eng = result
            .groups
            .iter()
            .find(|g| g.get("dept").and_then(|v| v.as_str()) == Some("eng"))
            .unwrap();
        assert_eq!(eng.get("n").and_then(|v| v.as_i64()), Some(2));
        assert!((eng.get("avg_sal").and_then(|v| v.as_f64()).unwrap() - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_min_max() {
        let records = vec![
            make_record(&[("age", serde_json::json!(10))]),
            make_record(&[("age", serde_json::json!(30))]),
            make_record(&[("age", serde_json::json!(20))]),
        ];

        let aggs = vec![
            Aggregation {
                function: "min".to_string(),
                property: Some(ApiName::new_unchecked("age")),
                alias: "min_age".to_string(),
            },
            Aggregation {
                function: "max".to_string(),
                property: Some(ApiName::new_unchecked("age")),
                alias: "max_age".to_string(),
            },
        ];

        let result = compute(&records, &[], &aggs);
        assert_eq!(result.groups.len(), 1);
        let row = &result.groups[0];
        assert_eq!(row.get("min_age").and_then(|v| v.as_i64()), Some(10));
        assert_eq!(row.get("max_age").and_then(|v| v.as_i64()), Some(30));
    }
}
