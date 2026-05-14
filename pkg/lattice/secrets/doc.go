// Package secrets resolves opaque secret references to plain string values.
//
// A SecretProvider is the only sanctioned way to read a credential at runtime;
// adapters and other consumers receive resolved values, not references. The
// package ships with two implementations:
//
//   - EnvProvider: looks up the reference in process environment variables.
//     Suitable when secrets are injected at process start (Kubernetes Secrets,
//     SOPS-decrypted env files, dotenv, Doppler, 1Password CLI, ...).
//
//   - StaticProvider: backed by an in-memory map; intended for tests and for
//     a small number of bootstrap-time secrets read from configuration files.
//
// Additional providers (Vault, AWS Secrets Manager, GCP Secret Manager,
// Kubernetes inline secrets) are added in Phase 6 and implement the same
// SecretProvider interface; nothing else in the codebase needs to change.
//
// References can be supplied in two equivalent forms throughout configuration:
//
//   - "${NAME}" — looked up via the configured provider.
//   - "literal" — used as-is.
//
// Use ResolveReference (single value) or ResolveReferences (whole map) to
// expand both forms uniformly.
package secrets
