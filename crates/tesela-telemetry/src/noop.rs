//! No-op metrics implementation.

use crate::traits::{Counter, Gauge, Histogram, MetricsRegistry};
use std::sync::Arc;

/// Zero-cost no-op counter.
pub struct NoopCounter;

impl Counter for NoopCounter {
    fn inc(&self, _n: u64) {}
}

/// Zero-cost no-op histogram.
pub struct NoopHistogram;

impl Histogram for NoopHistogram {
    fn record(&self, _value: f64) {}
}

/// Zero-cost no-op gauge.
pub struct NoopGauge;

impl Gauge for NoopGauge {
    fn set(&self, _value: f64) {}
}

/// No-op metrics registry.
pub struct NoopMetricsRegistry;

impl MetricsRegistry for NoopMetricsRegistry {
    fn counter(
        &self,
        _name: &str,
        _description: &str,
        _labels: &[(String, String)],
    ) -> Arc<dyn Counter> {
        Arc::new(NoopCounter)
    }
    fn histogram(
        &self,
        _name: &str,
        _description: &str,
        _labels: &[(String, String)],
    ) -> Arc<dyn Histogram> {
        Arc::new(NoopHistogram)
    }
    fn gauge(
        &self,
        _name: &str,
        _description: &str,
        _labels: &[(String, String)],
    ) -> Arc<dyn Gauge> {
        Arc::new(NoopGauge)
    }
}
