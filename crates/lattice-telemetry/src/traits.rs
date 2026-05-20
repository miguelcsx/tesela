//! Metrics port traits.

use std::sync::Arc;

/// A monotonically-increasing counter.
pub trait Counter: Send + Sync {
    /// Increment the counter by `n`.
    fn inc(&self, n: u64);
}

/// A duration / value histogram.
pub trait Histogram: Send + Sync {
    /// Record a value (e.g. milliseconds).
    fn record(&self, value: f64);
}

/// A gauge that can go up or down.
pub trait Gauge: Send + Sync {
    /// Set the gauge to `value`.
    fn set(&self, value: f64);
}

/// Factory for metrics instruments.
pub trait MetricsRegistry: Send + Sync {
    /// Get or create a counter.
    fn counter(
        &self,
        name: &str,
        description: &str,
        labels: &[(String, String)],
    ) -> Arc<dyn Counter>;
    /// Get or create a histogram.
    fn histogram(
        &self,
        name: &str,
        description: &str,
        labels: &[(String, String)],
    ) -> Arc<dyn Histogram>;
    /// Get or create a gauge.
    fn gauge(&self, name: &str, description: &str, labels: &[(String, String)]) -> Arc<dyn Gauge>;
}
