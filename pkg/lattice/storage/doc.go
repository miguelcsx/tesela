// Package store implements Lattice's metadata persistence layer.
//
// The package is structured around two concerns:
//
//  1. Schema management — embedded SQL migrations applied via goose, runnable
//     from a single MigrateUp/MigrateDown entry point.
//
//  2. Typed repositories — one per entity (workspaces, datasources, object
//     types, link types, action types, roles, policy rules, custom tools,
//     agents, assets, action runs, agent runs, uploads, audit records,
//     ontology versions). Each repository exposes a focused interface that
//     higher layers (internal/ontology, internal/actions, internal/audit, ...)
//     consume; the Postgres implementation lives in the pg/ subpackage.
//
// Persistence rules baked into the schema:
//
//   - audit_records is append-only — UPDATE and DELETE on this table are
//     revoked from the application role at migration time.
//   - workspace_id is part of every operational query as the tenant boundary.
//   - action_runs has a UNIQUE (workspace_id, idempotency_key) constraint to
//     enforce idempotency at the database level.
package storage
