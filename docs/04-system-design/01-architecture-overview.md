# Architecture Overview

## System Boundary

Tesela is a Rust runtime library plus optional server and CLI wrappers. The
runtime owns ontology application, policy evaluation, adapter dispatch, action
execution, agent tools, upload orchestration, and audit emission. Teams can
embed it directly or wrap it in their own service.

External components that Tesela integrates with but does not own: identity providers (for token validation), object storage (for uploads and assets), datasources (for operational data), secret providers (for credential resolution), and observability backends (for telemetry export).

## Runtime Surfaces

The **native runtime** is the primary SDK execution surface. Language SDKs build
the canonical IR, load the Rust runtime through the native ABI, register backends
and handlers as callbacks, and execute operations without HTTP or generated
client code.

**tesela serve** wraps the same runtime in an HTTP server when a team wants a
network service. It serves REST-style runtime operations for a spec file and is
stateless beyond the configured backend and metadata dependencies.

**Workers** are user-wired runtime loops for asynchronous jobs: async action
execution, upload ingestion pipelines, asset transformations, and indexing
tasks. Queue and storage choices are adapters, not built-in cloud connectors.

**tesela** (CLI) is a command-line tool for operators and developers. It
validates, diffs, inspects, and serves canonical IR specs.

## Internal Package Organization

The workspace is split into Rust crates with clear responsibilities:

The **tesela-core** crate defines identifiers, errors, values, and domain
enums.

The **tesela-ir** crate defines the canonical `tesela.spec.v1` structures.

The **tesela-compiler** and **tesela-graph** crates validate, normalize, diff,
and analyze ontology graphs.

The **tesela-runtime** crate owns the governed execution pipeline over backend,
policy, audit, upload, action, and agent ports.

The **tesela-memory** crate is an explicit development/test backend adapter.

The **tesela-graphql**, **tesela-server**, **tesela-mcp**, and
**tesela-cabi** crates are runtime wrappers. They delegate to `tesela-runtime`
rather than duplicating policy or adapter behavior.

Language SDKs are maintained source packages, not generated output. They depend
on the published IR shape and native runtime ABI.

The **secrets** package resolves secret references through the configured provider. It depends on core only.

The **telemetry** package provides OpenTelemetry instrumentation helpers. It depends on core only.

## Dependency Direction

Dependencies flow inward toward `tesela-core`, `tesela-ir`, and
`tesela-runtime`. Wrappers do not bypass the runtime pipeline, and production
paths require user-wired adapters, policy evaluation, and audit sinks.
