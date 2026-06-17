use crate::json::adapter_error;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use serde::de::DeserializeOwned;
use serde_json::{Value as JsonValue, json};
use tesela_core::{ApiName, Error, Value};
use tesela_ir::{AggregateResult, ExplainPlan, MutationResult, Page, Record};
use tesela_runtime::ports::{
    Aggregator, Backend, BulkLoader, Getter, Mutator, Rollbacker, SearchExplainer, Searcher,
    Traverser,
};
use tesela_runtime::query::{AggregateQuery, BackendCapabilities, Mutation, Query, TraversalQuery};

pub(crate) struct PyBackend {
    adapter_type: String,
    handler: Py<PyAny>,
}

impl PyBackend {
    pub(crate) fn new(adapter_type: String, handler: Py<PyAny>) -> Self {
        Self {
            adapter_type,
            handler,
        }
    }

    fn call<T: DeserializeOwned>(&self, req: JsonValue) -> Result<T, Error> {
        Python::attach(|py| {
            let json_mod = PyModule::import(py, "json").map_err(adapter_error)?;
            let req_str = serde_json::to_string(&req).map_err(adapter_error)?;
            let req_obj = json_mod
                .getattr("loads")
                .and_then(|loads| loads.call1((req_str,)))
                .map_err(adapter_error)?;
            let resp = self.handler.call1(py, (req_obj,)).map_err(adapter_error)?;
            let resp_json: String = json_mod
                .getattr("dumps")
                .and_then(|dumps| dumps.call1((resp,)))
                .and_then(|value| value.extract())
                .map_err(adapter_error)?;
            serde_json::from_str(&resp_json).map_err(adapter_error)
        })
    }
}

impl Backend for PyBackend {
    fn backend_type(&self) -> &str {
        &self.adapter_type
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.call(json!({"op": "capabilities"}))
            .unwrap_or(BackendCapabilities {
                search: true,
                get: true,
                mutate: true,
                aggregate: true,
                traverse: true,
                bulk_load: true,
                rollback: true,
                explain: true,
            })
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

    fn as_traverser(&self) -> Option<&dyn Traverser> {
        Some(self)
    }

    fn as_bulk_loader(&self) -> Option<&dyn BulkLoader> {
        Some(self)
    }

    fn as_rollbacker(&self) -> Option<&dyn Rollbacker> {
        Some(self)
    }

    fn as_explainer(&self) -> Option<&dyn SearchExplainer> {
        Some(self)
    }
}

impl Searcher for PyBackend {
    fn search(&self, object_type: &ApiName, query: &Query) -> Result<Page, Error> {
        self.call(json!({"op": "search", "object_type": object_type, "query": query}))
    }
}

impl Getter for PyBackend {
    fn get(&self, object_type: &ApiName, pk: &Value) -> Result<Option<Record>, Error> {
        #[derive(serde::Deserialize)]
        struct Resp {
            record: Option<Record>,
        }
        self.call::<Resp>(json!({"op": "get", "object_type": object_type, "primary_key": pk}))
            .map(|resp| resp.record)
    }
}

impl Mutator for PyBackend {
    fn mutate(&self, object_type: &ApiName, mutation: &Mutation) -> Result<MutationResult, Error> {
        self.call(json!({"op": "mutate", "object_type": object_type, "mutation": mutation}))
    }
}

impl Aggregator for PyBackend {
    fn aggregate(
        &self,
        object_type: &ApiName,
        query: &AggregateQuery,
    ) -> Result<AggregateResult, Error> {
        self.call(json!({"op": "aggregate", "object_type": object_type, "query": query}))
    }
}

impl Traverser for PyBackend {
    fn traverse(&self, link_type: &ApiName, query: &TraversalQuery) -> Result<Page, Error> {
        self.call(json!({"op": "traverse", "link_type": link_type, "query": query}))
    }
}

impl BulkLoader for PyBackend {
    fn bulk_load(
        &self,
        object_type: &ApiName,
        records: Vec<Record>,
        load_id: &str,
    ) -> Result<i64, Error> {
        #[derive(serde::Deserialize)]
        struct Resp {
            count: i64,
        }
        self.call::<Resp>(
            json!({"op": "bulk_load", "object_type": object_type, "records": records, "load_id": load_id}),
        )
        .map(|resp| resp.count)
    }
}

impl Rollbacker for PyBackend {
    fn rollback(&self, object_type: &ApiName, load_id: &str) -> Result<(), Error> {
        let _: JsonValue =
            self.call(json!({"op": "rollback", "object_type": object_type, "load_id": load_id}))?;
        Ok(())
    }
}

impl SearchExplainer for PyBackend {
    fn explain_search(&self, object_type: &ApiName, query: &Query) -> Result<ExplainPlan, Error> {
        self.call(json!({"op": "explain", "object_type": object_type, "query": query}))
    }
}
