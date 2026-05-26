//! Spec hashing, normalization, and the `HasApiName` trait.

use sha2::{Digest, Sha256};
use tesela_core::ApiName;
use tesela_ir::{
    ActionType, Agent, AggregateView, ArtifactType, Asset, CapabilityGrant, CustomTool, Datasource,
    Environment, EventType, JobType, LinkType, ObjectType, PolicyRule, Role, Spec, Trait,
    UploadFlow,
};

/// Compute a SHA-256 hash of the normalized JSON representation of a spec.
pub fn hash_spec(spec: &Spec) -> String {
    let normalized = normalize_spec(spec.clone());
    let json = serde_json::to_string(&normalized).unwrap_or_default();
    let digest = Sha256::digest(json.as_bytes());
    hex::encode(digest)
}

/// Trait for items that have an `api_name` field.
pub trait HasApiName {
    /// Return the API name.
    fn api_name(&self) -> &ApiName;
}

macro_rules! impl_has_api_name {
    ($($t:ty),*) => {
        $(
            impl HasApiName for $t {
                fn api_name(&self) -> &ApiName {
                    &self.api_name
                }
            }
        )*
    };
}

impl_has_api_name!(
    Datasource,
    Trait,
    ObjectType,
    LinkType,
    ActionType,
    Role,
    PolicyRule,
    Agent,
    CustomTool,
    Asset,
    Environment,
    ArtifactType,
    UploadFlow,
    JobType,
    EventType,
    CapabilityGrant,
    AggregateView
);

fn sort_by_name<T: HasApiName>(items: &mut [T]) {
    items.sort_by(|a, b| a.api_name().cmp(b.api_name()));
}

pub(crate) fn normalize_spec(mut spec: Spec) -> Spec {
    sort_by_name(&mut spec.datasources);
    sort_by_name(&mut spec.traits);
    sort_by_name(&mut spec.object_types);
    sort_by_name(&mut spec.link_types);
    sort_by_name(&mut spec.actions);
    sort_by_name(&mut spec.roles);
    sort_by_name(&mut spec.policies);
    sort_by_name(&mut spec.agents);
    sort_by_name(&mut spec.custom_tools);
    sort_by_name(&mut spec.assets);
    sort_by_name(&mut spec.environments);
    sort_by_name(&mut spec.artifact_types);
    sort_by_name(&mut spec.upload_flows);
    sort_by_name(&mut spec.job_types);
    sort_by_name(&mut spec.event_types);
    sort_by_name(&mut spec.capability_grants);
    sort_by_name(&mut spec.aggregate_views);

    for ot in &mut spec.object_types {
        ot.properties.sort_by(|a, b| a.api_name.cmp(&b.api_name));
        ot.indexes.sort_by(|a, b| a.api_name.cmp(&b.api_name));
    }

    for tr in &mut spec.traits {
        tr.properties.sort_by(|a, b| a.api_name.cmp(&b.api_name));
    }

    spec
}
