//! Embedder trait and built-in implementations.
//!
//! Re-exports the [`Embedder`] port trait from `lattice-runtime` and provides
//! a [`NoopEmbedder`] for tests.

use lattice_core::Error;

/// Re-export so callers can use `lattice_vector::Embedder` directly.
pub use lattice_runtime::ports::Embedder;

/// Embedder that returns a zero vector — useful for tests that exercise
/// indexing / retrieval without needing real embeddings.
pub struct NoopEmbedder {
    dimension: u32,
}

impl NoopEmbedder {
    /// Create a noop embedder with the given output dimension.
    pub fn new(dimension: u32) -> Self {
        Self { dimension }
    }
}

impl Embedder for NoopEmbedder {
    fn embed(&self, _text: &str) -> Result<Vec<f32>, Error> {
        Ok(vec![0.0_f32; self.dimension as usize])
    }
}
