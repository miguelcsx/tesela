//! C ABI bridge for the Tesela runtime.
//!
//! Exposes a stable FFI surface for Python and other native SDK consumers.
//!
//! # Memory contract
//!
//! * Strings returned as `*mut c_char` are heap-allocated C strings.  Callers
//!   **must** free them with `tesela_string_free`.
//! * Binary blobs are returned as [`TeselaBuffer`].  Callers **must** free
//!   them with `tesela_buffer_free`.
//! * Strings passed *in* to exported functions are borrowed for the duration of
//!   the call only; the caller retains ownership.
//!
//! # Thread safety
//!
//! All exported functions are thread-safe.  Internally a global
//! `Mutex<HashMap>` maps opaque `u64` handles to `Arc<Runtime>` instances.

#![allow(clippy::missing_safety_doc)]

pub mod handle;

mod advanced_ops;
mod callback_backend;
mod data_ops;
mod lifecycle;
mod subscribe;
mod system;

pub use handle::{CallbackFn, TeselaBuffer};
