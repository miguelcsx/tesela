//! Schema-aware upload mapping, heuristic column matching, type coercion,
//! and post-write validation.

use tesela_core::{DataType, Error, Value};
use tesela_ir::{ObjectType, Property};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Mapping engine
// ---------------------------------------------------------------------------

/// Maps incoming column names to object-type properties.
pub struct UploadMappingEngine {
    /// Explicit column → property mappings from the spec.
    pub explicit: BTreeMap<String, String>,
    /// Whether to run fuzzy fallback matching.
    pub heuristic: bool,
}

impl UploadMappingEngine {
    /// Build from an object type's column mappings.
    pub fn from_object_type(ot: &ObjectType) -> Self {
        let mut explicit = BTreeMap::new();
        for prop in &ot.properties {
            if let Some(ref col) = prop.source_column {
                explicit.insert(col.clone(), prop.api_name.to_string());
            }
        }
        Self {
            explicit,
            heuristic: true,
        }
    }

    /// Map a raw column name to a property name.
    pub fn map_column(&self, col: &str, ot: &ObjectType) -> Option<String> {
        // 1. Explicit mapping
        if let Some(mapped) = self.explicit.get(col) {
            return Some(mapped.clone());
        }
        // 2. Exact match on property api_name
        if ot.properties.iter().any(|p| p.api_name.to_string() == col) {
            return Some(col.to_string());
        }
        // 3. Heuristic fuzzy match
        if self.heuristic
            && let Some(best) = fuzzy_match(col, &ot.properties)
        {
            return Some(best);
        }
        None
    }
}

/// Simple fuzzy match: returns the property name with the highest
/// Jaro-Winkler-like similarity above a threshold.
fn fuzzy_match(col: &str, properties: &[Property]) -> Option<String> {
    let col_lower = col.to_lowercase();
    let mut best: Option<(String, f64)> = None;
    for prop in properties {
        let name = prop.api_name.to_string().to_lowercase();
        let sim = similarity(&col_lower, &name);
        if sim > 0.8 {
            best = match best {
                None => Some((prop.api_name.to_string(), sim)),
                Some((_, b)) if sim > b => Some((prop.api_name.to_string(), sim)),
                other => other,
            };
        }
    }
    best.map(|(name, _)| name)
}

/// Normalised Levenshtein distance (0.0 = identical, 1.0 = completely different).
fn similarity(a: &str, b: &str) -> f64 {
    let dist = levenshtein(a, b) as f64;
    let max_len = a.len().max(b.len()) as f64;
    if max_len == 0.0 {
        return 1.0;
    }
    1.0 - (dist / max_len)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (curr[j - 1] + 1).min(prev[j] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

// ---------------------------------------------------------------------------
// Type coercion
// ---------------------------------------------------------------------------

/// Coerce a raw string value into the target property's data type.
pub fn coerce_value(raw: &str, prop: &Property) -> Result<Value, Error> {
    if raw.is_empty() && prop.nullable == Some(true) {
        return Ok(Value::null());
    }
    match prop.data_type {
        DataType::Integer | DataType::BigInt => raw.parse::<i64>().map(Value::from).map_err(|_| {
            Error::validation(format!(
                "cannot parse '{}' as integer for property '{}'",
                raw, prop.api_name
            ))
        }),
        DataType::Float | DataType::Decimal => raw.parse::<f64>().map(Value::from).map_err(|_| {
            Error::validation(format!(
                "cannot parse '{}' as float for property '{}'",
                raw, prop.api_name
            ))
        }),
        DataType::Boolean => {
            let v = match raw.to_lowercase().as_str() {
                "true" | "1" | "yes" | "t" | "y" => true,
                "false" | "0" | "no" | "f" | "n" => false,
                _ => {
                    return Err(Error::validation(format!(
                        "cannot parse '{}' as boolean for property '{}'",
                        raw, prop.api_name
                    )));
                }
            };
            Ok(Value::from(v))
        }
        DataType::Timestamp | DataType::Date => chrono::DateTime::parse_from_rfc3339(raw)
            .map(|dt| Value::from(dt.to_rfc3339()))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
                    .map(|dt| Value::from(dt.to_string()))
            })
            .map_err(|_| {
                Error::validation(format!(
                    "cannot parse '{}' as datetime for property '{}'",
                    raw, prop.api_name
                ))
            }),
        DataType::Uuid => uuid::Uuid::parse_str(raw)
            .map(|u| Value::from(u.to_string()))
            .map_err(|_| {
                Error::validation(format!(
                    "cannot parse '{}' as uuid for property '{}'",
                    raw, prop.api_name
                ))
            }),
        _ => Ok(Value::from(raw)),
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate a batch of records against an object type's schema.
pub fn validate_records(records: &[tesela_ir::Record], ot: &ObjectType) -> Result<(), Error> {
    let mut errors = Vec::new();
    for (idx, record) in records.iter().enumerate() {
        for prop in &ot.properties {
            let key = &prop.api_name;
            if prop.nullable != Some(true) && !record.values.contains_key(key) {
                errors.push(format!("row {}: missing required field '{}'", idx, key));
            }
            if let Some(value) = record.values.get(key)
                && value.is_null()
                && prop.nullable != Some(true)
            {
                errors.push(format!(
                    "row {}: null value for required field '{}'",
                    idx, key
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::validation(errors.join("; ")))
    }
}
