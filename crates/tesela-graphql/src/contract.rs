//! Small helpers for GraphQL response contracts built from Tesela records.

use serde_json::{Map, Value as JsonValue, json};
use tesela_core::ApiName;
use tesela_ir::Record;

/// Field mapping from a Tesela record property to a GraphQL response field.
#[derive(Debug, Clone)]
pub struct FieldProjection {
    /// GraphQL field name returned to the client.
    pub response_name: &'static str,
    /// Tesela record property name.
    pub record_name: &'static str,
    /// Value used when the record does not contain the property.
    pub default: Option<JsonValue>,
}

impl FieldProjection {
    /// Create a projection without a default value.
    pub fn new(response_name: &'static str, record_name: &'static str) -> Self {
        Self {
            response_name,
            record_name,
            default: None,
        }
    }

    /// Create a projection with a default value.
    pub fn with_default(
        response_name: &'static str,
        record_name: &'static str,
        default: JsonValue,
    ) -> Self {
        Self {
            response_name,
            record_name,
            default: Some(default),
        }
    }
}

/// Project a Tesela record into a GraphQL-facing JSON object.
pub fn project_record(record: &Record, fields: &[FieldProjection]) -> JsonValue {
    let mut object = Map::new();
    for field in fields {
        let value = record_value(record, field.record_name)
            .or_else(|| field.default.clone())
            .unwrap_or(JsonValue::Null);
        object.insert(field.response_name.to_string(), value);
    }
    JsonValue::Object(object)
}

/// Build the common GraphQL page shape used by list fields.
pub fn graphql_page(items: Vec<JsonValue>, page: usize, page_size: usize) -> JsonValue {
    let total = items.len();
    json!({
        "items": items,
        "total": total,
        "page": page,
        "pageSize": page_size
    })
}

/// Return true when a GraphQL document references a field or operation name.
pub fn graphql_has_field(document: &str, field: &str) -> bool {
    document.match_indices(field).any(|(start, _)| {
        let before = document[..start].chars().next_back();
        let after = document[start + field.len()..].chars().next();
        !before.is_some_and(is_ident_char) && !after.is_some_and(is_ident_char)
    })
}

/// Return true when a GraphQL document references any of the provided fields.
pub fn graphql_has_any_field<'a>(
    document: &str,
    fields: impl IntoIterator<Item = &'a str>,
) -> bool {
    fields
        .into_iter()
        .any(|field| graphql_has_field(document, field))
}

fn record_value(record: &Record, key: &str) -> Option<JsonValue> {
    record
        .values
        .get(&ApiName::new_unchecked(key))
        .map(|value| value.0.clone())
}

fn is_ident_char(value: char) -> bool {
    value.is_ascii_alphanumeric() || value == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tesela_core::Value;

    #[test]
    fn projects_record_fields_with_defaults() {
        let mut values = BTreeMap::new();
        values.insert(ApiName::new_unchecked("id"), Value::string("org-1"));
        let record = Record {
            primary_key: Some(Value::string("org-1")),
            values,
        };

        let projected = project_record(
            &record,
            &[
                FieldProjection::new("id", "id"),
                FieldProjection::with_default("isActive", "is_active", json!(true)),
            ],
        );

        assert_eq!(projected["id"], json!("org-1"));
        assert_eq!(projected["isActive"], json!(true));
    }

    #[test]
    fn builds_page_shape() {
        let page = graphql_page(vec![json!({"id": "one"})], 1, 25);

        assert_eq!(page["total"], json!(1));
        assert_eq!(page["page"], json!(1));
        assert_eq!(page["pageSize"], json!(25));
        assert_eq!(page["items"][0]["id"], json!("one"));
    }

    #[test]
    fn detects_graphql_fields_on_identifier_boundaries() {
        let query = "query { scenarioStats { totalTrips } scenario(id: $id) { id } }";

        assert!(graphql_has_field(query, "scenarioStats"));
        assert!(graphql_has_field(query, "scenario"));
        assert!(!graphql_has_field(query, "scenarioStat"));
        assert!(graphql_has_any_field(query, ["zones", "scenarioStats"]));
    }
}
