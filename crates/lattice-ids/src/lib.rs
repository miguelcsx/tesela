//! Time-ordered unique ID generation for Lattice.
//!
//! Provides two generators:
//! - [`new_ulid`] — 26-char Crockford-base32 ULID, monotonic within a millisecond.
//! - [`new_uuid_v7`] — UUID v7 (time-ordered, RFC 9562).
//!
//! Both are safe for concurrent use and cryptographically random in the
//! entropy portion.

#![deny(warnings)]
#![deny(missing_docs)]

use std::sync::Mutex;
use ulid::{Generator, Ulid};

// ---------------------------------------------------------------------------
// ULID — monotonic, Crockford-base32, 26 characters
// ---------------------------------------------------------------------------

/// Monotonic ULID generator shared across threads.
static ULID_GEN: Mutex<Generator> = Mutex::new(Generator::new());

/// Mint a fresh ULID.
///
/// The 48-bit timestamp prefix is the current Unix millisecond; the 80-bit
/// entropy suffix is drawn from the OS CSPRNG via [`ulid`].  Calls within
/// the same millisecond produce strictly increasing values (monotonic mode).
pub fn new_ulid() -> String {
    let mut generator = ULID_GEN.lock().unwrap_or_else(|e| e.into_inner());
    generator
        .generate()
        .unwrap_or_else(|_| Ulid::new())
        .to_string()
}

// ---------------------------------------------------------------------------
// UUID v7 — time-ordered, RFC 9562
// ---------------------------------------------------------------------------

/// Mint a UUID v7.
///
/// UUID v7 embeds a Unix millisecond timestamp in the most-significant bits,
/// making it naturally sortable by creation time — a useful property for
/// database primary keys and distributed tracing correlation IDs.
pub fn new_uuid_v7() -> String {
    uuid::Uuid::now_v7().to_string()
}

// ---------------------------------------------------------------------------
// UlidIdGenerator — implements lattice_runtime::ports::IdGenerator
// ---------------------------------------------------------------------------

/// [`IdGenerator`] implementation backed by ULID.
///
/// The optional `prefix` passed to [`UlidIdGenerator::new_id`] is prepended
/// followed by `_`, e.g. `run_01HXYZ…`.
///
/// [`IdGenerator`]: lattice_runtime::ports::IdGenerator
pub struct UlidIdGenerator;

impl UlidIdGenerator {
    /// Create a new generator (stateless; safe to share across threads).
    pub fn new() -> Self {
        Self
    }
}

impl Default for UlidIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// Implement the lattice_runtime IdGenerator port directly so callers can
// plug this in without any glue.
impl lattice_runtime::ports::IdGenerator for UlidIdGenerator {
    fn new_id(&self, prefix: &str) -> String {
        let id = new_ulid();
        if prefix.is_empty() {
            id
        } else {
            format!("{}_{}", prefix, id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulid_is_26_chars() {
        let id = new_ulid();
        assert_eq!(id.len(), 26, "ULID must be 26 chars, got: {}", id);
    }

    #[test]
    fn ulids_are_ordered() {
        let a = new_ulid();
        let b = new_ulid();
        assert!(b >= a, "ULIDs must be non-decreasing");
    }

    #[test]
    fn uuid_v7_is_valid() {
        let id = new_uuid_v7();
        assert!(
            uuid::Uuid::parse_str(&id).is_ok(),
            "UUID v7 must parse: {}",
            id
        );
    }

    #[test]
    fn ulid_generator_prefixes() {
        let id_gen = UlidIdGenerator::new();
        use lattice_runtime::ports::IdGenerator;
        let id = id_gen.new_id("run");
        assert!(id.starts_with("run_"), "expected prefix: {}", id);

        let bare = id_gen.new_id("");
        assert!(
            !bare.contains('_'),
            "no prefix should mean no underscore: {}",
            bare
        );
    }
}
