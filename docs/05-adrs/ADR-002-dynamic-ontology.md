# ADR-002: Dynamic Runtime Ontology Over Compile-Time Code Generation

## Status

Accepted

## Context

Ontology-driven systems can be implemented in two ways. In the compile-time approach, the user defines the schema in a DSL or configuration file, a code generator produces typed application code from the schema, and the application is rebuilt and redeployed for every schema change. In the runtime approach, the schema is stored as data in a database, loaded into memory at runtime, and used to drive dynamic query generation and API exposure. Changes take effect without rebuilding or redeploying.

## Decision

Tesela uses the runtime (dynamic) approach. The ontology is stored in the metadata database, cached in memory, and drives all query, action, policy, and API generation at request time. There is no code generation step for the core runtime.

## Reasoning

**Foundry parity**: The system that most closely matches Tesela's goals (Palantir Foundry) uses a dynamic ontology. Object types can be defined, modified, and deleted through the platform UI or API without any deployment cycle. This is the expected behavior for teams coming from or comparing to Foundry.

**Operational flexibility**: Teams need to iterate on their domain model frequently, especially in early stages. A compile-time approach requires a full build-and-deploy cycle for every property addition, link type creation, or policy change. A runtime approach makes these changes immediate.

**Multi-tenancy**: In a multi-workspace deployment, each workspace has its own ontology. A compile-time approach would require generating separate code artifacts per workspace, which is impractical. A runtime approach serves all workspaces from a single deployment.

**Live schema for GraphQL and OpenAPI**: The REST and GraphQL APIs must reflect the current ontology. A dynamic schema that updates when the ontology changes is simpler and more correct than regenerating and redeploying code.

## Trade-offs Accepted

Dynamic query generation means that type errors in query construction are caught at runtime rather than at compile time. This is mitigated by the ontology validation step, which catches structural inconsistencies at definition time, and by the integration test suite.

A dynamic system has slightly higher per-request overhead than a compiled, type-specialized system. This is negligible because the bottleneck is the external data store, not Tesela's internal processing.

## Consequences

SDKs are hand-written language surfaces that build the canonical Tesela IR and
load the same native runtime. They are not generated clients, they are not
derived from OpenAPI, and they do not require a running HTTP service to execute
Tesela operations.

The core runtime still has no generated code. Language-specific builders remain
thin authoring layers over `tesela.spec.v1`; execution is delegated to the
Rust runtime through native ABI calls or direct Rust calls.
