//! Evidence, decision, and event records for generic governance contracts.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tesela_core::{ApiName, Value};

/// A node of evidence referenced by decisions, runs, or events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceNode {
    /// Evidence identifier.
    pub id: String,
    /// Evidence kind.
    pub kind: String,
    /// Optional external or logical reference.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reference: Option<String>,
    /// Optional inline payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub payload: Option<Value>,
    /// Producing resource API name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub produced_by: Option<ApiName>,
    /// Timestamp associated with the evidence.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub occurred_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A durable record of a decision and the evidence used to make it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// Decision identifier.
    pub id: String,
    /// Decision status.
    pub status: String,
    /// Human-readable decision summary.
    pub decision: String,
    /// Actor or system that made the decision.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decided_by: Option<String>,
    /// Rationale for the decision.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rationale: Option<String>,
    /// Referenced evidence identifiers.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub evidence: Vec<String>,
    /// Alternative options considered.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub alternatives: Vec<String>,
    /// Decision timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decided_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}

/// A concrete domain event instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainEvent {
    /// Event identifier.
    pub id: String,
    /// Event type API name.
    pub event_type: ApiName,
    /// Optional subject resource API name.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub subject: Option<ApiName>,
    /// Event payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub payload: Option<Value>,
    /// Actor or system that emitted the event.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub emitted_by: Option<String>,
    /// Correlation identifier.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub correlation_id: Option<String>,
    /// Event timestamp.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub occurred_at: Option<String>,
    /// Metadata.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<BTreeMap<String, Value>>,
}
