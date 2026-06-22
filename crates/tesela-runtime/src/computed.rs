//! Computed property evaluation.

use crate::ports::{ComputedEvaluator, ComputedLang, RecordContext};
use tesela_core::Error;
use tesela_ir::{ObjectType, Record};

/// Materialise all computed properties on a record in-place.
///
/// Iterates over every property on `object_type` that carries a `computed`
/// expression, evaluates it, and writes the result back into `record.values`.
/// Properties without a `computed` expression are left untouched.
///
/// The function is a no-op when the object type has no computed properties.
/// If computed properties are declared, an evaluator is required.
pub fn materialize_computed(
    record: &mut Record,
    object_type: &ObjectType,
    evaluator: Option<&dyn ComputedEvaluator>,
) -> Result<(), Error> {
    for prop in &object_type.properties {
        let computed = match &prop.computed {
            Some(c) => c,
            None => continue,
        };
        let eval = evaluator.ok_or_else(|| {
            Error::unsupported(format!(
                "computed property '{}' requires a computed evaluator",
                prop.api_name
            ))
        })?;
        let lang = ComputedLang::from_spec_str(&computed.language);
        let ctx = RecordContext {
            record,
            object_type: &object_type.api_name,
        };
        let val = eval.evaluate(lang, &computed.expression, &ctx)?;
        record.values.insert(prop.api_name.clone(), val);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tesela_core::{ApiName, DataType};
    use tesela_ir::{Computed, ObjectSource, ObjectType, Property, Record};

    use super::materialize_computed;

    #[test]
    fn materialize_computed_ignores_plain_object_without_evaluator() {
        let mut record = Record {
            primary_key: None,
            values: BTreeMap::new(),
        };
        materialize_computed(&mut record, &object_type(None), None).expect("no computed fields");
    }

    #[test]
    fn materialize_computed_requires_evaluator_for_computed_property() {
        let mut record = Record {
            primary_key: None,
            values: BTreeMap::new(),
        };
        let error = materialize_computed(
            &mut record,
            &object_type(Some(Computed {
                language: "cel".to_string(),
                expression: "first_name + last_name".to_string(),
            })),
            None,
        )
        .expect_err("computed fields require an evaluator");

        assert!(error.to_string().contains("computed evaluator"));
    }

    fn object_type(computed: Option<Computed>) -> ObjectType {
        ObjectType {
            api_name: ApiName::new_unchecked("person"),
            display: None,
            description: None,
            source: ObjectSource {
                datasource: ApiName::new_unchecked("memory"),
                resource: None,
            },
            primary_key: ApiName::new_unchecked("id"),
            properties: vec![Property {
                api_name: ApiName::new_unchecked("full_name"),
                display: None,
                description: None,
                data_type: DataType::String,
                nullable: None,
                indexed: None,
                unique: None,
                tags: Vec::new(),
                markings: Vec::new(),
                default: None,
                computed,
                source_column: None,
                allowed_values: None,
                sort_order: None,
                metadata: None,
                encrypted: None,
                quality: Vec::new(),
            }],
            traits: Vec::new(),
            tags: Vec::new(),
            metadata: None,
            indexes: Vec::new(),
            temporal: None,
            lifecycle: None,
            scoring: None,
            classification: None,
            quality_rules: Vec::new(),
            lineage: Vec::new(),
            deprecated_at: None,
        }
    }
}
