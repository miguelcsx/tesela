use std::collections::BTreeMap;
use std::sync::Arc;

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_json::{Value as Json, json};
use tesela::core::{ApiName, Error, Value};
use tesela::ir::{ActionResult, AggregateResult, MutationResult, Page, Record, Spec};
use tesela::runtime::{AllowAllPolicy, Runtime, RuntimeOptions, ontology_tool_definitions};
use tesela::store::{
    ActionRequest, Actor, AggregateQuery, AuditEvent, AuditSink, EventBus, MemoryStore, Mutation,
    OntologyEvent, OntologyStore, PolicyDecision, PolicyEngine, PolicyRequest, Query,
    StaticStoreRouter, StoreCapabilities, TraversalQuery,
};

fn decode<T: serde::de::DeserializeOwned>(input: &str) -> PyResult<T> {
    serde_json::from_str(input).map_err(|error| PyValueError::new_err(error.to_string()))
}

fn encode<T: serde::Serialize>(value: T) -> PyResult<String> {
    serde_json::to_string(&value).map_err(|error| PyRuntimeError::new_err(error.to_string()))
}

fn run<T, F>(py: Python<'_>, work: F) -> PyResult<String>
where
    T: serde::Serialize + Send,
    F: FnOnce() -> Result<T, Error> + Send,
{
    encode(py.detach(work).map_err(runtime_error)?)
}

fn api_name(input: &str) -> PyResult<ApiName> {
    ApiName::new(input).map_err(|error| PyValueError::new_err(error.to_string()))
}

fn runtime_error(error: Error) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

fn callback<T: serde::de::DeserializeOwned>(
    object: &Py<PyAny>,
    method: &str,
    payload: Json,
) -> Result<T, Error> {
    let input =
        serde_json::to_string(&payload).map_err(|error| Error::adapter(error.to_string()))?;
    let output: String =
        Python::attach(|py| object.call_method1(py, method, (input,))?.extract(py))
            .map_err(|error| Error::adapter(format!("Python {method}: {error}")))?;
    serde_json::from_str(&output)
        .map_err(|error| Error::adapter(format!("Python {method} returned invalid JSON: {error}")))
}

struct PythonStore {
    object: Py<PyAny>,
    capabilities: StoreCapabilities,
}

impl OntologyStore for PythonStore {
    fn store_type(&self) -> &str {
        "python"
    }
    fn capabilities(&self) -> StoreCapabilities {
        self.capabilities.clone()
    }
    fn search(&self, object_type: &ApiName, query: &Query) -> Result<Page, Error> {
        callback(
            &self.object,
            "search",
            json!({"object_type": object_type, "query": query}),
        )
    }
    fn get(&self, object_type: &ApiName, primary_key: &Value) -> Result<Option<Record>, Error> {
        callback(
            &self.object,
            "get",
            json!({"object_type": object_type, "primary_key": primary_key}),
        )
    }
    fn create(
        &self,
        object_type: &ApiName,
        values: BTreeMap<ApiName, Value>,
    ) -> Result<MutationResult, Error> {
        callback(
            &self.object,
            "create",
            json!({"object_type": object_type, "values": values}),
        )
    }
    fn update(
        &self,
        object_type: &ApiName,
        primary_key: &Value,
        values: BTreeMap<ApiName, Value>,
    ) -> Result<MutationResult, Error> {
        callback(
            &self.object,
            "update",
            json!({"object_type": object_type, "primary_key": primary_key, "values": values}),
        )
    }
    fn delete(&self, object_type: &ApiName, primary_key: &Value) -> Result<MutationResult, Error> {
        callback(
            &self.object,
            "delete",
            json!({"object_type": object_type, "primary_key": primary_key}),
        )
    }
    fn execute_action(&self, request: ActionRequest) -> Result<ActionResult, Error> {
        if !self.capabilities.execute_action {
            return Err(Error::unsupported("execute_action"));
        }
        callback(&self.object, "execute_action", json!({"request": request}))
    }
    fn aggregate(
        &self,
        object_type: &ApiName,
        query: &AggregateQuery,
    ) -> Result<AggregateResult, Error> {
        if !self.capabilities.aggregate {
            return Err(Error::unsupported("aggregate"));
        }
        callback(
            &self.object,
            "aggregate",
            json!({"object_type": object_type, "query": query}),
        )
    }
    fn traverse(&self, link_type: &ApiName, query: &TraversalQuery) -> Result<Page, Error> {
        if !self.capabilities.traverse {
            return Err(Error::unsupported("traverse"));
        }
        callback(
            &self.object,
            "traverse",
            json!({"link_type": link_type, "query": query}),
        )
    }
}

struct PythonPolicy(Py<PyAny>);
impl PolicyEngine for PythonPolicy {
    fn evaluate(&self, request: &PolicyRequest) -> Result<PolicyDecision, Error> {
        callback(&self.0, "evaluate", json!({"request": request}))
    }
}

struct PythonAudit(Py<PyAny>);
impl AuditSink for PythonAudit {
    fn record(&self, event: AuditEvent) -> Result<(), Error> {
        callback::<Json>(
            &self.0,
            "record",
            json!({"event": {"actor_id": event.actor_id, "operation": event.operation, "resource_kind": event.resource_kind, "resource": event.resource, "success": event.success, "result_count": event.result_count}}),
        )?;
        Ok(())
    }
}

struct PythonEvent(Py<PyAny>);
impl EventBus for PythonEvent {
    fn publish(&self, event: OntologyEvent) -> Result<(), Error> {
        callback::<Json>(
            &self.0,
            "publish",
            json!({"event": {"kind": event.kind, "resource_kind": event.resource_kind, "resource": event.resource, "actor_id": event.actor_id}}),
        )?;
        Ok(())
    }
}

#[pyclass]
struct NativeMemoryStore {
    inner: Arc<MemoryStore>,
}

#[pymethods]
impl NativeMemoryStore {
    #[new]
    fn new() -> Self {
        Self {
            inner: MemoryStore::new(),
        }
    }
}

#[pyclass]
struct NativeRuntime {
    inner: Arc<Runtime>,
    memory_stores: Vec<Arc<MemoryStore>>,
}

#[pymethods]
impl NativeRuntime {
    #[new]
    #[pyo3(signature = (spec_json, stores, policy, audit=None, events=None, max_query_limit=None))]
    fn new(
        spec_json: &str,
        stores: &Bound<'_, PyDict>,
        policy: &Bound<'_, PyAny>,
        audit: Option<&Bound<'_, PyAny>>,
        events: Option<&Bound<'_, PyAny>>,
        max_query_limit: Option<i32>,
    ) -> PyResult<Self> {
        let spec: Spec = decode(spec_json)?;
        let router = Arc::new(StaticStoreRouter::new());
        let mut memory_stores = Vec::new();
        for (name, store) in stores.iter() {
            let datasource: String = name.extract()?;
            let adapted: Arc<dyn OntologyStore> =
                if let Ok(memory) = store.extract::<PyRef<'_, NativeMemoryStore>>() {
                    memory.inner.set_spec(spec.clone()).map_err(runtime_error)?;
                    memory_stores.push(memory.inner.clone());
                    memory.inner.clone()
                } else {
                    let object = store.unbind();
                    let capabilities =
                        callback(&object, "capabilities", json!({})).map_err(runtime_error)?;
                    Arc::new(PythonStore {
                        object,
                        capabilities,
                    })
                };
            router
                .register(api_name(&datasource)?, adapted)
                .map_err(runtime_error)?;
        }
        let policy_engine: Arc<dyn PolicyEngine> =
            if policy.extract::<String>().ok().as_deref() == Some("allow_all") {
                Arc::new(AllowAllPolicy)
            } else {
                Arc::new(PythonPolicy(policy.clone().unbind()))
            };
        let options = RuntimeOptions {
            store_router: Some(router),
            policy_engine: Some(policy_engine),
            audit_sink: audit
                .filter(|item| !item.is_none())
                .map(|item| Arc::new(PythonAudit(item.clone().unbind())) as Arc<dyn AuditSink>),
            event_bus: events
                .filter(|item| !item.is_none())
                .map(|item| Arc::new(PythonEvent(item.clone().unbind())) as Arc<dyn EventBus>),
            max_query_limit,
        };
        Ok(Self {
            inner: Runtime::new(spec, options).map_err(runtime_error)?,
            memory_stores,
        })
    }

    fn spec(&self) -> PyResult<String> {
        encode(self.inner.spec().map_err(runtime_error)?)
    }
    fn apply_spec(&self, spec_json: &str) -> PyResult<()> {
        let spec: Spec = decode(spec_json)?;
        for store in &self.memory_stores {
            store.set_spec(spec.clone()).map_err(runtime_error)?;
        }
        self.inner.apply_spec(spec).map_err(runtime_error)
    }
    fn search(
        &self,
        py: Python<'_>,
        actor_json: &str,
        object_type: &str,
        query_json: &str,
    ) -> PyResult<String> {
        let actor = decode::<Actor>(actor_json)?;
        let object = api_name(object_type)?;
        let query = decode::<Query>(query_json)?;
        run(py, || self.inner.search(&actor, &object, query))
    }
    fn get(
        &self,
        py: Python<'_>,
        actor_json: &str,
        object_type: &str,
        pk_json: &str,
    ) -> PyResult<String> {
        let actor = decode::<Actor>(actor_json)?;
        let object = api_name(object_type)?;
        let pk = Value::new(decode::<Json>(pk_json)?);
        run(py, || self.inner.get(&actor, &object, &pk))
    }
    fn mutate(
        &self,
        py: Python<'_>,
        actor_json: &str,
        object_type: &str,
        mutation_json: &str,
    ) -> PyResult<String> {
        let actor = decode::<Actor>(actor_json)?;
        let object = api_name(object_type)?;
        let mutation = decode::<Mutation>(mutation_json)?;
        run(py, || self.inner.mutate(&actor, &object, mutation))
    }
    fn aggregate(
        &self,
        py: Python<'_>,
        actor_json: &str,
        object_type: &str,
        query_json: &str,
    ) -> PyResult<String> {
        let actor = decode::<Actor>(actor_json)?;
        let object = api_name(object_type)?;
        let query = decode::<AggregateQuery>(query_json)?;
        run(py, || self.inner.aggregate(&actor, &object, query))
    }
    fn traverse(
        &self,
        py: Python<'_>,
        actor_json: &str,
        link_type: &str,
        query_json: &str,
    ) -> PyResult<String> {
        let actor = decode::<Actor>(actor_json)?;
        let link = api_name(link_type)?;
        let query = decode::<TraversalQuery>(query_json)?;
        run(py, || self.inner.traverse(&actor, &link, query))
    }
    fn resolve_object_set(&self, py: Python<'_>, actor_json: &str, name: &str) -> PyResult<String> {
        let actor = decode::<Actor>(actor_json)?;
        let name = api_name(name)?;
        run(py, || self.inner.resolve_object_set(&actor, &name))
    }
    fn compose_object_sets(
        &self,
        py: Python<'_>,
        actor_json: &str,
        names_json: &str,
        op_json: &str,
    ) -> PyResult<String> {
        let names: Vec<String> = decode(names_json)?;
        let names: Vec<ApiName> = names
            .iter()
            .map(|item| api_name(item))
            .collect::<PyResult<_>>()?;
        let actor = decode::<Actor>(actor_json)?;
        let op = decode::<tesela::ir::SetOp>(op_json)?;
        run(py, || self.inner.compose_object_sets(&actor, &names, op))
    }
    fn execute_action(&self, py: Python<'_>, request_json: &str) -> PyResult<String> {
        let request = decode::<ActionRequest>(request_json)?;
        run(py, || self.inner.execute_action(request))
    }
}

#[pyfunction]
fn normalize_spec(spec_json: &str) -> PyResult<String> {
    let spec = decode::<Spec>(spec_json)?;
    if spec.version.as_ref() != tesela::ir::SPEC_VERSION {
        return Err(PyValueError::new_err(format!(
            "unsupported spec version '{}'",
            spec.version
        )));
    }
    encode(spec)
}

#[pyfunction]
fn tool_definitions() -> PyResult<String> {
    encode(ontology_tool_definitions().map_err(runtime_error)?)
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<NativeMemoryStore>()?;
    module.add_class::<NativeRuntime>()?;
    module.add_function(wrap_pyfunction!(normalize_spec, module)?)?;
    module.add_function(wrap_pyfunction!(tool_definitions, module)?)?;
    Ok(())
}
