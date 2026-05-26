# Domain Model Overview

## Structure

The Tesela domain model is organized in three layers. The configuration layer contains entities that teams define to describe their domain. The runtime layer contains entities created during operation. The governance layer contains entities that record what happened and enforce access control.

## Configuration Layer

These entities are defined by the team and stored in the ontology registry. They are the inputs from which everything else is derived.

**Workspace** is the root isolation boundary. All other entities belong to a workspace.

**Datasource** describes a connection to an external data store. Object types reference datasources to locate their data.

**Object Type** describes a class of operational entities, their properties, and their data source.

**Property** describes a single attribute of an object type.

**Link Type** describes a relationship between two object types and the mapping that joins them.

**Action Type** describes a typed mutation: what it accepts as input, what it produces as output, who is allowed to execute it, and how it is implemented.

**Role** describes a principal category that actors can hold, with an optional inheritance chain.

**Policy Rule** describes what roles can do with which object types under what conditions.

**Custom Tool** describes a user-defined capability available to agents.

**Agent** describes an AI assistant: its model, its system prompt, its tool set, its limits, and who can invoke it.

**Asset** describes a dataset with a schema, quality rules, and a target sink for ingestion.

## Runtime Layer

These entities are created during operation and represent work in progress or completed work.

**Upload** represents a file upload in progress or completed. It tracks the upload URL, the status, the schema discovery result, and the column mapping.

**Action Run** represents a single execution of an action type. It tracks the actor, the subject, the input, the output, the status, and the idempotency key.

**Agent Run** represents a single execution of an agent. It tracks the actor, the tool calls, the token consumption, the cost, and the final response.

**Asset Version** represents a specific committed state of an asset — a set of rows that passed quality validation and were loaded into the sink.

**Job** represents a unit of work queued for the worker: an ingestion job, an async action job, or an asset transformation job.

## Governance Layer

These entities are immutable records of what happened. They are written by the system and cannot be modified by users.

**Audit Record** is a single entry in the audit log. It records the actor, the operation, the resource, the policy decision, and the timestamp. Every query, every action, every policy check, and every ontology change produces an audit record.

**Ontology Version** is a snapshot of the complete ontology at a point in time. It is written when the team publishes a named version.

## Relationships Between Layers

Configuration entities drive runtime behavior: object type definitions drive query generation, action type definitions drive action dispatch, agent definitions drive tool assembly. Runtime entities produce governance records: every action run produces an audit record, every agent run produces tool call audit records. Governance records are derived from but do not modify configuration or runtime state.
