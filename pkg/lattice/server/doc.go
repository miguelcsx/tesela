// Package api owns the HTTP server: chi router, middleware chain, request
// parsers, response encoders, and the per-resource handlers that translate
// HTTP into pipeline calls.
//
// Handlers are intentionally small — at most a few dozen lines each — and
// delegate everything to the underlying domain packages (query, actions,
// agents, upload, ontology). All cross-cutting behavior (auth, request id,
// logging, error mapping, telemetry) lives in middleware.
package server
