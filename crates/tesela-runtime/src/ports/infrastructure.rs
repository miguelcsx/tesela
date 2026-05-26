//! Infrastructure port traits (audit, events, actions, scheduling, etc.).

use crate::query::*;
use tesela_core::{ApiName, Error};
use tesela_ir::{Record, Spec};

/// Sink for audit records.
pub trait AuditSink: Send + Sync {
    /// Write an audit record.
    fn write_audit(&self, record: AuditRecord) -> Result<(), Error>;
}

/// Event bus for publishing domain events.
pub trait EventBus: Send + Sync {
    /// Publish an event.
    fn publish(&self, event: Event) -> Result<(), Error>;
}

/// Message bus for logical event types with adapter-owned delivery semantics.
pub trait MessageBus: Send + Sync {
    /// Publish an event to a logical topic.
    fn publish_message(&self, event_type: &ApiName, event: Event) -> Result<String, Error>;
    /// Dequeue the next event for a logical topic.
    fn dequeue_message(&self, event_type: &ApiName) -> Result<Option<Event>, Error>;
    /// Acknowledge successful processing.
    fn ack_message(&self, event_type: &ApiName, message_id: &str) -> Result<(), Error>;
    /// Reject a message and optionally requeue it.
    fn nack_message(
        &self,
        event_type: &ApiName,
        message_id: &str,
        requeue: bool,
    ) -> Result<(), Error>;
}

/// Handles execution of a single action.
pub trait ActionHandler: Send + Sync {
    /// Execute the action.
    fn execute(&self, req: ActionRequest) -> Result<tesela_ir::ActionResult, Error>;
}

/// Dispatches action requests to the correct handler.
pub trait ActionDispatcher: Send + Sync {
    /// Look up a handler by action name.
    fn get_handler(&self, action_name: &ApiName) -> Option<&dyn ActionHandler>;
    /// Register a handler for an action.
    fn register(
        &mut self,
        action_name: ApiName,
        handler: Box<dyn ActionHandler>,
    ) -> Result<(), Error>;
}

/// Generates unique IDs.
pub trait IdGenerator: Send + Sync {
    /// Generate a new ID with optional prefix.
    fn new_id(&self, prefix: &str) -> String;
}

/// Clock abstraction.
pub trait Clock: Send + Sync {
    /// Current UTC time.
    fn now(&self) -> chrono::DateTime<chrono::Utc>;
}

/// Scheduler for cron-like jobs.
pub trait Scheduler: Send + Sync {
    /// Schedule a job.
    fn schedule(&self, cron: &str, task: WorkItem) -> Result<String, Error>;
    /// Cancel a scheduled job.
    fn cancel(&self, job_id: &str) -> Result<(), Error>;
}

/// Approval provider for high-risk actions.
pub trait ApprovalProvider: Send + Sync {
    /// Request approval.
    fn request_approval(&self, req: ApprovalRequest) -> Result<ApprovalDecision, Error>;
}

/// Work queue for async tasks.
pub trait WorkQueue: Send + Sync {
    /// Enqueue a work item.
    fn enqueue(&self, item: WorkItem) -> Result<String, Error>;
    /// Dequeue the next work item.
    fn dequeue(&self) -> Result<Option<WorkItem>, Error>;
}

/// Object store for signed upload/read URLs and object metadata.
pub trait ObjectStore: Send + Sync {
    /// Generate a signed upload URL.
    fn signed_upload_url(
        &self,
        path: &str,
        ttl_secs: u64,
        metadata: &std::collections::BTreeMap<String, tesela_core::Value>,
    ) -> Result<SignedUpload, Error>;
    /// Generate a signed read URL or adapter-owned locator.
    fn signed_read_url(
        &self,
        path: &str,
        ttl_secs: u64,
        metadata: &std::collections::BTreeMap<String, tesela_core::Value>,
    ) -> Result<ArtifactLocator, Error>;
    /// Read object metadata.
    fn stat(&self, path: &str) -> Result<ObjectMetadata, Error>;
    /// List objects under a prefix.
    fn list(&self, prefix: &str) -> Result<Vec<ObjectMetadata>, Error>;
    /// Delete an object.
    fn delete(&self, path: &str) -> Result<(), Error>;
}

/// Stores run records and enforces idempotency for actions, jobs, and uploads.
pub trait RunStore: Send + Sync {
    /// Create a run unless an idempotent run already exists.
    fn create_or_reuse(&self, run: RunRecord) -> Result<RunRecord, Error>;
    /// Fetch a run by ID.
    fn get_run(&self, run_id: &str) -> Result<Option<RunRecord>, Error>;
    /// Update the status and output for a run.
    fn update_run(&self, run: RunRecord) -> Result<RunRecord, Error>;
}

/// Issues and validates constrained capability tokens.
pub trait CapabilityIssuer: Send + Sync {
    /// Issue a token for a declared grant.
    fn issue_capability(
        &self,
        grant: &tesela_ir::CapabilityGrant,
        actor: &Actor,
        constraints: std::collections::BTreeMap<String, tesela_core::Value>,
    ) -> Result<CapabilityToken, Error>;
    /// Verify a token string.
    fn verify_capability(&self, token: &str) -> Result<CapabilityToken, Error>;
    /// Revoke a token.
    fn revoke_capability(&self, token_id: &str) -> Result<(), Error>;
}

/// Secret resolver (e.g., environment variables, vault).
pub trait SecretResolver: Send + Sync {
    /// Resolve a secret reference to its value.
    fn resolve(&self, secret_ref: &str) -> Result<String, Error>;
}

/// Meta-store for ontology history / entity CRUD.
pub trait MetaStore: Send + Sync {
    /// Store a spec snapshot.
    fn store_spec(&self, spec: &Spec, hash: &str) -> Result<(), Error>;
    /// Retrieve a spec by hash.
    fn get_spec(&self, hash: &str) -> Result<Option<Spec>, Error>;
}

/// Operation carried by a change event.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeOp {
    /// A new record was inserted.
    Insert,
    /// An existing record was updated.
    Update,
    /// A record was deleted.
    Delete,
}

/// A single change event emitted by a CDC source.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangeEvent {
    /// Object type this change belongs to.
    pub object_type: ApiName,
    /// Kind of change.
    pub operation: ChangeOp,
    /// Record state before the change (present for Update/Delete).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub before: Option<Record>,
    /// Record state after the change (present for Insert/Update).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub after: Option<Record>,
    /// UTC timestamp of the change (RFC 3339).
    pub occurred_at: String,
}

/// Agnostic CDC / streaming source port.
pub trait ChangeStreamSource: Send + Sync {
    /// Subscribe to change events for a single object type.
    fn subscribe(
        &self,
        object_type: &ApiName,
    ) -> Result<std::sync::mpsc::Receiver<ChangeEvent>, Error>;
}

/// Extends [`EventBus`] with client-facing subscriptions.
pub trait SubscriptionBus: EventBus {
    /// Subscribe to events for a given object type.
    fn subscribe(
        &self,
        object_type: Option<&ApiName>,
    ) -> Result<std::sync::mpsc::Receiver<Event>, Error>;
}

/// Creates a new [`crate::Runtime`] for a workspace on demand.
pub trait WorkspaceFactory: Send + Sync {
    /// Instantiate a runtime for the given workspace ID and spec.
    fn create(
        &self,
        workspace_id: &str,
        spec: Spec,
    ) -> Result<std::sync::Arc<crate::Runtime>, Error>;
}
