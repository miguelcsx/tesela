# Functional Requirements

## Ontology Management

- The system must provide CRUD operations for workspaces, datasources, object types, properties, link types, action types, roles, policy rules, custom tools, and agents via a versioned REST API.
- The system must validate ontology definitions for internal consistency: all referenced datasources must exist, all referenced object types in link types must exist, all referenced roles in policy rules must exist, and role inheritance must not form cycles.
- The system must store a complete history of ontology changes and allow the current state to be exported as a portable declarative document.
- The system must apply ontology changes without requiring a server restart.
- The system must support publishing named ontology snapshots for versioning and rollback reference.

## Query Execution

- The system must support retrieving a single object by its primary key.
- The system must support searching objects with compound filter expressions covering equality, comparison, negation, null checks, and logical combinations.
- The system must support following link types from an object to retrieve related objects, with optional filters on the target type.
- The system must support aggregate queries with group-by and metric functions (count, sum, average, min, max).
- All queries must apply the actor's policy row filters before executing against the adapter.
- All queries must redact properties listed in the actor's property deny rules from the response.
- All queries must enforce workspace-level quotas on maximum rows returned and maximum bytes scanned.

## Action Execution

- The system must validate action input against the action type's declared JSON Schema before executing.
- The system must evaluate the actor's policy for the target action before executing.
- The system must check the idempotency key against the action runs table and return the previous result if a matching run exists.
- The system must support synchronous and asynchronous execution modes.
- The system must support declarative handler types: update, create, delete, and composite.
- The system must support webhook handler types, including configurable timeout and retry policies.
- The system must write an action run record and an audit record for every execution attempt.

## Upload and Ingestion

- The system must generate a time-limited signed URL for direct client upload to object storage.
- The system must detect the format and schema of an uploaded file without requiring the schema to be declared in advance.
- The system must present detected columns alongside the asset's declared properties and support user-defined column mappings.
- The system must validate a sample of uploaded rows against the asset's quality rules before initiating the bulk load.
- The system must trigger a bulk load from object storage to the target datasource without routing data through the Tesela server.
- The system must run post-load validation queries in the target datasource after bulk load.
- The system must report validation errors with row-level detail up to a configurable limit, and write the full error set to object storage.
- The system must support rollback of a failed load using the upload identifier as a filter key.

## Agent Execution

- The system must auto-generate tool definitions for every object type, link type, and action type in the ontology.
- The system must filter the agent's tool list to only include tools the actor's policy permits.
- The system must validate every proposed tool call against the tool's input schema before executing.
- The system must enforce the actor's policy for every tool call that results in a query or action.
- The system must record every tool call, its inputs, outputs, latency, and policy decision in the run trace.
- The system must enforce the agent's declared limits: maximum tool calls, maximum token spend, maximum monetary cost, and timeout.

## API and SDK Runtime

- The system must serve a dynamically generated GraphQL schema reflecting the current ontology.
- The system must provide hand-written SDKs that build canonical IR in language-native code.
- The system must let SDKs execute against the same runtime through a native ABI without HTTP.
- The system must rebuild runtime wrapper schemas, including GraphQL, when the ontology changes without requiring a restart.
