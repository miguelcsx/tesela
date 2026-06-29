//! Runtime data records and result types.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Error, Value};

/// A single record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// Primary key value.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub primary_key: Option<Value>,
    /// Field values keyed by property API name.
    #[serde(default)]
    pub values: BTreeMap<ApiName, Value>,
}

impl Record {
    fn key(key: &str) -> ApiName {
        ApiName::new_unchecked(key)
    }

    /// Return a field value by string key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(&Self::key(key))
    }

    /// Return a string field.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }

    /// Return a field as JSON.
    pub fn get_json(&self, key: &str) -> Option<serde_json::Value> {
        self.get(key).cloned().map(serde_json::Value::from)
    }

    /// Decode a field into a typed value.
    pub fn decode<T: DeserializeOwned>(&self, key: &str) -> Result<T, Error> {
        let json = self
            .get_json(key)
            .ok_or_else(|| Error::bad_request(format!("{key} is missing")))?;
        serde_json::from_value(json).map_err(|error| Error::bad_request(error.to_string()))
    }

    /// Consume the record into a JSON object map.
    pub fn into_json_map(self) -> serde_json::Map<String, serde_json::Value> {
        self.values
            .into_iter()
            .map(|(key, value)| (key.to_string(), serde_json::Value::from(value)))
            .collect()
    }
}

impl From<Record> for serde_json::Value {
    fn from(record: Record) -> Self {
        serde_json::Value::Object(record.into_json_map())
    }
}

/// A page of records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    /// Records in this page.
    #[serde(default)]
    pub records: Vec<Record>,
    /// Next page cursor.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub next_cursor: Option<String>,
}

/// Result of a mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationResult {
    /// Mutated record, when available.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub record: Option<Record>,
    /// Number of affected rows.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rows_affected: Option<i64>,
}

/// Result of an action execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionResult {
    /// Status: success, failed, rejected, queued.
    pub status: String,
    /// Output payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<Value>,
    /// Error message.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    /// Run ID.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub run_id: Option<String>,
}

/// Result of an aggregate query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateResult {
    /// Aggregated groups.
    #[serde(default)]
    pub groups: Vec<BTreeMap<String, Value>>,
}
