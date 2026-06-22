//! Top-level spec and workspace types.

use crate::{
    ActionType, Agent, AggregateView, ArtifactType, Asset, CapabilityGrant, CustomTool,
    Environment, EventType, FunctionDefinition, JobType, LayerDefinition, LinkType, ObjectSet,
    ObjectType, PolicyRule, Role, Trait, TransformPipeline, UploadFlow, WorkflowDefinition,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value, Version};

/// Current spec version string.
pub const SPEC_VERSION: &str = "tesela.spec.v1";

/// The root of a Tesela ontology document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spec {
    /// Spec version identifier.
    pub version: Version,
    /// Workspace definition.
    pub workspace: Workspace,
    /// Datasource connections.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub datasources: Vec<Datasource>,
    /// Reusable trait definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub traits: Vec<Trait>,
    /// Object type definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub object_types: Vec<ObjectType>,
    /// Link type definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub link_types: Vec<LinkType>,
    /// Action type definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub actions: Vec<ActionType>,
    /// Role definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub roles: Vec<Role>,
    /// Policy rule definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub policies: Vec<PolicyRule>,
    /// Agent definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub agents: Vec<Agent>,
    /// Layer definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub layers: Vec<LayerDefinition>,
    /// Function definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub functions: Vec<FunctionDefinition>,
    /// Workflow definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub workflows: Vec<WorkflowDefinition>,
    /// Custom tool definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub custom_tools: Vec<CustomTool>,
    /// Asset definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub assets: Vec<Asset>,
    /// Environment definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub environments: Vec<Environment>,
    /// Named, reusable object set definitions (saved queries / composable filters).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub object_sets: Vec<ObjectSet>,
    /// Transform pipeline definitions (Code Repository equivalent).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub pipelines: Vec<TransformPipeline>,
    /// Byte-oriented artifact definitions governed by policy and adapters.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub artifact_types: Vec<ArtifactType>,
    /// Declarative upload / ingestion flows.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub upload_flows: Vec<UploadFlow>,
    /// Long-running job definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub job_types: Vec<JobType>,
    /// Logical event definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub event_types: Vec<EventType>,
    /// Capability grant definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub capability_grants: Vec<CapabilityGrant>,
    /// Named aggregate views.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aggregate_views: Vec<AggregateView>,
}

impl Spec {
    /// Parse a spec from JSON bytes.
    pub fn parse(data: &[u8]) -> Result<Self, tesela_core::Error> {
        serde_json::from_slice(data)
            .map_err(|e| tesela_core::Error::validation(format!("invalid spec JSON: {}", e)))
    }

    /// Serialize to compact JSON bytes.
    pub fn to_json(&self) -> Result<Vec<u8>, tesela_core::Error> {
        serde_json::to_vec(self)
            .map_err(|e| tesela_core::Error::internal(format!("JSON serialization failed: {}", e)))
    }

    /// Serialize to pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, tesela_core::Error> {
        serde_json::to_string_pretty(self)
            .map_err(|e| tesela_core::Error::internal(format!("JSON serialization failed: {}", e)))
    }

    /// Serialize to compact JSON string.
    pub fn to_json_string(&self) -> Result<String, tesela_core::Error> {
        serde_json::to_string(self)
            .map_err(|e| tesela_core::Error::internal(format!("JSON serialization failed: {}", e)))
    }

    /// Deep clone via serde round-trip.
    pub fn clone_deep(&self) -> Self {
        self.clone()
    }

    /// Compute a SHA-256 hash of the normalized spec JSON.
    pub fn hash(&self) -> String {
        let json = self.to_json_string().unwrap_or_default();
        let digest = sha2::Sha256::digest(json.as_bytes());
        hex::encode(digest)
    }

    /// Add or replace an object type by api_name.
    pub fn upsert_object_type(&mut self, item: ObjectType) {
        upsert_by_name(&mut self.object_types, item);
    }

    /// Add or replace a link type by api_name.
    pub fn upsert_link_type(&mut self, item: LinkType) {
        upsert_by_name(&mut self.link_types, item);
    }

    /// Add or replace an action type by api_name.
    pub fn upsert_action(&mut self, item: ActionType) {
        upsert_by_name(&mut self.actions, item);
    }

    /// Add or replace a policy rule by api_name.
    pub fn upsert_policy(&mut self, item: PolicyRule) {
        upsert_by_name(&mut self.policies, item);
    }

    /// Add or replace an agent by api_name.
    pub fn upsert_agent(&mut self, item: Agent) {
        upsert_by_name(&mut self.agents, item);
    }

    /// Add or replace a layer by api_name.
    pub fn upsert_layer(&mut self, item: LayerDefinition) {
        upsert_by_name(&mut self.layers, item);
    }

    /// Add or replace a function by api_name.
    pub fn upsert_function(&mut self, item: FunctionDefinition) {
        upsert_by_name(&mut self.functions, item);
    }

    /// Add or replace a workflow by api_name.
    pub fn upsert_workflow(&mut self, item: WorkflowDefinition) {
        upsert_by_name(&mut self.workflows, item);
    }

    /// Add or replace a trait by api_name.
    pub fn upsert_trait(&mut self, item: Trait) {
        upsert_by_name(&mut self.traits, item);
    }

    /// Add or replace a pipeline by api_name.
    pub fn upsert_pipeline(&mut self, item: TransformPipeline) {
        upsert_by_name(&mut self.pipelines, item);
    }

    /// Remove an entity by kind and api_name. Returns true if found.
    pub fn remove_entity(&mut self, kind: &str, api_name: &ApiName) -> bool {
        match kind {
            "object_type" => remove_by_name(&mut self.object_types, api_name),
            "link_type" => remove_by_name(&mut self.link_types, api_name),
            "action" => remove_by_name(&mut self.actions, api_name),
            "policy" => remove_by_name(&mut self.policies, api_name),
            "agent" => remove_by_name(&mut self.agents, api_name),
            "layer" => remove_by_name(&mut self.layers, api_name),
            "function" => remove_by_name(&mut self.functions, api_name),
            "workflow" => remove_by_name(&mut self.workflows, api_name),
            "trait" => remove_by_name(&mut self.traits, api_name),
            "pipeline" => remove_by_name(&mut self.pipelines, api_name),
            "role" => remove_by_name(&mut self.roles, api_name),
            "datasource" => remove_by_name(&mut self.datasources, api_name),
            "custom_tool" => remove_by_name(&mut self.custom_tools, api_name),
            "asset" => remove_by_name(&mut self.assets, api_name),
            "artifact_type" => remove_by_name(&mut self.artifact_types, api_name),
            "upload_flow" => remove_by_name(&mut self.upload_flows, api_name),
            "job_type" => remove_by_name(&mut self.job_types, api_name),
            "event_type" => remove_by_name(&mut self.event_types, api_name),
            "capability_grant" => remove_by_name(&mut self.capability_grants, api_name),
            "aggregate_view" => remove_by_name(&mut self.aggregate_views, api_name),
            _ => false,
        }
    }

    /// Whether the spec contains no definitions beyond the workspace.
    pub fn is_empty(&self) -> bool {
        self.datasources.is_empty()
            && self.traits.is_empty()
            && self.object_types.is_empty()
            && self.link_types.is_empty()
            && self.actions.is_empty()
            && self.roles.is_empty()
            && self.policies.is_empty()
            && self.agents.is_empty()
            && self.layers.is_empty()
            && self.functions.is_empty()
            && self.workflows.is_empty()
            && self.custom_tools.is_empty()
            && self.assets.is_empty()
            && self.environments.is_empty()
            && self.object_sets.is_empty()
            && self.pipelines.is_empty()
            && self.artifact_types.is_empty()
            && self.upload_flows.is_empty()
            && self.job_types.is_empty()
            && self.event_types.is_empty()
            && self.capability_grants.is_empty()
            && self.aggregate_views.is_empty()
    }
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            version: Version::new(SPEC_VERSION),
            workspace: Workspace::default(),
            datasources: Vec::new(),
            traits: Vec::new(),
            object_types: Vec::new(),
            link_types: Vec::new(),
            actions: Vec::new(),
            roles: Vec::new(),
            policies: Vec::new(),
            agents: Vec::new(),
            layers: Vec::new(),
            functions: Vec::new(),
            workflows: Vec::new(),
            custom_tools: Vec::new(),
            assets: Vec::new(),
            environments: Vec::new(),
            object_sets: Vec::new(),
            pipelines: Vec::new(),
            artifact_types: Vec::new(),
            upload_flows: Vec::new(),
            job_types: Vec::new(),
            event_types: Vec::new(),
            capability_grants: Vec::new(),
            aggregate_views: Vec::new(),
        }
    }
}

/// A workspace is the top-level isolation boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    /// API name of the workspace.
    pub api_name: ApiName,
    /// Human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub display: Option<String>,
    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            api_name: ApiName::new_unchecked("default"),
            display: None,
            metadata: None,
        }
    }
}

/// Trait for types that have an `api_name` field for identity comparison.
pub trait HasApiName {
    /// Return the api_name of this entity.
    fn api_name(&self) -> &ApiName;
}

macro_rules! impl_has_api_name {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl HasApiName for $ty {
                fn api_name(&self) -> &ApiName { &self.api_name }
            }
        )+
    };
}

impl_has_api_name!(
    ObjectType,
    LinkType,
    ActionType,
    PolicyRule,
    Agent,
    LayerDefinition,
    FunctionDefinition,
    WorkflowDefinition,
    Trait,
    TransformPipeline,
    Role,
    Datasource,
    CustomTool,
    Asset,
    ArtifactType,
    UploadFlow,
    JobType,
    EventType,
    CapabilityGrant,
    AggregateView,
);

fn upsert_by_name<T: HasApiName>(vec: &mut Vec<T>, item: T) {
    let name = item.api_name().clone();
    if let Some(pos) = vec.iter().position(|e| *e.api_name() == name) {
        vec[pos] = item;
    } else {
        vec.push(item);
    }
}

fn remove_by_name<T: HasApiName>(vec: &mut Vec<T>, api_name: &ApiName) -> bool {
    let len = vec.len();
    vec.retain(|e| e.api_name() != api_name);
    vec.len() < len
}

/// A named connection to an external data store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Datasource {
    /// API name of the datasource.
    pub api_name: ApiName,
    /// Adapter type identifier (e.g., "postgres", "bigquery", "memory").
    pub adapter_type: String,
    /// Connection configuration (host, port, database, etc.).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<BTreeMap<String, Value>>,
    /// Secret references (not raw credentials).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub secrets: Option<BTreeMap<String, Value>>,
}
