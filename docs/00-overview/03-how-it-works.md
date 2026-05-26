# How Tesela Works

## The Flow from Ontology to API

The core loop in Tesela is: define, deploy, operate.

**Define.** A team writes a declarative ontology spec describing their domain. It declares datasources, object types with their properties and source configurations, link types between objects, action types with their handlers, roles with their inheritance hierarchy, and policy rules. This spec is the authoritative description of the domain.

**Apply.** The team applies the ontology spec to an embedded runtime or to a
server wrapping that runtime. The runtime validates the definitions for
consistency, swaps the live ontology state, and emits audit.

**Operate.** The runtime now serves queries and actions for all declared object
types and action types. REST and GraphQL are optional wrappers. SDKs call the
native runtime directly. Agents can be invoked. All operations pass through the
policy engine and are written to the audit log.

## The Request Path for a Query

When a client requests an object, Tesela follows a deterministic sequence. It extracts the actor identity from the request token and resolves the actor's roles. It loads the target object type from the ontology cache. It evaluates all applicable policy rules to determine whether the operation is allowed, what row-level filter to inject, and which properties to redact. It constructs a query for the adapter backing that object type and executes it. It applies property redaction to the result and returns the response. It writes an audit record before responding.

## The Request Path for an Action

When a client submits an action, Tesela validates the input against the action type's declared JSON Schema. It checks the actor's policy to confirm the action is permitted. It checks the idempotency key against the action runs table to prevent duplicate execution. If the action is new, it creates a run record, dispatches to the handler (a declarative mutation or an HTTP webhook), records the result, emits the audit record, and returns the outcome. Async actions return a run identifier immediately and complete in the background.

## The Ontology as a Graph

Object types and link types form a directed graph. The schema graph is stored in memory and enables multi-hop traversal queries, impact analysis (what changes when an object type is modified), lineage tracing (where does a property's data originate), and automatic agent tool generation. The instance graph — the actual objects and their connections — lives in the underlying data stores and is accessed through SQL joins generated at query time from the link type definitions.

## How Agents Operate

When an agent run starts, the runtime assembles a tool list by combining the auto-generated tools for all declared object types, link types, and action types with any custom tools defined in the agent's configuration. It filters this list by what the actor's policy permits. It builds a system prompt from the agent definition and an automatically generated ontology summary. It calls the language model with the tool list and the user's input. For each tool call the model proposes, the runtime validates the input, checks policy, executes the underlying query or action, and returns the result. The model never accesses data directly.

## How Uploads Work

For large data files, Tesela issues a signed URL pointing to object storage. The client uploads directly to that URL without passing data through the Tesela server. When the upload completes, a worker picks up the file, detects its format and schema, maps the detected columns to the asset's declared properties using the column mapping configuration, runs pre-load quality checks on a sample, triggers a bulk load job from object storage to the target datasource, runs post-load validation as queries in the target system, and commits the asset version if all checks pass.
