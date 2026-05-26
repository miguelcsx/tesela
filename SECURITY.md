# Security policy

This document describes how to report security issues in Tesela and the
hardening guarantees the runtime ships with.

## Reporting a vulnerability

Email security@tesela.example with a description of the issue, the affected
version, and (when possible) a reproduction. We will acknowledge within 48
hours and aim to publish a fix and CVE within 30 days for critical issues.

Please do **not** open a public GitHub issue for security reports.

## Hardening guarantees

* **Authentication**: every authenticated endpoint requires a valid JWT
  validated against the configured OIDC issuer (`internal/auth`). Tokens are
  rejected on signature mismatch, expiry, audience mismatch, or issuer
  mismatch. We do not accept `alg=none`. The accepted algorithms are
  configurable; the default is `RS256`.
* **Authorization**: every read/write/agent operation runs through the
  policy engine (`internal/policy`). Deny rules override allows; an empty
  policy denies by default. Property redactions are enforced both on output
  and on inbound filters/sort to prevent oracle-style information leakage.
* **Tenant isolation**: every persisted query in `internal/store` scopes by
  `workspace_id`. The pgxpool driver uses prepared statements so user input
  is always parameterized.
* **Credentials at rest**: datasource credentials are AES-256-GCM sealed
  (`internal/crypto`) using a KEK loaded through a `secrets.SecretProvider`.
  Sealed blobs are stored as BYTEA; plaintext is held in memory only for the
  lifetime of an adapter `Connect` call.
* **Audit log**: every operation produces an `audit_records` entry. The
  table is `REVOKE UPDATE, DELETE` for the application role at migration
  time, enforcing append-only semantics at the database level.
* **Webhook signing**: webhook handlers sign their bodies with HMAC-SHA256
  and a configurable signing key from the secrets provider. Receivers can
  verify the `X-Tesela-Signature` header to detect tampering or replay.
* **SSRF**: webhook clients honor a per-action timeout and retry budget.
  Future versions will add an outbound allowlist enforced before DNS
  resolution.
* **Rate limiting**: per-workspace rate limits (`internal/ratelimit`)
  enforce request quotas; deployments should additionally place an L7
  rate-limiter (e.g., NGINX `limit_req`, Cloudflare) in front of the API.

## Threat model assumptions

* The metadata Postgres database is trusted. An attacker with direct write
  access to the metadata DB can bypass policy and audit.
* The OIDC issuer and its JWKS endpoint are trusted. Mis-issuance of tokens
  bypasses authentication.
* Object storage backends (S3/GCS) are assumed to enforce the IAM policies
  that gate signed URL validity.

## Disclosure

We follow [coordinated disclosure](https://en.wikipedia.org/wiki/Coordinated_disclosure):
report privately first, fix, then publish.
