//! Interceptor chain.

use crate::ports::Interceptor;
use crate::query::InterceptorOp;
use tesela_core::{ApiName, Error, Value};
use std::collections::BTreeMap;

/// A chain of interceptors.
pub struct InterceptorChain {
    interceptors: Vec<Box<dyn Interceptor>>,
}

impl InterceptorChain {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    /// Add an interceptor.
    pub fn add(&mut self, interceptor: Box<dyn Interceptor>) {
        self.interceptors.push(interceptor);
    }

    /// Run the chain for an operation.
    pub fn run(
        &self,
        op: InterceptorOp,
        operation: tesela_core::Operation,
        object_type: &ApiName,
        context: &mut BTreeMap<String, Value>,
    ) -> Result<(), Error> {
        for interceptor in &self.interceptors {
            interceptor.intercept(op.clone(), operation, object_type, context)?;
        }
        Ok(())
    }
}

impl Default for InterceptorChain {
    fn default() -> Self {
        Self::new()
    }
}
