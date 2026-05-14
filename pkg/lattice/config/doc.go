// Package config defines the canonical Lattice configuration schema and the
// loader that hydrates it from layered sources.
//
// Sources are merged in this order (later overrides earlier):
//
//  1. Built-in defaults
//  2. Optional YAML file (path supplied by --config or LATTICE_CONFIG)
//  3. Environment variables prefixed with LATTICE_, with nested keys joined
//     by underscores (e.g., LATTICE_HTTP_LISTEN → http.listen)
//  4. Explicit overrides set programmatically (used by tests)
//
// Each binary owns a config struct: APIConfig (cmd/lattice-api), WorkerConfig
// (cmd/lattice-worker), CLIConfig (cmd/lattice). They share common sub-schemas
// (Service, MetadataDB, Secrets, Crypto, Auth, Telemetry) so the wire format
// of a single lattice.yaml drives all three binaries.
package config
