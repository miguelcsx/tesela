//! Top-level ontology spec and datasource types.

use crate::{ActionType, LinkType, ObjectSet, ObjectType, PolicyRule, Role, Trait};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::BTreeMap;
use tesela_core::{ApiName, Error, Value, Version};

/// Current spec version string.
pub const SPEC_VERSION: &str = "tesela.spec.v1";

/// The root ontology document.
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
    /// Named object set definitions.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub object_sets: Vec<ObjectSet>,
}

impl Spec {
    /// Parse a spec from JSON bytes.
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(data)
            .map_err(|error| Error::validation(format!("invalid spec JSON: {error}")))
    }

    /// Serialize to compact JSON bytes.
    pub fn to_json(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(self)
            .map_err(|error| Error::internal(format!("JSON serialization failed: {error}")))
    }

    /// Serialize to compact JSON string.
    pub fn to_json_string(&self) -> Result<String, Error> {
        serde_json::to_string(self)
            .map_err(|error| Error::internal(format!("JSON serialization failed: {error}")))
    }

    /// Serialize to pretty JSON.
    pub fn to_json_pretty(&self) -> Result<String, Error> {
        serde_json::to_string_pretty(self)
            .map_err(|error| Error::internal(format!("JSON serialization failed: {error}")))
    }

    /// Compute a SHA-256 hash of the serialized spec.
    pub fn hash(&self) -> Result<String, Error> {
        let json = self.to_json_string()?;
        let digest = sha2::Sha256::digest(json.as_bytes());
        Ok(hex::encode(digest))
    }

    /// Add or replace an object type by API name.
    pub fn upsert_object_type(&mut self, item: ObjectType) {
        upsert_by_name(&mut self.object_types, item);
    }

    /// Add or replace a link type by API name.
    pub fn upsert_link_type(&mut self, item: LinkType) {
        upsert_by_name(&mut self.link_types, item);
    }

    /// Add or replace an action by API name.
    pub fn upsert_action(&mut self, item: ActionType) {
        upsert_by_name(&mut self.actions, item);
    }

    /// Add or replace a policy by API name.
    pub fn upsert_policy(&mut self, item: PolicyRule) {
        upsert_by_name(&mut self.policies, item);
    }

    /// Add or replace a trait by API name.
    pub fn upsert_trait(&mut self, item: Trait) {
        upsert_by_name(&mut self.traits, item);
    }

    /// Remove an entity by kind and API name.
    pub fn remove_entity(&mut self, kind: &str, api_name: &ApiName) -> bool {
        match kind {
            "object_type" => remove_by_name(&mut self.object_types, api_name),
            "link_type" => remove_by_name(&mut self.link_types, api_name),
            "action" => remove_by_name(&mut self.actions, api_name),
            "policy" => remove_by_name(&mut self.policies, api_name),
            "trait" => remove_by_name(&mut self.traits, api_name),
            "role" => remove_by_name(&mut self.roles, api_name),
            "datasource" => remove_by_name(&mut self.datasources, api_name),
            "object_set" => remove_by_name(&mut self.object_sets, api_name),
            _ => false,
        }
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
            object_sets: Vec::new(),
        }
    }
}

/// Workspace isolation boundary.
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
    /// Platform-defined store type identifier.
    pub adapter_type: String,
    /// Connection configuration.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub config: Option<BTreeMap<String, Value>>,
    /// Secret references.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub secrets: Option<BTreeMap<String, Value>>,
}

trait HasApiName {
    fn api_name(&self) -> &ApiName;
}

macro_rules! impl_has_api_name {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl HasApiName for $ty {
                fn api_name(&self) -> &ApiName {
                    &self.api_name
                }
            }
        )+
    };
}

impl_has_api_name!(
    ActionType, Datasource, LinkType, ObjectSet, ObjectType, PolicyRule, Role, Trait,
);

fn upsert_by_name<T: HasApiName>(items: &mut Vec<T>, item: T) {
    let name = item.api_name().clone();
    if let Some(index) = items
        .iter()
        .position(|existing| *existing.api_name() == name)
    {
        items[index] = item;
    } else {
        items.push(item);
    }
}

fn remove_by_name<T: HasApiName>(items: &mut Vec<T>, api_name: &ApiName) -> bool {
    let initial_len = items.len();
    items.retain(|item| item.api_name() != api_name);
    items.len() < initial_len
}
