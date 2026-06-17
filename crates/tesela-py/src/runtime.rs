use crate::backend::PyBackend;
use crate::json::{compile_spec, from_json, py_err, to_json};
use pyo3::prelude::*;
use std::sync::Arc;
use tesela_core::{ApiName, Value};
use tesela_memory::{DefaultBackendRegistry, MemoryBackend};
use tesela_runtime::audit::VecAuditSink;
use tesela_runtime::ports::Backend;
use tesela_runtime::query::{Actor, AggregateQuery, Mutation, Query, TraversalQuery};
use tesela_runtime::runtime::{Runtime, RuntimeOptions};

fn default_actor() -> Actor {
    Actor {
        user_id: "python-sdk".to_string(),
        roles: vec!["admin".to_string()],
        claims: Default::default(),
    }
}

fn parse_actor(raw: Option<&str>) -> PyResult<Actor> {
    match raw {
        Some(value) if !value.is_empty() => from_json(value),
        _ => Ok(default_actor()),
    }
}

fn runtime_options(registry: Arc<DefaultBackendRegistry>) -> RuntimeOptions {
    RuntimeOptions {
        backend_registry: Some(registry),
        audit_sink: Some(Arc::new(VecAuditSink::new())),
        allow_dev_defaults: true,
        ..Default::default()
    }
}

#[pyclass]
pub(crate) struct NativeRuntime {
    runtime: Arc<Runtime>,
    registry: Arc<DefaultBackendRegistry>,
    callbacks: Vec<Py<PyAny>>,
}

#[pymethods]
impl NativeRuntime {
    #[new]
    fn new(spec_json: &str) -> PyResult<Self> {
        let spec = compile_spec(spec_json)?;
        let registry = DefaultBackendRegistry::new();
        for datasource in &spec.datasources {
            if datasource.adapter_type == "memory" {
                registry
                    .register(datasource.api_name.clone(), MemoryBackend::new())
                    .map_err(py_err)?;
            }
        }
        let runtime = Runtime::new(spec, runtime_options(registry.clone())).map_err(py_err)?;
        Ok(Self {
            runtime,
            registry,
            callbacks: Vec::new(),
        })
    }

    fn spec_json(&self) -> PyResult<String> {
        to_json(&self.runtime.spec().map_err(py_err)?)
    }

    fn apply_spec_json(&mut self, spec_json: &str) -> PyResult<String> {
        let spec = compile_spec(spec_json)?;
        let diff = self.runtime.apply_spec(spec).map_err(py_err)?;
        to_json(&diff)
    }

    fn register_backend(&mut self, adapter_type: &str, handler: Py<PyAny>) -> PyResult<()> {
        let backend_handler = Python::attach(|py| handler.clone_ref(py));
        let backend: Arc<dyn Backend> =
            Arc::new(PyBackend::new(adapter_type.to_string(), backend_handler));
        self.registry
            .register(ApiName::new(adapter_type).map_err(py_err)?, backend.clone())
            .map_err(py_err)?;
        for datasource in &self.runtime.spec().map_err(py_err)?.datasources {
            if datasource.adapter_type == adapter_type {
                self.registry
                    .register(datasource.api_name.clone(), backend.clone())
                    .map_err(py_err)?;
            }
        }
        self.callbacks.push(handler);
        Ok(())
    }

    fn search_json(
        &self,
        object_type: &str,
        query_json: &str,
        actor_json: Option<&str>,
    ) -> PyResult<String> {
        let query: Query = from_json(query_json)?;
        let actor = parse_actor(actor_json)?;
        let object_type = ApiName::new(object_type).map_err(py_err)?;
        to_json(
            &self
                .runtime
                .search(&actor, &object_type, query)
                .map_err(py_err)?,
        )
    }

    fn get_json(
        &self,
        object_type: &str,
        primary_key_json: &str,
        actor_json: Option<&str>,
    ) -> PyResult<String> {
        let primary_key: Value = from_json(primary_key_json)?;
        let actor = parse_actor(actor_json)?;
        let object_type = ApiName::new(object_type).map_err(py_err)?;
        to_json(
            &self
                .runtime
                .get(&actor, &object_type, &primary_key)
                .map_err(py_err)?,
        )
    }

    fn mutate_json(
        &self,
        object_type: &str,
        mutation_json: &str,
        actor_json: Option<&str>,
    ) -> PyResult<String> {
        let mutation: Mutation = from_json(mutation_json)?;
        let actor = parse_actor(actor_json)?;
        let object_type = ApiName::new(object_type).map_err(py_err)?;
        to_json(
            &self
                .runtime
                .mutate(&actor, &object_type, mutation)
                .map_err(py_err)?,
        )
    }

    fn execute_action_json(
        &self,
        action: &str,
        input_json: &str,
        actor_json: Option<&str>,
    ) -> PyResult<String> {
        let input: Value = from_json(input_json)?;
        let actor = parse_actor(actor_json)?;
        let action = ApiName::new(action).map_err(py_err)?;
        to_json(
            &self
                .runtime
                .execute_action(&actor, &action, input)
                .map_err(py_err)?,
        )
    }

    fn explain_json(
        &self,
        object_type: &str,
        query_json: &str,
        actor_json: Option<&str>,
    ) -> PyResult<String> {
        let query: Query = from_json(query_json)?;
        let actor = parse_actor(actor_json)?;
        let object_type = ApiName::new(object_type).map_err(py_err)?;
        to_json(
            &self
                .runtime
                .explain(&actor, &object_type, query)
                .map_err(py_err)?,
        )
    }

    fn traverse_json(
        &self,
        link_type: &str,
        query_json: &str,
        actor_json: Option<&str>,
    ) -> PyResult<String> {
        let query: TraversalQuery = from_json(query_json)?;
        let actor = parse_actor(actor_json)?;
        let link_type = ApiName::new(link_type).map_err(py_err)?;
        to_json(
            &self
                .runtime
                .traverse(&actor, &link_type, query)
                .map_err(py_err)?,
        )
    }

    fn aggregate_json(
        &self,
        object_type: &str,
        query_json: &str,
        actor_json: Option<&str>,
    ) -> PyResult<String> {
        let query: AggregateQuery = from_json(query_json)?;
        let actor = parse_actor(actor_json)?;
        let object_type = ApiName::new(object_type).map_err(py_err)?;
        to_json(
            &self
                .runtime
                .aggregate(&actor, &object_type, query)
                .map_err(py_err)?,
        )
    }

    fn aggregate_view_json(&self, view_name: &str, actor_json: Option<&str>) -> PyResult<String> {
        let actor = parse_actor(actor_json)?;
        let view_name = ApiName::new(view_name).map_err(py_err)?;
        to_json(
            &self
                .runtime
                .aggregate_view(&actor, &view_name)
                .map_err(py_err)?,
        )
    }

    fn health_json(&self) -> PyResult<String> {
        to_json(&self.runtime.health().map_err(py_err)?)
    }

    fn capabilities_json(&self) -> PyResult<String> {
        to_json(&self.runtime.capabilities())
    }

    fn add_entity_json(&mut self, kind: &str, entity_json: &str) -> PyResult<String> {
        let mut spec = self.runtime.spec().map_err(py_err)?;
        let entity = serde_json::from_str(entity_json).map_err(py_err)?;
        match kind {
            "object_type" => spec
                .object_types
                .push(serde_json::from_value(entity).map_err(py_err)?),
            "link_type" => spec
                .link_types
                .push(serde_json::from_value(entity).map_err(py_err)?),
            "action" => spec
                .actions
                .push(serde_json::from_value(entity).map_err(py_err)?),
            "policy" => spec
                .policies
                .push(serde_json::from_value(entity).map_err(py_err)?),
            "agent" => spec
                .agents
                .push(serde_json::from_value(entity).map_err(py_err)?),
            "pipeline" => spec
                .pipelines
                .push(serde_json::from_value(entity).map_err(py_err)?),
            _ => return Err(py_err(format!("unsupported entity kind: {kind}"))),
        }
        let diff = self.runtime.apply_spec(spec).map_err(py_err)?;
        to_json(&diff)
    }
}
