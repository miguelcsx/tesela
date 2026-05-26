# ADR-005: Secret Provider Abstraction for Credential Resolution

## Status

Accepted

## Context

Tesela needs access to datasource credentials, object storage credentials, and other secrets to function. Different teams use different secret management systems: environment variables, HashiCorp Vault, AWS Secrets Manager, GCP Secret Manager, and others. Hardcoding support for environment variables only would exclude teams with centralized secret management requirements.

## Decision

Tesela introduces a SecretProvider interface. The configured secret provider resolves secret references — either environment variable names or paths in a secret management system — to their string values. The rest of the system interacts only with resolved string values and is unaware of how those values were obtained.

Datasource connection configurations in the ontology YAML support two forms: an environment variable reference using a dollar-sign-brace syntax, and a structured secret reference with a path understood by the configured provider.

## Reasoning

**Production requirements**: Enterprise teams operating in regulated environments are required to use centralized secret management systems. Vault dynamic credentials, for example, rotate automatically and cannot be stored as environment variables. Supporting only environment variables would exclude these teams.

**Separation of concerns**: The mechanism by which a secret is obtained is independent of how Tesela uses it. The adapter that needs a database password should not care whether that password came from an environment variable, Vault, or AWS Secrets Manager.

**SOPS and similar tools**: Teams using SOPS decrypt secrets to environment variables at deploy time. These teams do not need Tesela to understand SOPS — they use the environment variable provider transparently. The abstraction accommodates both simple and complex secret management workflows.

## Provider Implementations

**Environment provider** (default): Resolves references by reading the process environment. Works with any tool that injects secrets as environment variables before process startup: dotenv files in development, Kubernetes Secrets projected as environment variables, SOPS with env injection, Doppler, 1Password CLI, and similar.

**Vault provider**: Resolves references by reading KV secrets from a HashiCorp Vault cluster. Supports Kubernetes, AppRole, and token authentication methods. Handles lease renewal for dynamic credentials.

**AWS Secrets Manager provider**: Resolves references by reading secrets from AWS Secrets Manager using the instance's IAM role or configured credentials.

**GCP Secret Manager provider**: Resolves references by reading secrets from GCP Secret Manager using Application Default Credentials.

## Trade-offs Accepted

Secret references in the ontology YAML are opaque strings — their format depends on the configured provider. A configuration file with Vault secret references cannot be used directly with the AWS Secrets Manager provider without modification. Teams that switch secret providers must update their secret reference format.

## Consequences

The server configuration file specifies which secret provider to use and its configuration (endpoint, authentication method). The secret provider is initialized at startup. Datasource credentials are resolved once at startup and cached — they are not re-resolved on every request. For dynamic credentials with short leases, the Vault provider implements background lease renewal.
