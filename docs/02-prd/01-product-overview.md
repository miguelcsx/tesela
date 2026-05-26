# Product Overview

## Product Definition

Tesela is an open-source application framework and runtime for ontology-driven operational systems. It provides the infrastructure layer between a team's data stores and their operational applications, enforcing policy, generating APIs, executing actions, and serving AI agents — all driven by a live, versioned ontology definition.

## Core Value Propositions

**Define once, operate everywhere.** A team defines their domain in a canonical ontology IR. From that definition, Tesela derives runtime operations, a GraphQL schema, language-native SDK authoring surfaces, agent tool definitions, and audit infrastructure. None of these must be maintained separately.

**Any data store.** Tesela's adapter system allows object types to source data from Postgres, BigQuery, MySQL, DuckDB, ClickHouse, Snowflake, or any store with a Tesela adapter. Object types from different adapters appear unified to API consumers.

**Policy as a first-class citizen.** Access control is not a middleware bolt-on. It is evaluated by a dedicated policy engine at every query and action, using a rule model that supports role hierarchies, attribute-based conditions, relationship conditions, time-based restrictions, and field-level redaction.

**Governed AI agents.** Agents receive auto-generated tools derived from the ontology, constrained by the same policy engine that governs human users. Every tool call is audited identically to a human API request.

**Production grade from day one.** Idempotent action execution, append-only audit logs, encrypted credential storage, multi-workspace isolation, OpenTelemetry instrumentation, and bounded query execution are not optional features — they are part of the baseline runtime.

## Delivery Model

Tesela is delivered as:
- Rust crates for the runtime, IR, compiler, adapters, GraphQL, MCP, and server wrappers
- A native ABI library for Python and future SDKs
- A hand-written Python package named `tesela-sdk`
- Optional CLI/server wrappers for teams that want an HTTP boundary
- User-owned adapter packages for data stores, queues, object stores, and action handlers

## Licensing

Apache 2.0. No proprietary components, no telemetry callbacks to external services, no usage-based restrictions.
