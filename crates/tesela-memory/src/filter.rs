use tesela_core::{FilterOp, Value};
use tesela_ir::Filter;
use std::collections::BTreeMap;

/// Evaluate a filter against a record's field values.
///
/// Returns `true` if the record matches the filter.
pub fn evaluate(filter: &Filter, record: &BTreeMap<tesela_core::ApiName, Value>) -> bool {
    match filter.op {
        FilterOp::And => {
            if filter.args.is_empty() {
                return true;
            }
            filter.args.iter().all(|f| evaluate(f, record))
        }
        FilterOp::Or => {
            if filter.args.is_empty() {
                return true;
            }
            filter.args.iter().any(|f| evaluate(f, record))
        }
        FilterOp::Not => filter.args.first().is_none_or(|f| !evaluate(f, record)),
        _ => {
            let field = match &filter.field {
                Some(f) => f,
                None => return true,
            };
            let field_val = record.get(field);
            eval_scalar(filter.op, field_val, &filter.value, &filter.values)
        }
    }
}

fn eval_scalar(
    op: FilterOp,
    field_val: Option<&Value>,
    filter_val: &Option<Value>,
    filter_vals: &[Value],
) -> bool {
    match op {
        FilterOp::IsNull => field_val.is_none_or(|v| v.is_null()),
        FilterOp::IsNotNull => field_val.is_some_and(|v| !v.is_null()),

        FilterOp::Eq => {
            let fv = match filter_val {
                Some(v) => v,
                None => return field_val.is_none_or(|v| v.is_null()),
            };
            field_val.is_some_and(|v| v == fv)
        }

        FilterOp::Ne => {
            let fv = match filter_val {
                Some(v) => v,
                None => return field_val.is_some_and(|v| !v.is_null()),
            };
            field_val.is_none_or(|v| v != fv)
        }

        FilterOp::Lt => {
            let fv = match filter_val {
                Some(v) => v,
                None => return false,
            };
            field_val.is_some_and(|v| v < fv)
        }

        FilterOp::Lte => {
            let fv = match filter_val {
                Some(v) => v,
                None => return false,
            };
            field_val.is_some_and(|v| v <= fv)
        }

        FilterOp::Gt => {
            let fv = match filter_val {
                Some(v) => v,
                None => return false,
            };
            field_val.is_some_and(|v| v > fv)
        }

        FilterOp::Gte => {
            let fv = match filter_val {
                Some(v) => v,
                None => return false,
            };
            field_val.is_some_and(|v| v >= fv)
        }

        FilterOp::Like => {
            let pattern = match filter_val {
                Some(v) => match v.as_str() {
                    Some(s) => s.to_string(),
                    None => return false,
                },
                None => return false,
            };
            field_val
                .and_then(|v| v.as_str())
                .is_some_and(|s| like_match(s, &pattern))
        }

        FilterOp::StartsWith => {
            let prefix = match filter_val {
                Some(v) => match v.as_str() {
                    Some(s) => s.to_string(),
                    None => return false,
                },
                None => return false,
            };
            field_val
                .and_then(|v| v.as_str())
                .is_some_and(|s| s.starts_with(prefix.as_str()))
        }

        FilterOp::Contains => {
            let needle = match filter_val {
                Some(v) => match v.as_str() {
                    Some(s) => s.to_string(),
                    None => return false,
                },
                None => return false,
            };
            field_val
                .and_then(|v| v.as_str())
                .is_some_and(|s| s.contains(needle.as_str()))
        }

        FilterOp::Between => {
            if filter_vals.len() < 2 {
                return false;
            }
            let lo = &filter_vals[0];
            let hi = &filter_vals[1];
            field_val.is_some_and(|v| v >= lo && v <= hi)
        }

        FilterOp::In => {
            if filter_vals.is_empty() {
                return false;
            }
            field_val.is_some_and(|v| filter_vals.contains(v))
        }

        FilterOp::NotIn => {
            if filter_vals.is_empty() {
                return true;
            }
            field_val.is_none_or(|v| !filter_vals.contains(v))
        }

        // Logical ops handled above; this branch is unreachable in normal flow.
        FilterOp::And | FilterOp::Or | FilterOp::Not => true,
    }
}

/// SQL-style LIKE matching: `%` matches any sequence, `_` matches any single char.
fn like_match(text: &str, pattern: &str) -> bool {
    fn matches(t: &[char], p: &[char]) -> bool {
        match (t, p) {
            (_, []) => t.is_empty(),
            (_, ['%', rest @ ..]) => (0..=t.len()).any(|i| matches(&t[i..], rest)),
            ([], _) => false,
            ([_tc, t_rest @ ..], ['_', p_rest @ ..]) => matches(t_rest, p_rest),
            ([tc, t_rest @ ..], [pc, p_rest @ ..]) => tc == pc && matches(t_rest, p_rest),
        }
    }
    let tc: Vec<char> = text.chars().collect();
    let pc: Vec<char> = pattern.chars().collect();
    matches(&tc, &pc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesela_core::ApiName;

    fn rec(pairs: &[(&str, serde_json::Value)]) -> BTreeMap<ApiName, Value> {
        pairs
            .iter()
            .map(|(k, v)| (ApiName::new_unchecked(k), Value::new(v.clone())))
            .collect()
    }

    fn filt(op: FilterOp, field: &str, val: serde_json::Value) -> Filter {
        Filter {
            op,
            field: Some(ApiName::new_unchecked(field)),
            value: Some(Value::new(val)),
            values: Vec::new(),
            args: Vec::new(),
        }
    }

    #[test]
    fn test_eq() {
        let r = rec(&[("name", serde_json::json!("Alice"))]);
        assert!(evaluate(
            &filt(FilterOp::Eq, "name", serde_json::json!("Alice")),
            &r
        ));
        assert!(!evaluate(
            &filt(FilterOp::Eq, "name", serde_json::json!("Bob")),
            &r
        ));
    }

    #[test]
    fn test_like() {
        let r = rec(&[("name", serde_json::json!("Alice"))]);
        let f = Filter {
            op: FilterOp::Like,
            field: Some(ApiName::new_unchecked("name")),
            value: Some(Value::string("Al%")),
            values: Vec::new(),
            args: Vec::new(),
        };
        assert!(evaluate(&f, &r));

        let f2 = Filter {
            op: FilterOp::Like,
            field: Some(ApiName::new_unchecked("name")),
            value: Some(Value::string("%ice")),
            values: Vec::new(),
            args: Vec::new(),
        };
        assert!(evaluate(&f2, &r));

        let f3 = Filter {
            op: FilterOp::Like,
            field: Some(ApiName::new_unchecked("name")),
            value: Some(Value::string("%ob%")),
            values: Vec::new(),
            args: Vec::new(),
        };
        assert!(!evaluate(&f3, &r));
    }

    #[test]
    fn test_between() {
        let r = rec(&[("age", serde_json::json!(25))]);
        let f = Filter {
            op: FilterOp::Between,
            field: Some(ApiName::new_unchecked("age")),
            value: None,
            values: vec![Value::integer(20), Value::integer(30)],
            args: Vec::new(),
        };
        assert!(evaluate(&f, &r));
    }

    #[test]
    fn test_and_or_not() {
        let r = rec(&[
            ("age", serde_json::json!(25)),
            ("name", serde_json::json!("Alice")),
        ]);
        let age_ok = filt(FilterOp::Gt, "age", serde_json::json!(20));
        let name_ok = filt(FilterOp::Eq, "name", serde_json::json!("Alice"));
        let name_bad = filt(FilterOp::Eq, "name", serde_json::json!("Bob"));

        let and_f = Filter {
            op: FilterOp::And,
            field: None,
            value: None,
            values: Vec::new(),
            args: vec![age_ok.clone(), name_ok.clone()],
        };
        assert!(evaluate(&and_f, &r));

        let or_f = Filter {
            op: FilterOp::Or,
            field: None,
            value: None,
            values: Vec::new(),
            args: vec![name_bad.clone(), name_ok.clone()],
        };
        assert!(evaluate(&or_f, &r));

        let not_f = Filter {
            op: FilterOp::Not,
            field: None,
            value: None,
            values: Vec::new(),
            args: vec![name_bad],
        };
        assert!(evaluate(&not_f, &r));
    }

    #[test]
    fn test_in_not_in() {
        let r = rec(&[("status", serde_json::json!("active"))]);
        let in_f = Filter {
            op: FilterOp::In,
            field: Some(ApiName::new_unchecked("status")),
            value: None,
            values: vec![Value::string("active"), Value::string("pending")],
            args: Vec::new(),
        };
        assert!(evaluate(&in_f, &r));

        let not_in_f = Filter {
            op: FilterOp::NotIn,
            field: Some(ApiName::new_unchecked("status")),
            value: None,
            values: vec![Value::string("deleted"), Value::string("banned")],
            args: Vec::new(),
        };
        assert!(evaluate(&not_in_f, &r));
    }
}
