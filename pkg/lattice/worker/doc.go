// Package worker is the async job runtime. It owns the queue consumer (the
// default backend is a Postgres-backed polling poller; River is an optional
// drop-in replacement) and the per-kind handlers.
//
// Phase 2 ships with an in-process polling consumer that scans the
// action_runs table for pending rows and re-enters the action pipeline.
// Phase 6 introduces alternative queues (River, Redis, NATS).
package worker
