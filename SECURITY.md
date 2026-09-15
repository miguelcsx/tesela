# Security

API names parsed from JSON are validated, and constructing a runtime requires a policy engine. The `AllowAllPolicy` helper is for trusted local development. Application operators provide authentication, concrete policy decisions, data stores and transport security.

Audit and event ports are optional; Tesela does not persist an append-only audit log or provide JWT/OIDC authentication, tenant isolation, encryption, webhook delivery or rate limiting. The runtime invokes configured ports after completed store operations; operators must configure those ports if their deployment requires them.

Report suspected vulnerabilities privately to the repository maintainer before opening a public issue. Include a minimal reproduction and affected version when possible.
