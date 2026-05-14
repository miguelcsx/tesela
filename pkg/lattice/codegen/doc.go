// Package codegen produces SDKs (TypeScript, Python, Go, Rust) from a live
// ontology snapshot. Each language target is a small generator that reads
// the ontology and emits a tarred zip the user can download.
//
// The generators are intentionally simple: a single template per language
// expands per object type / action type. They produce typed data classes
// plus a thin client that hits the lattice-api REST endpoints.
package codegen
