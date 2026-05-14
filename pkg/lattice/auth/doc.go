// Package auth validates inbound JWT bearer tokens and assembles a
// types.Actor from the resulting claims.
//
// Lattice does not issue tokens — that is the job of the team's identity
// provider (Auth0, Keycloak, Clerk, Cognito, ...). This package only
// validates them: signature against the OIDC discovery JWKS (with automatic
// rotation), expiry, audience, and issuer. It then maps configured claim
// names (UserIDClaim, RolesClaim, plus everything else as opaque Claims)
// onto the canonical types.Actor.
//
// The Authenticator is consumed by the HTTP middleware in internal/api,
// which annotates the request context with the resolved actor for every
// downstream pipeline stage to read.
package auth
