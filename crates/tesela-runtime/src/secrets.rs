//! Secret resolution implementations.
//!
//! All resolvers implement [`SecretResolver`] and are backend-agnostic.
//! For Vault/AWS Secrets Manager integration, implement the port directly.

use crate::ports::SecretResolver;
use tesela_core::Error;
use std::collections::BTreeMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// EnvSecretResolver — reads from environment variables
// ---------------------------------------------------------------------------

/// Resolves secrets from environment variables.
///
/// A `secret_ref` like `DB_PASSWORD` is looked up directly in the process
/// environment.  An optional prefix is stripped first, so `secret:DB_PASSWORD`
/// with prefix `"secret:"` becomes `DB_PASSWORD`.
pub struct EnvSecretResolver {
    prefix: Option<String>,
}

impl EnvSecretResolver {
    /// Create a resolver with no prefix stripping.
    pub fn new() -> Self {
        Self { prefix: None }
    }

    /// Create a resolver that strips `prefix` before the env-var lookup.
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: Some(prefix.into()),
        }
    }
}

impl Default for EnvSecretResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretResolver for EnvSecretResolver {
    fn resolve(&self, secret_ref: &str) -> Result<String, Error> {
        let key = match &self.prefix {
            Some(pfx) => secret_ref.strip_prefix(pfx.as_str()).unwrap_or(secret_ref),
            None => secret_ref,
        };
        std::env::var(key).map_err(|_| Error::not_found("environment variable", key))
    }
}

// ---------------------------------------------------------------------------
// StaticSecretResolver — in-memory map, useful for tests
// ---------------------------------------------------------------------------

/// Resolves secrets from a pre-populated in-memory map.
///
/// Intended for tests and local development.  Do not use in production
/// without coupling to a real secret store.
pub struct StaticSecretResolver {
    secrets: BTreeMap<String, String>,
}

impl StaticSecretResolver {
    /// Create a resolver from a map of `reference → value`.
    pub fn new(secrets: BTreeMap<String, String>) -> Self {
        Self { secrets }
    }
}

impl SecretResolver for StaticSecretResolver {
    fn resolve(&self, secret_ref: &str) -> Result<String, Error> {
        self.secrets
            .get(secret_ref)
            .cloned()
            .ok_or_else(|| Error::not_found("secret", secret_ref))
    }
}

// ---------------------------------------------------------------------------
// ChainSecretResolver — tries resolvers in order
// ---------------------------------------------------------------------------

/// Tries a chain of [`SecretResolver`]s in order, returning the first
/// successful resolution.
///
/// If all resolvers fail (return `Error::NotFound`), the last error is
/// returned.  Non-not-found errors propagate immediately.
pub struct ChainSecretResolver {
    resolvers: Vec<Arc<dyn SecretResolver>>,
}

impl ChainSecretResolver {
    /// Create a chain from an ordered list of delegates.
    pub fn new(resolvers: Vec<Arc<dyn SecretResolver>>) -> Self {
        Self { resolvers }
    }
}

impl SecretResolver for ChainSecretResolver {
    fn resolve(&self, secret_ref: &str) -> Result<String, Error> {
        let mut last = Error::not_found("secret", secret_ref);
        for r in &self.resolvers {
            match r.resolve(secret_ref) {
                Ok(v) => return Ok(v),
                Err(e @ Error::NotFound { .. }) => last = e,
                Err(e) => return Err(e),
            }
        }
        Err(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_resolver_hits() {
        let mut m = BTreeMap::new();
        m.insert("DB_PASS".to_string(), "hunter2".to_string());
        let r = StaticSecretResolver::new(m);
        assert_eq!(r.resolve("DB_PASS").unwrap(), "hunter2");
    }

    #[test]
    fn static_resolver_miss() {
        let r = StaticSecretResolver::new(BTreeMap::new());
        assert!(r.resolve("NOPE").is_err());
    }

    #[test]
    fn chain_falls_through() {
        let a: Arc<dyn SecretResolver> = Arc::new(StaticSecretResolver::new(BTreeMap::new()));
        let mut m = BTreeMap::new();
        m.insert("KEY".to_string(), "val".to_string());
        let b: Arc<dyn SecretResolver> = Arc::new(StaticSecretResolver::new(m));
        let chain = ChainSecretResolver::new(vec![a, b]);
        assert_eq!(chain.resolve("KEY").unwrap(), "val");
    }

    #[test]
    fn env_resolver_reads_env() {
        unsafe { std::env::set_var("TESELA_TEST_SECRET_XYZ", "abc") };
        let r = EnvSecretResolver::new();
        assert_eq!(r.resolve("TESELA_TEST_SECRET_XYZ").unwrap(), "abc");
        unsafe { std::env::remove_var("TESELA_TEST_SECRET_XYZ") };
    }
}
