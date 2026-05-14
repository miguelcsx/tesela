// Package ratelimit enforces per-workspace request quotas. The default
// implementation is in-memory token-bucket; production deployments wire
// Redis-backed implementations from pkg/ratelimitbackend (Phase 6.1).
package ratelimit
