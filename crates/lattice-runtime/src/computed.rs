//! Computed property evaluation.

use crate::ports::{ComputedEvaluator, ComputedLang, RecordContext};
use lattice_core::{Error, Value};
use lattice_ir::{ObjectType, Record};

/// Evaluator that returns `Value::Null` for every expression.
///
/// Use this as a placeholder when computed properties are declared in the spec
/// but no expression engine (CEL, SQL, Python) is configured.  It keeps all
/// non-computed fields intact and silently zeroes computed ones.
pub struct NoopComputedEvaluator;

impl ComputedEvaluator for NoopComputedEvaluator {
    fn evaluate(
        &self,
        _lang: ComputedLang,
        _expr: &str,
        _ctx: &RecordContext<'_>,
    ) -> Result<Value, Error> {
        Ok(Value::null())
    }
}

/// Materialise all computed properties on a record in-place.
///
/// Iterates over every property on `object_type` that carries a `computed`
/// expression, evaluates it, and writes the result back into `record.values`.
/// Properties without a `computed` expression are left untouched.
///
/// The function is a no-op when `evaluator` is `None`.
pub fn materialize_computed(
    record: &mut Record,
    object_type: &ObjectType,
    evaluator: Option<&dyn ComputedEvaluator>,
) {
    let eval = match evaluator {
        Some(e) => e,
        None => return,
    };

    for prop in &object_type.properties {
        let computed = match &prop.computed {
            Some(c) => c,
            None => continue,
        };
        let lang = ComputedLang::from_spec_str(&computed.language);
        let ctx = RecordContext {
            record,
            object_type: &object_type.api_name,
        };
        match eval.evaluate(lang, &computed.expression, &ctx) {
            Ok(val) => {
                record.values.insert(prop.api_name.clone(), val);
            }
            Err(_) => {
                // Non-fatal: leave the field absent rather than crashing.
            }
        }
    }
}
