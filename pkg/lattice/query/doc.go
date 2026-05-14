// Package query is the read-side request orchestrator. Every API endpoint
// that returns ontology data — Get, Search, Aggregate, Traverse — funnels
// through Pipeline, which composes seven stages: actor resolution, ontology
// lookup, policy evaluation, adapter query construction, adapter execution,
// hydration + redaction, audit.
//
// Stages are pure functions. The pipeline composes them via runStages so
// each stage is independently testable and the request flow stays linear.
package query
