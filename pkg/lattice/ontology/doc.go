// Package ontology owns the in-memory ontology cache, the declarative
// document (de)serialization, the validator, the diff engine, and the registry that
// stitches together the metadata store and the rest of the runtime.
//
// The Registry is the single source of truth for "what is the ontology of
// workspace X right now?". Every reader (query pipeline, action runtime,
// agent runtime, GraphQL builder, SDK codegen) accesses the ontology
// exclusively through Snapshot, which returns an immutable *types.Ontology.
//
// Apply is the public mutation entry point: it parses a declarative document,
// runs the validator, computes a Diff against the current snapshot, persists
// every changed entity inside a single store transaction, then atomically
// installs a fresh snapshot and notifies subscribers.
//
// Hot-reload is implemented via atomic.Pointer[types.Ontology] swap, so reads
// are O(1) and free of locks. Subscribers receive Changes asynchronously on
// a buffered channel; slow consumers get dropped change events rather than
// stalling the producer.
package ontology
