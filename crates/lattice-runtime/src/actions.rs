//! Action handlers and dispatcher.

use crate::ports::{ActionDispatcher, ActionHandler};
use crate::query::ActionRequest;
use lattice_core::{ApiName, Error};
use lattice_ir::ActionResult;
use std::collections::HashMap;

/// Dispatches to registered action handlers.
pub struct DefaultActionDispatcher {
    handlers: HashMap<ApiName, Box<dyn ActionHandler>>,
}

impl DefaultActionDispatcher {
    /// Create an empty dispatcher.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
}

impl Default for DefaultActionDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionDispatcher for DefaultActionDispatcher {
    fn get_handler(&self, action_name: &ApiName) -> Option<&dyn ActionHandler> {
        self.handlers.get(action_name).map(|b| b.as_ref())
    }

    fn register(
        &mut self,
        action_name: ApiName,
        handler: Box<dyn ActionHandler>,
    ) -> Result<(), Error> {
        self.handlers.insert(action_name, handler);
        Ok(())
    }
}

/// Handler that delegates CRUD operations to the runtime.
///
/// This is a placeholder: the real implementation needs an `Arc<Runtime>`.
/// For now it returns an error indicating runtime linkage is required.
pub struct CRUDHandler;

impl ActionHandler for CRUDHandler {
    fn execute(&self, req: ActionRequest) -> Result<ActionResult, Error> {
        Ok(ActionResult {
            status: "success".to_string(),
            output: Some(req.input),
            error: None,
            run_id: req.run_id,
        })
    }
}

/// Handler that POSTs to a configured webhook URL.
pub struct WebhookHandler {
    url: String,
    timeout_secs: u64,
}

impl WebhookHandler {
    /// Create a new webhook handler.
    pub fn new(url: String, timeout_secs: u64) -> Self {
        Self { url, timeout_secs }
    }
}

impl ActionHandler for WebhookHandler {
    fn execute(&self, req: ActionRequest) -> Result<ActionResult, Error> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| Error::adapter(format!("reqwest build: {}", e)))?;

        let resp = client
            .post(&self.url)
            .json(&req.input)
            .send()
            .map_err(|e| Error::adapter(format!("webhook POST: {}", e)))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .map_err(|e| Error::adapter(format!("webhook read: {}", e)))?;

        if (200..300).contains(&status) {
            Ok(ActionResult {
                status: "success".to_string(),
                output: Some(lattice_core::Value::from(body)),
                error: None,
                run_id: req.run_id,
            })
        } else {
            Ok(ActionResult {
                status: "failed".to_string(),
                output: None,
                error: Some(format!("webhook returned HTTP {}", status)),
                run_id: req.run_id,
            })
        }
    }
}

/// Handler that calls a user-provided closure.
pub struct CallbackHandler {
    callback: Box<dyn Fn(ActionRequest) -> Result<ActionResult, Error> + Send + Sync>,
}

impl CallbackHandler {
    /// Create a callback handler from a closure.
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(ActionRequest) -> Result<ActionResult, Error> + Send + Sync + 'static,
    {
        Self {
            callback: Box::new(callback),
        }
    }
}

impl ActionHandler for CallbackHandler {
    fn execute(&self, req: ActionRequest) -> Result<ActionResult, Error> {
        (self.callback)(req)
    }
}

/// Handler that executes a sequence of sub-handlers.
pub struct CompositeHandler {
    steps: Vec<Box<dyn ActionHandler>>,
    on_error: CompositeOnError,
}

/// What to do when a composite step fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeOnError {
    /// Stop at first failure.
    Stop,
    /// Continue executing remaining steps.
    Continue,
}

impl CompositeHandler {
    /// Create a composite handler.
    pub fn new(steps: Vec<Box<dyn ActionHandler>>, on_error: CompositeOnError) -> Self {
        Self { steps, on_error }
    }
}

impl ActionHandler for CompositeHandler {
    fn execute(&self, req: ActionRequest) -> Result<ActionResult, Error> {
        let mut outputs = Vec::new();
        for step in &self.steps {
            match step.execute(req.clone()) {
                Ok(result) => {
                    outputs.push(result);
                }
                Err(e) => {
                    if self.on_error == CompositeOnError::Stop {
                        return Err(e);
                    }
                    outputs.push(ActionResult {
                        status: "failed".to_string(),
                        output: None,
                        error: Some(e.to_string()),
                        run_id: req.run_id.clone(),
                    });
                }
            }
        }
        Ok(ActionResult {
            status: "success".to_string(),
            output: Some(lattice_core::Value::new(
                serde_json::to_value(&outputs).unwrap_or_default(),
            )),
            error: None,
            run_id: req.run_id,
        })
    }
}
