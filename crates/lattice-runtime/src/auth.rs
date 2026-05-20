//! Actor resolution implementations.
//!
//! All resolvers implement [`ActorResolver`] and are backend-agnostic.
//! For OIDC/JWT validation, wire your own implementation through the port.

use crate::ports::ActorResolver;
use crate::query::{Actor, RequestMeta};
use lattice_core::Error;
use std::collections::BTreeMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// StaticActorResolver — fixed actor, useful in tests and single-tenant CLIs
// ---------------------------------------------------------------------------

/// Resolves every request to the same pre-configured [`Actor`].
///
/// Suitable for single-tenant deployments, CLI tools, and tests.  **Do not
/// use in multi-tenant HTTP servers without additional authentication.**
pub struct StaticActorResolver {
    actor: Actor,
}

impl StaticActorResolver {
    /// Create a resolver that always returns `actor`.
    pub fn new(actor: Actor) -> Self {
        Self { actor }
    }
}

impl ActorResolver for StaticActorResolver {
    fn resolve(&self, _request: &RequestMeta) -> Result<Actor, Error> {
        Ok(self.actor.clone())
    }
}

// ---------------------------------------------------------------------------
// ApiKeyActorResolver — maps `X-Api-Key` header values to actors
// ---------------------------------------------------------------------------

/// Resolves an actor by looking up the `X-Api-Key` request header in a
/// pre-populated map.
///
/// Unknown keys are rejected with [`Error::Unauthorized`].  The key map is
/// immutable after construction (rotate by creating a new resolver).
pub struct ApiKeyActorResolver {
    /// Maps raw API key → Actor.
    keys: BTreeMap<String, Actor>,
    /// Header name to inspect (default: `x-api-key`).
    header: String,
}

impl ApiKeyActorResolver {
    /// Create a resolver with the given key→actor mapping.
    ///
    /// `header` is the lowercase HTTP header name (e.g. `"x-api-key"`).
    pub fn new(keys: BTreeMap<String, Actor>, header: impl Into<String>) -> Self {
        Self {
            keys,
            header: header.into(),
        }
    }

    /// Convenience constructor using the standard `x-api-key` header.
    pub fn with_keys(keys: BTreeMap<String, Actor>) -> Self {
        Self::new(keys, "x-api-key")
    }
}

impl ActorResolver for ApiKeyActorResolver {
    fn resolve(&self, request: &RequestMeta) -> Result<Actor, Error> {
        let key = request
            .headers
            .get(&self.header)
            .or_else(|| request.headers.get("x-api-key"))
            .ok_or_else(|| Error::unauthorized("missing API key header"))?;

        self.keys
            .get(key)
            .cloned()
            .ok_or_else(|| Error::unauthorized("invalid API key"))
    }
}

// ---------------------------------------------------------------------------
// BearerTokenActorResolver — maps `Authorization: Bearer <token>` to actors
// ---------------------------------------------------------------------------

/// Resolves an actor by stripping the `Bearer ` prefix from the
/// `Authorization` header and looking the token up in a pre-populated map.
///
/// For production OIDC/JWT validation, implement [`ActorResolver`] directly
/// and verify the token against a JWKS endpoint.
pub struct BearerTokenActorResolver {
    tokens: BTreeMap<String, Actor>,
}

impl BearerTokenActorResolver {
    /// Create a resolver with the given token→actor mapping.
    pub fn new(tokens: BTreeMap<String, Actor>) -> Self {
        Self { tokens }
    }
}

impl ActorResolver for BearerTokenActorResolver {
    fn resolve(&self, request: &RequestMeta) -> Result<Actor, Error> {
        let auth = request
            .authorization
            .as_deref()
            .ok_or_else(|| Error::unauthorized("missing Authorization header"))?;

        let token = auth
            .strip_prefix("Bearer ")
            .or_else(|| auth.strip_prefix("bearer "))
            .ok_or_else(|| Error::unauthorized("Authorization header must use Bearer scheme"))?;

        self.tokens
            .get(token)
            .cloned()
            .ok_or_else(|| Error::unauthorized("invalid bearer token"))
    }
}

// ---------------------------------------------------------------------------
// CompositeActorResolver — try resolvers in order, first success wins
// ---------------------------------------------------------------------------

/// Tries a chain of [`ActorResolver`]s in order, returning the first
/// successful resolution.
///
/// If all resolvers fail, returns the last error.  Use this to support both
/// API key and bearer token auth on the same server.
pub struct CompositeActorResolver {
    resolvers: Vec<Arc<dyn ActorResolver>>,
}

impl CompositeActorResolver {
    /// Create a composite resolver from an ordered list of delegates.
    pub fn new(resolvers: Vec<Arc<dyn ActorResolver>>) -> Self {
        Self { resolvers }
    }
}

impl ActorResolver for CompositeActorResolver {
    fn resolve(&self, request: &RequestMeta) -> Result<Actor, Error> {
        let mut last_err = Error::unauthorized("no resolvers configured");
        for resolver in &self.resolvers {
            match resolver.resolve(request) {
                Ok(actor) => return Ok(actor),
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(header: &str, value: &str) -> RequestMeta {
        let mut headers = BTreeMap::new();
        headers.insert(header.to_string(), value.to_string());
        RequestMeta {
            authorization: None,
            headers,
            remote_addr: None,
            workspace: None,
            correlation_id: None,
        }
    }

    fn guest() -> Actor {
        Actor {
            user_id: "guest".into(),
            roles: vec!["guest".into()],
            claims: BTreeMap::new(),
        }
    }

    #[test]
    fn static_resolver_always_succeeds() {
        let r = StaticActorResolver::new(guest());
        let meta = RequestMeta {
            authorization: None,
            headers: BTreeMap::new(),
            remote_addr: None,
            workspace: None,
            correlation_id: None,
        };
        assert_eq!(r.resolve(&meta).unwrap().user_id, "guest");
    }

    #[test]
    fn api_key_resolver_accepts_valid_key() {
        let mut keys = BTreeMap::new();
        keys.insert("secret123".to_string(), guest());
        let r = ApiKeyActorResolver::with_keys(keys);
        let m = meta("x-api-key", "secret123");
        assert!(r.resolve(&m).is_ok());
    }

    #[test]
    fn api_key_resolver_rejects_unknown_key() {
        let r = ApiKeyActorResolver::with_keys(BTreeMap::new());
        let m = meta("x-api-key", "bad");
        assert!(r.resolve(&m).is_err());
    }

    #[test]
    fn bearer_resolver_strips_prefix() {
        let mut tokens = BTreeMap::new();
        tokens.insert("tok".to_string(), guest());
        let r = BearerTokenActorResolver::new(tokens);
        let mut m = RequestMeta {
            authorization: Some("Bearer tok".into()),
            headers: BTreeMap::new(),
            remote_addr: None,
            workspace: None,
            correlation_id: None,
        };
        assert!(r.resolve(&m).is_ok());
        m.authorization = Some("Bearer wrong".into());
        assert!(r.resolve(&m).is_err());
    }

    #[test]
    fn composite_falls_through() {
        let mut keys = BTreeMap::new();
        keys.insert("k".to_string(), guest());
        let a: Arc<dyn ActorResolver> = Arc::new(ApiKeyActorResolver::with_keys(BTreeMap::new()));
        let b: Arc<dyn ActorResolver> = Arc::new(ApiKeyActorResolver::with_keys(keys));
        let r = CompositeActorResolver::new(vec![a, b]);
        let m = meta("x-api-key", "k");
        assert!(r.resolve(&m).is_ok());
    }
}
