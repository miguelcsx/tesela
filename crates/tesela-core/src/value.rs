//! Value wrapper for record data.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::Deref;

/// A thin newtype around `serde_json::Value` that provides [`Ord`] support
/// so that values can be used as keys in `BTreeMap` when needed.
///
/// The ordering is best-effort: strings < numbers < booleans < null < arrays < objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Value(pub serde_json::Value);

impl Value {
    /// Create a `Value` from a `serde_json::Value`.
    pub fn new(v: serde_json::Value) -> Self {
        Self(v)
    }

    /// Create a null value.
    pub fn null() -> Self {
        Self(serde_json::Value::Null)
    }

    /// Create a string value.
    pub fn string<S: Into<String>>(s: S) -> Self {
        Self(serde_json::Value::String(s.into()))
    }

    /// Create a boolean value.
    pub fn bool(b: bool) -> Self {
        Self(serde_json::Value::Bool(b))
    }

    /// Create an integer value.
    pub fn integer(i: i64) -> Self {
        Self(serde_json::Value::from(i))
    }

    /// Create a float value.
    pub fn float(f: f64) -> Self {
        Self(serde_json::Value::from(f))
    }

    /// Returns `true` if this value is null.
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// Returns the value as a string, if it is one.
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_str()
    }

    /// Returns the value as an integer, if it is one.
    pub fn as_i64(&self) -> Option<i64> {
        self.0.as_i64()
    }

    /// Returns the value as a float, if it is one.
    pub fn as_f64(&self) -> Option<f64> {
        self.0.as_f64()
    }

    /// Returns the value as a bool, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        self.0.as_bool()
    }

    /// Returns the value as an array, if it is one.
    pub fn as_array(&self) -> Option<&Vec<serde_json::Value>> {
        self.0.as_array()
    }

    /// Returns the value as an object, if it is one.
    pub fn as_object(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.0.as_object()
    }
}

impl Deref for Value {
    type Target = serde_json::Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        Self(v)
    }
}

impl From<Value> for serde_json::Value {
    fn from(v: Value) -> Self {
        v.0
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self(serde_json::Value::String(s))
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self(serde_json::Value::String(s.to_string()))
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Self(serde_json::Value::from(i))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Self(serde_json::Value::from(f))
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self(serde_json::Value::Bool(b))
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        use serde_json::Value as Json;
        match (&self.0, &other.0) {
            (Json::Null, Json::Null) => Ordering::Equal,
            (Json::Null, _) => Ordering::Greater,
            (_, Json::Null) => Ordering::Less,
            (Json::Bool(a), Json::Bool(b)) => a.cmp(b),
            (Json::Bool(_), _) => Ordering::Less,
            (_, Json::Bool(_)) => Ordering::Greater,
            (Json::Number(a), Json::Number(b)) => {
                // Try integer comparison first, then float.
                match (a.as_i64(), b.as_i64()) {
                    (Some(ai), Some(bi)) => ai.cmp(&bi),
                    _ => {
                        let af = number_as_f64(a);
                        let bf = number_as_f64(b);
                        match af.partial_cmp(&bf) {
                            Some(ordering) => ordering,
                            None => Ordering::Equal,
                        }
                    }
                }
            }
            (Json::Number(_), _) => Ordering::Less,
            (_, Json::Number(_)) => Ordering::Greater,
            (Json::String(a), Json::String(b)) => a.cmp(b),
            (Json::String(_), _) => Ordering::Less,
            (_, Json::String(_)) => Ordering::Greater,
            (Json::Array(a), Json::Array(b)) => a.len().cmp(&b.len()).then_with(|| {
                a.iter()
                    .map(|v| Value::new(v.clone()))
                    .cmp(b.iter().map(|v| Value::new(v.clone())))
            }),
            (Json::Array(_), _) => Ordering::Less,
            (_, Json::Array(_)) => Ordering::Greater,
            (Json::Object(a), Json::Object(b)) => a.len().cmp(&b.len()).then_with(|| {
                let mut av: Vec<(&String, Value)> =
                    a.iter().map(|(k, v)| (k, Value::new(v.clone()))).collect();
                let mut bv: Vec<(&String, Value)> =
                    b.iter().map(|(k, v)| (k, Value::new(v.clone()))).collect();
                av.sort_by(|a, b| a.0.cmp(b.0));
                bv.sort_by(|a, b| a.0.cmp(b.0));
                av.cmp(&bv)
            }),
        }
    }
}

fn number_as_f64(value: &serde_json::Number) -> f64 {
    if let Some(number) = value.as_f64() {
        return number;
    }
    f64::NAN
}

impl Default for Value {
    fn default() -> Self {
        Self::null()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Self(serde_json::Value::Array(
            v.into_iter().map(|x| x.0).collect(),
        ))
    }
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash based on the JSON type tag and content.
        std::mem::discriminant(&self.0).hash(state);
        match &self.0 {
            serde_json::Value::Null => {}
            serde_json::Value::Bool(b) => b.hash(state),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    i.hash(state);
                } else if let Some(f) = n.as_f64() {
                    f.to_bits().hash(state);
                }
            }
            serde_json::Value::String(s) => s.hash(state),
            serde_json::Value::Array(arr) => arr.hash(state),
            serde_json::Value::Object(map) => {
                for (k, v) in map.iter() {
                    k.hash(state);
                    v.hash(state);
                }
            }
        }
    }
}
