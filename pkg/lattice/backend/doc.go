// Package backend defines the contract every external data store must
// implement to participate in the Lattice ontology runtime, and the registry
// that wires concrete implementations to a workspace's datasources.
//
// Backends live in user code or example packages; this
// package owns only the interface contract and the in-memory registry.
//
// The registry is responsible for translating a sealed-credentials Datasource
// into a live Connection: it asks the SecretProvider to resolve credential
// refs, asks the Sealer to open the BYTEA blob, and merges both into the
// adapter ConfigMap before calling Connect.
package backend
