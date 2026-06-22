//! BigQuery backend adapter for Tesela.

mod sql;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::{Value as JsonValue, json};
use tesela_core::{ApiName, Error, Value};
use tesela_ir::{AggregateResult, Datasource, MutationResult, Page, Record, Spec};
use tesela_runtime::{
    ports::{Aggregator, Backend, BackendFactory, Getter, Mutator, Searcher},
    query::{AggregateQuery, BackendCapabilities, Mutation, Query},
};

use crate::sql::{
    aggregate_sql, create_table_sql, delete_sql, get_sql, insert_sql, search_sql, update_sql,
    upsert_sql,
};

/// BigQuery adapter configuration.
#[derive(Debug, Clone)]
pub struct BigQueryConfig {
    /// Google Cloud project ID.
    pub project_id: String,
    /// BigQuery dataset ID.
    pub dataset: String,
    /// Optional BigQuery job location, for example `US` or `EU`.
    pub location: Option<String>,
    /// Optional OAuth access token. If absent, the adapter uses metadata server auth.
    pub access_token: Option<String>,
}

impl BigQueryConfig {
    /// Build configuration from a Tesela datasource.
    pub fn from_datasource(ds: &Datasource) -> Result<Self, Error> {
        let config = ds
            .config
            .as_ref()
            .ok_or_else(|| Error::bad_request("bigquery datasource config is required"))?;
        Ok(Self {
            project_id: string_config(config, "project_id")?,
            dataset: string_config(config, "dataset")?,
            location: optional_string_config(config, "location"),
            access_token: optional_string_config(config, "access_token")
                .or_else(|| std::env::var("GOOGLE_OAUTH_ACCESS_TOKEN").ok()),
        })
    }
}

/// BigQuery-backed implementation of the Tesela `Backend` trait.
pub struct BigQueryBackend {
    config: BigQueryConfig,
    client: Client,
}

impl BigQueryBackend {
    /// Create a backend from explicit configuration.
    pub fn new(config: BigQueryConfig) -> Result<Arc<Self>, Error> {
        validate_identifier("project_id", &config.project_id)?;
        validate_identifier("dataset", &config.dataset)?;
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| Error::adapter_with_source("bigquery client", error.to_string()))?;
        Ok(Arc::new(Self { config, client }))
    }

    /// Create a backend from a Tesela datasource.
    pub fn open(ds: &Datasource) -> Result<Arc<Self>, Error> {
        Self::new(BigQueryConfig::from_datasource(ds)?)
    }

    /// Create missing BigQuery tables for object types owned by the selected datasources.
    ///
    /// Existing tables are left untouched. This is intentionally additive so callers can run it at
    /// startup without performing destructive schema migration.
    pub fn ensure_tables_for_datasources(
        &self,
        spec: &Spec,
        datasources: &[ApiName],
    ) -> Result<(), Error> {
        for object_type in &spec.object_types {
            if datasources.contains(&object_type.source.datasource) {
                self.query(
                    create_table_sql(&self.config.project_id, &self.config.dataset, object_type)?,
                    Vec::new(),
                )?;
            }
        }
        Ok(())
    }

    fn query(&self, sql: String, params: Vec<QueryParam>) -> Result<QueryResponse, Error> {
        let url = format!(
            "https://bigquery.googleapis.com/bigquery/v2/projects/{}/queries",
            self.config.project_id
        );
        let mut body = json!({
            "query": sql,
            "useLegacySql": false,
            "parameterMode": "NAMED",
            "queryParameters": params,
        });
        if let Some(location) = &self.config.location {
            body["location"] = json!(location);
        }
        let response = self
            .client
            .post(url)
            .bearer_auth(self.token()?)
            .json(&body)
            .send()
            .map_err(|error| Error::adapter_with_source("bigquery query", error.to_string()))?;
        if !response.status().is_success() {
            return Err(Error::adapter(format!(
                "bigquery query failed: {} {}",
                response.status(),
                response.text().unwrap_or_default()
            )));
        }
        response.json::<QueryResponse>().map_err(|error| {
            Error::adapter_with_source("bigquery query response", error.to_string())
        })
    }

    fn token(&self) -> Result<String, Error> {
        if let Some(token) = &self.config.access_token {
            return Ok(token.clone());
        }
        let response = self
            .client
            .get("http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token")
            .header("Metadata-Flavor", "Google")
            .send()
            .map_err(|error| Error::unauthorized(format!("metadata token unavailable: {error}")))?;
        if !response.status().is_success() {
            return Err(Error::unauthorized(format!(
                "metadata token unavailable: {}",
                response.status()
            )));
        }
        response
            .json::<MetadataToken>()
            .map(|token| token.access_token)
            .map_err(|error| Error::unauthorized(format!("metadata token parse failed: {error}")))
    }
}

impl Backend for BigQueryBackend {
    fn backend_type(&self) -> &str {
        "bigquery"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            search: true,
            get: true,
            mutate: true,
            aggregate: true,
            traverse: false,
            bulk_load: false,
            rollback: false,
            explain: false,
        }
    }

    fn close(&self) -> Result<(), Error> {
        Ok(())
    }

    fn as_searcher(&self) -> Option<&dyn Searcher> {
        Some(self)
    }

    fn as_getter(&self) -> Option<&dyn Getter> {
        Some(self)
    }

    fn as_mutator(&self) -> Option<&dyn Mutator> {
        Some(self)
    }

    fn as_aggregator(&self) -> Option<&dyn Aggregator> {
        Some(self)
    }
}

impl Searcher for BigQueryBackend {
    fn search(&self, object_type: &ApiName, query: &Query) -> Result<Page, Error> {
        let (sql, params) = search_sql(
            &self.config.project_id,
            &self.config.dataset,
            object_type,
            query,
        )?;
        let response = self.query(sql, params)?;
        Ok(Page {
            records: response.records(object_type),
            next_cursor: None,
        })
    }
}

impl Getter for BigQueryBackend {
    fn get(&self, object_type: &ApiName, pk: &Value) -> Result<Option<Record>, Error> {
        let (sql, params) = get_sql(
            &self.config.project_id,
            &self.config.dataset,
            object_type,
            pk,
        )?;
        Ok(self
            .query(sql, params)?
            .records(object_type)
            .into_iter()
            .next())
    }
}

impl Mutator for BigQueryBackend {
    fn mutate(&self, object_type: &ApiName, mutation: &Mutation) -> Result<MutationResult, Error> {
        let (sql, params) = match mutation {
            Mutation::Create { values } => insert_sql(
                &self.config.project_id,
                &self.config.dataset,
                object_type,
                values,
            )?,
            Mutation::Update {
                primary_key,
                values,
            } => update_sql(
                &self.config.project_id,
                &self.config.dataset,
                object_type,
                primary_key,
                values,
            )?,
            Mutation::Delete { primary_key } => delete_sql(
                &self.config.project_id,
                &self.config.dataset,
                object_type,
                primary_key,
            )?,
            Mutation::Upsert { values } => upsert_sql(
                &self.config.project_id,
                &self.config.dataset,
                object_type,
                values,
            )?,
            Mutation::Batch { items } => {
                let mut rows = 0;
                for item in items {
                    rows += self.mutate(object_type, item)?.rows_affected.unwrap_or(0);
                }
                return Ok(MutationResult {
                    record: None,
                    rows_affected: Some(rows),
                });
            }
        };
        self.query(sql, params)?;
        Ok(MutationResult {
            record: None,
            rows_affected: Some(1),
        })
    }
}

impl Aggregator for BigQueryBackend {
    fn aggregate(
        &self,
        object_type: &ApiName,
        query: &AggregateQuery,
    ) -> Result<AggregateResult, Error> {
        let (sql, params) = aggregate_sql(
            &self.config.project_id,
            &self.config.dataset,
            object_type,
            query,
        )?;
        Ok(self.query(sql, params)?.aggregate_result())
    }
}

/// Factory for opening BigQuery backends from datasource config.
#[derive(Default)]
pub struct BigQueryFactory;

impl BackendFactory for BigQueryFactory {
    fn factory_type(&self) -> &str {
        "bigquery"
    }

    fn open(&self, ds: &Datasource) -> Result<Box<dyn Backend>, Error> {
        Ok(Box::new(BigQueryBackendHandle(BigQueryBackend::open(ds)?)))
    }
}

struct BigQueryBackendHandle(Arc<BigQueryBackend>);

impl Backend for BigQueryBackendHandle {
    fn backend_type(&self) -> &str {
        self.0.backend_type()
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.0.capabilities()
    }

    fn close(&self) -> Result<(), Error> {
        self.0.close()
    }

    fn as_searcher(&self) -> Option<&dyn Searcher> {
        Some(self)
    }

    fn as_getter(&self) -> Option<&dyn Getter> {
        Some(self)
    }

    fn as_mutator(&self) -> Option<&dyn Mutator> {
        Some(self)
    }

    fn as_aggregator(&self) -> Option<&dyn Aggregator> {
        Some(self)
    }
}

impl Searcher for BigQueryBackendHandle {
    fn search(&self, object_type: &ApiName, query: &Query) -> Result<Page, Error> {
        self.0.search(object_type, query)
    }
}

impl Getter for BigQueryBackendHandle {
    fn get(&self, object_type: &ApiName, pk: &Value) -> Result<Option<Record>, Error> {
        self.0.get(object_type, pk)
    }
}

impl Mutator for BigQueryBackendHandle {
    fn mutate(&self, object_type: &ApiName, mutation: &Mutation) -> Result<MutationResult, Error> {
        self.0.mutate(object_type, mutation)
    }
}

impl Aggregator for BigQueryBackendHandle {
    fn aggregate(
        &self,
        object_type: &ApiName,
        query: &AggregateQuery,
    ) -> Result<AggregateResult, Error> {
        self.0.aggregate(object_type, query)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct QueryParam {
    name: String,
    #[serde(rename = "parameterType")]
    parameter_type: JsonValue,
    #[serde(rename = "parameterValue")]
    parameter_value: JsonValue,
}

impl QueryParam {
    pub(crate) fn new(name: String, value: &Value) -> Self {
        let (parameter_type, parameter_value) = bigquery_param(value);
        Self {
            name,
            parameter_type,
            parameter_value,
        }
    }
}

#[derive(Debug, Deserialize)]
struct QueryResponse {
    schema: Option<TableSchema>,
    rows: Option<Vec<TableRow>>,
}

impl QueryResponse {
    fn records(self, object_type: &ApiName) -> Vec<Record> {
        let Some(schema) = self.schema else {
            return Vec::new();
        };
        self.rows
            .unwrap_or_default()
            .into_iter()
            .map(|row| row.record(object_type, &schema.fields))
            .collect()
    }

    fn aggregate_result(self) -> AggregateResult {
        let Some(schema) = self.schema else {
            return AggregateResult { groups: Vec::new() };
        };
        let groups = self
            .rows
            .unwrap_or_default()
            .into_iter()
            .map(|row| row.group(&schema.fields))
            .collect();
        AggregateResult { groups }
    }
}

#[derive(Debug, Deserialize)]
struct TableSchema {
    fields: Vec<TableField>,
}

#[derive(Debug, Deserialize)]
struct TableField {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
}

#[derive(Debug, Deserialize)]
struct TableRow {
    f: Vec<Cell>,
}

impl TableRow {
    fn record(self, _object_type: &ApiName, fields: &[TableField]) -> Record {
        let mut values = BTreeMap::new();
        for (field, cell) in fields.iter().zip(self.f) {
            values.insert(
                ApiName::new_unchecked(&field.name),
                cell_value(cell.v, &field.field_type),
            );
        }
        let primary_key = values
            .get(&ApiName::new_unchecked("id"))
            .cloned()
            .or_else(|| values.values().next().cloned());
        Record {
            primary_key,
            values,
        }
    }

    fn group(self, fields: &[TableField]) -> BTreeMap<String, Value> {
        fields
            .iter()
            .zip(self.f)
            .map(|(field, cell)| (field.name.clone(), cell_value(cell.v, &field.field_type)))
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct Cell {
    v: JsonValue,
}

#[derive(Debug, Deserialize)]
struct MetadataToken {
    access_token: String,
}

fn string_config(config: &BTreeMap<String, Value>, key: &str) -> Result<String, Error> {
    optional_string_config(config, key)
        .ok_or_else(|| Error::bad_request(format!("bigquery config.{key} is required")))
}

fn optional_string_config(config: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn validate_identifier(kind: &str, value: &str) -> Result<(), Error> {
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        return Err(Error::bad_request(format!(
            "invalid bigquery {kind}: {value}"
        )));
    }
    Ok(())
}

fn bigquery_param(value: &Value) -> (JsonValue, JsonValue) {
    match &value.0 {
        JsonValue::Bool(value) => (
            json!({"type": "BOOL"}),
            json!({"value": if *value { "true" } else { "false" }}),
        ),
        JsonValue::Number(value) if value.is_i64() || value.is_u64() => (
            json!({"type": "INT64"}),
            json!({"value": value.to_string()}),
        ),
        JsonValue::Number(value) => (
            json!({"type": "FLOAT64"}),
            json!({"value": value.to_string()}),
        ),
        JsonValue::Array(_) | JsonValue::Object(_) => (
            json!({"type": "JSON"}),
            json!({"value": value.0.to_string()}),
        ),
        JsonValue::Null => (json!({"type": "STRING"}), json!({"value": JsonValue::Null})),
        JsonValue::String(value) => (json!({"type": "STRING"}), json!({"value": value})),
    }
}

fn cell_value(value: JsonValue, field_type: &str) -> Value {
    match field_type {
        "INTEGER" | "INT64" => value
            .as_str()
            .and_then(|value| value.parse::<i64>().ok())
            .map(Value::integer)
            .unwrap_or_else(|| Value::new(value)),
        "FLOAT" | "FLOAT64" | "NUMERIC" | "BIGNUMERIC" => value
            .as_str()
            .and_then(|value| value.parse::<f64>().ok())
            .map(Value::float)
            .unwrap_or_else(|| Value::new(value)),
        "BOOLEAN" | "BOOL" => value
            .as_str()
            .and_then(|value| value.parse::<bool>().ok())
            .map(Value::bool)
            .unwrap_or_else(|| Value::new(value)),
        "JSON" => value
            .as_str()
            .and_then(|value| serde_json::from_str(value).ok())
            .map(Value::new)
            .unwrap_or_else(|| Value::new(value)),
        _ => Value::new(value),
    }
}
