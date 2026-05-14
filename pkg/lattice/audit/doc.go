// Package audit is the buffered, append-only writer that every Lattice
// pipeline (query, actions, agents, upload, ontology apply) calls at its
// final stage. The DB layer enforces append-only via REVOKE UPDATE/DELETE on
// the audit_records table; this package guarantees high write throughput
// without losing records on graceful shutdown.
//
// The Writer is in-memory buffered. Write places the record on a channel; a
// background goroutine flushes batches to the audit_records table on either
// size threshold or interval. Flush() forces an immediate drain (used by
// graceful shutdown). On unrecoverable persist errors, the writer logs and
// drops, incrementing a metrics counter — the pipeline never blocks on
// audit. Sink is pluggable for export to Kafka/SIEM in Phase 6.
package audit
