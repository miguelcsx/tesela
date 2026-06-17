use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tesela_compiler::Compiler;
use tesela_core::Error;
use tesela_ir::Spec;

pub(crate) fn py_err(err: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

pub(crate) fn from_json<T: DeserializeOwned>(raw: &str) -> PyResult<T> {
    serde_json::from_str(raw).map_err(py_err)
}

pub(crate) fn to_json<T: Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(py_err)
}

pub(crate) fn compile_spec(raw: &str) -> PyResult<Spec> {
    let spec: Spec = from_json(raw)?;
    let result = Compiler::default_pipeline().compile(&spec);
    if !result.is_valid {
        let messages = result
            .diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(py_err(messages));
    }
    result
        .spec
        .ok_or_else(|| py_err("compiler produced no spec"))
}

pub(crate) fn adapter_error(err: impl std::fmt::Display) -> Error {
    Error::adapter(err.to_string())
}
