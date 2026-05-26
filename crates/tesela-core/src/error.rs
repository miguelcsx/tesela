//! Typed errors for the Tesela runtime.
//!
//! Every public function in the Tesela ecosystem returns `Result<T, Error>`.
//! This crate never uses `anyhow` in library surfaces.

use std::fmt;
use thiserror::Error;

/// The primary error type for Tesela operations.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Error {
    /// Validation failure (malformed spec, invalid references, etc.).
    #[error("validation failed: {message}")]
    Validation {
        /// Human-readable description.
        message: String,
    },

    /// Entity not found.
    #[error("not found: {resource_kind} '{resource}'")]
    NotFound {
        /// Kind of resource (object_type, action, agent, etc.).
        resource_kind: String,
        /// Identifier of the missing resource.
        resource: String,
    },

    /// Policy denied the operation.
    #[error("policy denied: {reason}")]
    PolicyDenied {
        /// Reason for denial.
        reason: String,
    },

    /// Backend does not support a requested capability.
    #[error("unsupported capability: {capability}")]
    UnsupportedCapability {
        /// Name of the unsupported capability.
        capability: String,
    },

    /// Backend adapter failure.
    #[error("adapter error: {message}")]
    Adapter {
        /// Description.
        message: String,
        /// Optional underlying source description.
        source_msg: Option<String>,
    },

    /// Internal runtime error.
    #[error("internal error: {message}")]
    Internal {
        /// Description.
        message: String,
    },

    /// Unauthorized / authentication failure.
    #[error("unauthorized: {message}")]
    Unauthorized {
        /// Description.
        message: String,
    },

    /// Bad request (malformed JSON, missing fields, etc.).
    #[error("bad request: {message}")]
    BadRequest {
        /// Description.
        message: String,
    },

    /// Conflict (duplicate entity, concurrent modification, etc.).
    #[error("conflict: {message}")]
    Conflict {
        /// Description.
        message: String,
    },

    /// Operation timed out.
    #[error("timeout: {message}")]
    Timeout {
        /// Description.
        message: String,
    },
}

impl Error {
    /// Shorthand for creating a validation error.
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Shorthand for creating a not-found error.
    pub fn not_found<K: fmt::Display, R: fmt::Display>(resource_kind: K, resource: R) -> Self {
        Self::NotFound {
            resource_kind: resource_kind.to_string(),
            resource: resource.to_string(),
        }
    }

    /// Shorthand for creating a policy-denied error.
    pub fn policy_denied<S: Into<String>>(reason: S) -> Self {
        Self::PolicyDenied {
            reason: reason.into(),
        }
    }

    /// Shorthand for creating an unsupported-capability error.
    pub fn unsupported<S: Into<String>>(capability: S) -> Self {
        Self::UnsupportedCapability {
            capability: capability.into(),
        }
    }

    /// Shorthand for creating an adapter error.
    pub fn adapter<S: Into<String>>(message: S) -> Self {
        Self::Adapter {
            message: message.into(),
            source_msg: None,
        }
    }

    /// Shorthand for creating an adapter error with a source.
    pub fn adapter_with_source<S1: Into<String>, S2: Into<String>>(
        message: S1,
        source: S2,
    ) -> Self {
        Self::Adapter {
            message: message.into(),
            source_msg: Some(source.into()),
        }
    }

    /// Shorthand for creating an internal error.
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Shorthand for creating an unauthorized error.
    pub fn unauthorized<S: Into<String>>(message: S) -> Self {
        Self::Unauthorized {
            message: message.into(),
        }
    }

    /// Shorthand for creating a bad-request error.
    pub fn bad_request<S: Into<String>>(message: S) -> Self {
        Self::BadRequest {
            message: message.into(),
        }
    }

    /// Shorthand for creating a conflict error.
    pub fn conflict<S: Into<String>>(message: S) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    /// Shorthand for creating a timeout error.
    pub fn timeout<S: Into<String>>(message: S) -> Self {
        Self::Timeout {
            message: message.into(),
        }
    }
}
