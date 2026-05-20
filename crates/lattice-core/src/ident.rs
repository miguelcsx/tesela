//! Strongly-typed identifiers for Lattice.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

/// A validated API name: lowercase ASCII letters, digits, and underscores,
/// starting with a letter.
///
/// # Examples
///
/// ```
/// use lattice_core::ApiName;
/// let name: ApiName = "customer_order".parse().unwrap();
/// assert_eq!(name.as_ref(), "customer_order");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiName(Arc<str>);

impl ApiName {
    /// Regex pattern for valid API names.
    pub const PATTERN: &'static str = r"^[a-z][a-z0-9_]*$";

    /// Create a new `ApiName`, validating the input.
    ///
    /// Returns an error if the string does not match the API name pattern.
    pub fn new<S: AsRef<str>>(s: S) -> Result<Self, crate::Error> {
        let s = s.as_ref();
        if s.is_empty() {
            return Err(crate::Error::validation("api_name cannot be empty"));
        }
        // Use a lazy_static or thread-local regex for performance.
        // For simplicity in a core crate we compile once per call; in practice
        // the caller should cache or use `lazy_regex`.
        static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let re = RE.get_or_init(|| {
            Regex::new(ApiName::PATTERN).expect("BUG: ApiName::PATTERN is a compile-time constant")
        });
        if !re.is_match(s) {
            return Err(crate::Error::validation(format!(
                "api_name '{}' does not match pattern {}",
                s,
                ApiName::PATTERN
            )));
        }
        Ok(Self(Arc::from(s)))
    }

    /// Create an `ApiName` without validation.
    ///
    /// # Safety
    /// The caller must ensure the string matches the API name pattern.
    /// Prefer `ApiName::new` or `parse` in production code.
    pub fn new_unchecked<S: AsRef<str>>(s: S) -> Self {
        Self(Arc::from(s.as_ref()))
    }
}

impl AsRef<str> for ApiName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for ApiName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for ApiName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ApiName {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl From<ApiName> for String {
    fn from(value: ApiName) -> Self {
        value.0.to_string()
    }
}

impl From<&ApiName> for String {
    fn from(value: &ApiName) -> Self {
        value.0.to_string()
    }
}

impl Serialize for ApiName {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ApiName {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

/// A version string for the Lattice spec.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(Arc<str>);

impl Version {
    /// Create a new version.
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self(Arc::from(s.as_ref()))
    }
}

impl AsRef<str> for Version {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Version {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl Serialize for Version {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}
