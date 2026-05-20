//! Top-level spec and workspace types.

use crate::{
    ActionType, Agent, AggregateView, ArtifactType, Asset, CapabilityGrant, CustomTool,
    Environment, EventType, JobType, LinkType, ObjectSet, ObjectType, PolicyRule, Role, Trait,
    TransformPipeline, UploadFlow,
};
use lattice_core::{ApiName, Value, Version};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::BTreeMap;

/// Current spec version string.
pub const SPEC_VERSION: &str = "lattice.spec.v1";

/// The root of a Lattice ontology document.
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
    pub fn parse(data: &[u8]) -> Result<Self, lattice_core::Error> {
        serde_json::from_slice(data)
            .map_err(|e| lattice_core::Error::validation(format!("invalid spec JSON: {}", e)))
    }

    /// Serialize to compact JSON bytes.
    pub fn to_json(&self) -> Result<Vec<u8>, lattice_core::Error> {
        serde_json::to_vec(self)
            .map_err(|e| lattice_core::Error::internal(format!("JSON serialization failed: {}", e)))
    }

    /// Serialize to pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, lattice_core::Error> {
        serde_json::to_string_pretty(self)
            .map_err(|e| lattice_core::Error::internal(format!("JSON serialization failed: {}", e)))
    }

    /// Serialize to compact JSON string.
    pub fn to_json_string(&self) -> Result<String, lattice_core::Error> {
        serde_json::to_string(self)
            .map_err(|e| lattice_core::Error::internal(format!("JSON serialization failed: {}", e)))
    }

    /// Deep clone via serde round-trip.
    pub fn clone_deep(&self) -> Self {
        let bytes = self.to_json().expect("infallible");
        Self::parse(&bytes).expect("infallible")
    }

    /// Compute a SHA-256 hash of the normalized spec JSON.
    pub fn hash(&self) -> String {
        let json = self.to_json_string().unwrap_or_default();
        let digest = sha2::Sha256::digest(json.as_bytes());
        hex::encode(digest)
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
