//! Quality rule validation.

use crate::ports::QualityRuleEvaluator;
use lattice_core::Error;
use lattice_ir::{ObjectType, Record};
use regex::Regex;

/// Quality rule kinds understood by [`StaticQualityRuleEvaluator`].
mod kind {
    pub const NOT_NULL: &str = "not_null";
    pub const RANGE: &str = "range";
    pub const REGEX: &str = "regex";
    pub const ALLOWED_VALUES: &str = "allowed_values";
    pub const MIN_LENGTH: &str = "min_length";
    pub const MAX_LENGTH: &str = "max_length";
}

/// Evaluator that enforces quality rules declared in the spec without any
/// external dependencies (no database round-trips).
///
/// Supported rule kinds:
/// - `not_null` — the property must not be null or missing.
/// - `range` — numeric value must be within `[min, max]` (inclusive).
/// - `regex` — string value must match the given pattern.
/// - `allowed_values` — value must be one of the enumerated values.
/// - `min_length` / `max_length` — string length constraints.
pub struct StaticQualityRuleEvaluator;

impl QualityRuleEvaluator for StaticQualityRuleEvaluator {
    fn validate(&self, object_type: &ObjectType, record: &Record) -> Result<(), Error> {
        for rule in &object_type.quality_rules {
            let prop_name = match &rule.property {
                Some(p) => p,
                None => continue,
            };

            let value = record.values.get(prop_name);

            match rule.kind.as_str() {
                kind::NOT_NULL => {
                    let is_null = value.map(|v| v.is_null()).unwrap_or(true);
                    if is_null {
                        return Err(Error::validation(format!(
                            "quality rule '{}': property '{}' must not be null",
                            rule.api_name, prop_name
                        )));
                    }
                }
                kind::RANGE => {
                    if let Some(val) = value {
                        let num = val.as_f64().ok_or_else(|| {
                            Error::validation(format!(
                                "quality rule '{}': property '{}' must be numeric for range check",
                                rule.api_name, prop_name
                            ))
                        })?;
                        let args = rule.args.as_ref();
                        if let Some(min) = args.and_then(|a| a.get("min")).and_then(|v| v.as_f64())
                        {
                            if num < min {
                                return Err(Error::validation(format!(
                                    "quality rule '{}': property '{}' value {} is below minimum {}",
                                    rule.api_name, prop_name, num, min
                                )));
                            }
                        }
                        if let Some(max) = args.and_then(|a| a.get("max")).and_then(|v| v.as_f64())
                        {
                            if num > max {
                                return Err(Error::validation(format!(
                                    "quality rule '{}': property '{}' value {} exceeds maximum {}",
                                    rule.api_name, prop_name, num, max
                                )));
                            }
                        }
                    }
                }
                kind::REGEX => {
                    if let Some(val) = value {
                        let s = val.as_str().ok_or_else(|| {
                            Error::validation(format!(
                                "quality rule '{}': property '{}' must be a string for regex check",
                                rule.api_name, prop_name
                            ))
                        })?;
                        let pattern = rule
                            .args
                            .as_ref()
                            .and_then(|a| a.get("pattern"))
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                Error::validation(format!(
                                    "quality rule '{}': missing 'pattern' argument",
                                    rule.api_name
                                ))
                            })?;
                        let re = Regex::new(pattern).map_err(|e| {
                            Error::validation(format!(
                                "quality rule '{}': invalid regex pattern: {}",
                                rule.api_name, e
                            ))
                        })?;
                        if !re.is_match(s) {
                            return Err(Error::validation(format!(
                                "quality rule '{}': property '{}' value does not match pattern '{}'",
                                rule.api_name, prop_name, pattern
                            )));
                        }
                    }
                }
                kind::ALLOWED_VALUES => {
                    if let Some(val) = value {
                        if !val.is_null() {
                            let allowed = rule
                                .args
                                .as_ref()
                                .and_then(|a| a.get("values"))
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .map(|v| lattice_core::Value::new(v.clone()))
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            if !allowed.is_empty() && !allowed.contains(val) {
                                return Err(Error::validation(format!(
                                    "quality rule '{}': property '{}' has disallowed value",
                                    rule.api_name, prop_name
                                )));
                            }
                        }
                    }
                }
                kind::MIN_LENGTH => {
                    if let Some(val) = value {
                        let s = val.as_str().unwrap_or("");
                        let min = rule
                            .args
                            .as_ref()
                            .and_then(|a| a.get("length"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0) as usize;
                        if s.len() < min {
                            return Err(Error::validation(format!(
                                "quality rule '{}': property '{}' is shorter than minimum length {}",
                                rule.api_name, prop_name, min
                            )));
                        }
                    }
                }
                kind::MAX_LENGTH => {
                    if let Some(val) = value {
                        let s = val.as_str().unwrap_or("");
                        let max = rule
                            .args
                            .as_ref()
                            .and_then(|a| a.get("length"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(i64::MAX) as usize;
                        if s.len() > max {
                            return Err(Error::validation(format!(
                                "quality rule '{}': property '{}' exceeds maximum length {}",
                                rule.api_name, prop_name, max
                            )));
                        }
                    }
                }
                _ => {
                    // Unknown rule kinds are silently skipped to allow forward compatibility.
                }
            }
        }
        Ok(())
    }
}
