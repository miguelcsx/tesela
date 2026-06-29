//! Strongly-typed identifiers for Tesela.

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
/// use tesela_core::ApiName;
/// let name = ApiName::new("customer_order")?;
/// assert_eq!(name.as_ref(), "customer_order");
/// # Ok::<(), tesela_core::Error>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiName(Arc<str>);

/// A typed source for stable Tesela API names.
///
/// Platform code should implement this trait for enums that name datasources,
/// object types, fields, tools, and other durable contracts. This keeps string
/// literals at the boundary of the enum implementation instead of spreading
/// magic strings across ontology definitions.
pub trait ApiNameSource {
    /// Return the stable API name.
    fn api_name(&self) -> &'static str;

    /// Convert the stable API name into a validated Tesela identifier.
    fn to_api_name(&self) -> ApiName {
        ApiName::from(self.api_name())
    }
}

impl ApiNameSource for &'static str {
    fn api_name(&self) -> &'static str {
        self
    }
}

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
        let re = RE.get_or_init(|| match Regex::new(ApiName::PATTERN) {
            Ok(regex) => regex,
            Err(error) => panic!("invalid ApiName pattern: {error}"),
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

/// Create an `ApiName` from a `&'static str` without runtime validation.
///
/// Use for string literals known at compile time to be valid API names.
/// Analogous to `http::Method::from_static`. Panics in debug builds if the
/// string is not a valid API name, so mistakes surface in tests rather than
/// production.
impl From<&'static str> for ApiName {
    fn from(s: &'static str) -> Self {
        debug_assert!(
            Self::new(s).is_ok(),
            "'{s}' is not a valid ApiName (must match {})",
            Self::PATTERN
        );
        Self::new_unchecked(s)
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

/// A version string for the Tesela spec.
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
