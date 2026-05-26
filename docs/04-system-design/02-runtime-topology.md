# Runtime Topology

## Runtime Topologies

Tesela supports embedded and server topologies that teams choose based on scale
and operational requirements. All topologies consume the same canonical IR. The
difference is whether callers invoke the runtime in-process through a language
SDK/native ABI or expose an HTTP server around that runtime.

## Embedded Native Runtime

Language SDKs in Python, Rust, and future languages build
`tesela.spec.v1` JSON and instantiate the same Rust runtime through a native ABI
or direct Rust calls. Backends and action handlers are registered as callbacks by
symbolic adapter type or handler kind. Runtime calls such as search, get,
mutate, and execute action cross the native boundary as JSON payloads matching
the public IR and port types.

This topology is the default SDK story. It does not use HTTP between the SDK
and the runtime and does not rely on generated client code.

## Single-Process Server (Local Development)

All functionality runs in a single process started by the CLI's development mode command. The metadata database uses SQLite or a local Postgres instance started automatically by the CLI. Object storage uses the local filesystem. There is no queue — async jobs execute inline. This topology starts in seconds and requires no external dependencies beyond a database.

This topology is for development and testing only. It is not suitable for production because it does not support concurrent load, has no fault isolation between components, and uses simplified storage backends.

## Modular Monolith (Small Production)

The API server and worker run as separate processes but deploy on the same host or in the same container group. They share a single Postgres database for metadata and use the team's object storage. Async jobs use an embedded in-memory queue with persistence to Postgres. This topology handles moderate load, supports independent scaling of API and worker, and is appropriate for teams with a single deployment environment.

## Split Runtime (Production at Scale)

The API server runs as a horizontally scaled set of stateless replicas behind a load balancer. Multiple worker instances pull from an external queue (a Postgres-backed queue, Redis queue, or any supported broker adapter). The metadata database is a managed, high-availability Postgres instance. Object storage is provided by the team's cloud provider. Datasources are external managed services.

In this topology, all components are independently scalable. API replicas scale for request throughput. Worker replicas scale for job throughput. The metadata database scales vertically or moves to a distributed SQL system for multi-region deployments.

## Component Interaction Map

The API server reads from and writes to the metadata database for ontology management, action run creation, and audit logging. It reads ontology data primarily from the in-memory cache. It calls datasource adapters for query execution. It calls the secrets provider to resolve credentials. It writes to object storage when generating upload signed URLs. It emits telemetry to the configured OTEL collector.

The worker reads job records from the metadata database, executes ingestion and transformation jobs, calls datasource adapters for bulk load operations, reads from and writes to object storage during ingestion, and writes results back to the metadata database.

The CLI validates, diffs, packages, and can serve IR specs. SDK/runtime
interaction stays native and local unless a user explicitly chooses to wrap the
runtime in an HTTP server.

## Network Boundaries

The API server exposes one port for HTTP traffic (REST and GraphQL). It does not expose any other port. All internal communication between the API server and adapters, the secrets provider, and object storage is outbound from the server — no inbound connections from these systems are required.

The worker exposes no HTTP port. All its activity is initiated by pulling from the job queue and making outbound calls to adapters and object storage.
