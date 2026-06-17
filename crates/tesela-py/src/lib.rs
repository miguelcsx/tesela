#![deny(warnings)]

//! PyO3 bindings for Tesela.

mod backend;
mod json;
mod runtime;

use pyo3::prelude::*;
use pyo3::types::PyModule;
use runtime::NativeRuntime;

#[pymodule]
#[pyo3(name = "_native")]
fn native_module(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<NativeRuntime>()?;
    Ok(())
}
