//! Configuration source port and environment-variable implementation.
//!
//! [`ConfigSource`] is the agnostic port; adapters for YAML files, Consul KV,
//! or AWS AppConfig can implement it directly.

use lattice_core::{lock_read, lock_write, Error};
use std::collections::BTreeMap;
use std::sync::RwLock;

// ---------------------------------------------------------------------------
// ConfigSource port
// ---------------------------------------------------------------------------

/// Read-only configuration source.
///
/// Keys are dot-separated paths, e.g. `"server.max_connections"`.
/// Returns `Ok(None)` for unknown keys.
///
/// This trait is dyn-compatible: use [`config_require`] and [`config_get_or`]
/// for the typed convenience helpers that cannot be object-safe methods.
pub trait ConfigSource: Send + Sync {
    /// Retrieve a configuration value by key.
    fn get(&self, key: &str) -> Result<Option<String>, Error>;
}

/// Return a required config value, failing if the key is absent.
pub fn config_require(src: &dyn ConfigSource, key: &str) -> Result<String, Error> {
    src.get(key)?
        .ok_or_else(|| Error::not_found("config key", key))
}

/// Return a config value parsed into `T`, or `default` when absent.
pub fn config_get_or<T>(src: &dyn ConfigSource, key: &str, default: T) -> Result<T, Error>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match src.get(key)? {
        Some(v) => v
            .parse::<T>()
            .map_err(|e| Error::validation(format!("config key {:?}: {}", key, e))),
        None => Ok(default),
    }
}

// ---------------------------------------------------------------------------
// EnvConfigSource — reads from environment variables
// ---------------------------------------------------------------------------

/// Reads configuration from environment variables.
///
/// Dot-separated key paths are converted to uppercase with dots replaced by
/// underscores, optionally prefixed.
///
/// Example: key `"server.port"` with prefix `"LATTICE"` → `LATTICE_SERVER_PORT`.
pub struct EnvConfigSource {
    prefix: Option<String>,
}

impl EnvConfigSource {
    /// Create a source with no prefix.
    pub fn new() -> Self {
        Self { prefix: None }
    }

    /// Create a source that prepends `prefix` (e.g. `"LATTICE"`).
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: Some(prefix.into()),
        }
    }

    fn env_key(&self, key: &str) -> String {
        let upper = key.to_uppercase().replace('.', "_");
        match &self.prefix {
            Some(p) => format!("{}_{}", p.to_uppercase(), upper),
            None => upper,
        }
    }
}

impl Default for EnvConfigSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for EnvConfigSource {
    fn get(&self, key: &str) -> Result<Option<String>, Error> {
        let env_key = self.env_key(key);
        Ok(std::env::var(&env_key).ok())
    }
}

// ---------------------------------------------------------------------------
// StaticConfigSource — in-memory map, useful for tests
// ---------------------------------------------------------------------------

/// Configuration source backed by an in-memory map.
///
/// Keys are matched exactly (no dot-to-underscore conversion).
pub struct StaticConfigSource {
    values: BTreeMap<String, String>,
}

impl StaticConfigSource {
    /// Create a source from a map of key→value pairs.
    pub fn new(values: BTreeMap<String, String>) -> Self {
        Self { values }
    }
}

impl ConfigSource for StaticConfigSource {
    fn get(&self, key: &str) -> Result<Option<String>, Error> {
        Ok(self.values.get(key).cloned())
    }
}

// ---------------------------------------------------------------------------
// LayeredConfigSource — overlay multiple sources, first hit wins
// ---------------------------------------------------------------------------

/// Layers multiple [`ConfigSource`]s.  Sources are queried in registration
/// order; the first non-`None` result is returned.
pub struct LayeredConfigSource {
    layers: Vec<Box<dyn ConfigSource>>,
}

impl LayeredConfigSource {
    /// Create an empty layered source.
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Append a layer.  Layers added first take precedence (highest priority).
    pub fn push(mut self, source: impl ConfigSource + 'static) -> Self {
        self.layers.push(Box::new(source));
        self
    }
}

impl Default for LayeredConfigSource {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for LayeredConfigSource {
    fn get(&self, key: &str) -> Result<Option<String>, Error> {
        for layer in &self.layers {
            if let Some(v) = layer.get(key)? {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// CachedConfigSource — memoises reads from an underlying source
// ---------------------------------------------------------------------------

/// Wraps another [`ConfigSource`] and caches every result after the first
/// lookup.  Useful when the underlying source performs I/O.
pub struct CachedConfigSource<S: ConfigSource> {
    inner: S,
    cache: RwLock<BTreeMap<String, Option<String>>>,
}

impl<S: ConfigSource> CachedConfigSource<S> {
    /// Create a cached wrapper over `inner`.
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            cache: RwLock::new(BTreeMap::new()),
        }
    }
}

impl<S: ConfigSource> ConfigSource for CachedConfigSource<S> {
    fn get(&self, key: &str) -> Result<Option<String>, Error> {
        {
            let cache = lock_read(&self.cache)?;
            if let Some(v) = cache.get(key) {
                return Ok(v.clone());
            }
        }
        let v = self.inner.get(key)?;
        lock_write(&self.cache)?.insert(key.to_string(), v.clone());
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn static_src(pairs: &[(&str, &str)]) -> StaticConfigSource {
        StaticConfigSource::new(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }

    #[test]
    fn static_get_hit() {
        let s = static_src(&[("key", "val")]);
        assert_eq!(s.get("key").unwrap(), Some("val".to_string()));
    }

    #[test]
    fn static_get_miss() {
        let s = static_src(&[]);
        assert_eq!(s.get("missing").unwrap(), None);
    }

    #[test]
    fn require_errors_on_missing() {
        let s = static_src(&[]);
        assert!(config_require(&s, "x").is_err());
    }

    #[test]
    fn get_or_parses_and_defaults() {
        let s = static_src(&[("port", "8080")]);
        assert_eq!(config_get_or(&s, "port", 3000u16).unwrap(), 8080u16);
        assert_eq!(config_get_or(&s, "missing_port", 3000u16).unwrap(), 3000u16);
    }

    #[test]
    fn layered_first_wins() {
        let high = static_src(&[("k", "high")]);
        let low = static_src(&[("k", "low"), ("only_low", "yes")]);
        let l = LayeredConfigSource::new().push(high).push(low);
        assert_eq!(l.get("k").unwrap(), Some("high".to_string()));
        assert_eq!(l.get("only_low").unwrap(), Some("yes".to_string()));
    }

    #[test]
    fn env_config_source_maps_key() {
        unsafe { std::env::set_var("LATTICE_SERVER_PORT", "9090") };
        let s = EnvConfigSource::with_prefix("LATTICE");
        assert_eq!(s.get("server.port").unwrap(), Some("9090".to_string()));
        unsafe { std::env::remove_var("LATTICE_SERVER_PORT") };
    }
}
