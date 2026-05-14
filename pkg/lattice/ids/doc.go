// Package ids provides identifier factories used across Lattice.
//
// Two flavors:
//
//   - ULID (26-char Crockford-base32) — used for entities where time-ordering
//     matters: action runs, agent runs, audit records, jobs, uploads. ULIDs
//     produced from the same goroutine within the same millisecond are
//     guaranteed to be lexicographically ordered.
//
//   - UUID (RFC 4122 v4) — used where time-ordering is irrelevant or
//     undesirable: workspace identifiers, datasource handles, opaque tokens.
//
// Both factories are safe for concurrent use and never panic; entropy comes
// from crypto/rand.
package ids
