//! Metrics telemetry port for Tesela.
//!
//! Provides trait abstractions for counters, histograms, and gauges.
//! The `noop` module contains a zero-cost no-op implementation.

pub mod noop;
pub mod traits;

pub use noop::*;
pub use traits::*;
