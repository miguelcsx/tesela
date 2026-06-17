//! Workflow contracts for generic orchestration declarations and runs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};

/// A reusable workflow definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// API name.
    pub api_name: ApiName,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// Workflow steps.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub steps: Vec<WorkflowStep>,
    /// Workflow triggers.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub triggers: Vec<WorkflowTrigger>,
    /// Input JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input_schema: Option<Value>,
    /// Output JSON schema.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output_schema: Option<Value>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A step in a workflow definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// API name unique within the workflow.
    pub api_name: ApiName,
    /// Step kind.
    pub kind: WorkflowStepKind,
    /// Target definition API name.
    pub target: ApiName,
    /// Step dependencies by API name.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub depends_on: Vec<ApiName>,
    /// Optional condition expression.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub when: Option<String>,
    /// Static input binding.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input: Option<Value>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// Workflow step target kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStepKind {
    /// Invoke an action.
    Action,
    /// Invoke an agent.
    Agent,
    /// Invoke a function.
    Function,
    /// Invoke a layer.
    Layer,
    /// Invoke another workflow.
    Workflow,
}

/// A workflow trigger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    /// Trigger kind.
    pub kind: String,
    /// Event type API name for event-driven triggers.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub event_type: Option<ApiName>,
    /// Schedule expression for scheduled triggers.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub schedule: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A concrete workflow execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRun {
    /// Run identifier.
    pub id: String,
    /// Workflow definition API name.
    pub workflow: ApiName,
    /// Execution status.
    pub status: String,
    /// Input payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input: Option<Value>,
    /// Output payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<Value>,
    /// Per-step run state.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub steps: Vec<WorkflowStepRun>,
    /// Error message.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    /// Start timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub started_at: Option<String>,
    /// Completion timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub completed_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// Runtime state for a workflow step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStepRun {
    /// Step API name.
    pub step: ApiName,
    /// Step run status.
    pub status: String,
    /// Child run identifier, when the step invokes another executable resource.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub run_id: Option<String>,
    /// Step output payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<Value>,
    /// Error message.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}
