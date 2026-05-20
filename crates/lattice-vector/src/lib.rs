//! Vector / semantic-search backend for Lattice.
//!
//! Provides:
//! - [`MemoryVectorBackend`] — brute-force cosine k-NN over an in-memory store
//! - [`Embedder`] / [`NoopEmbedder`] — text-to-vector conversion
//! - [`VectorAgentMemoryStore`] — semantic agent memory backed by a vector index

#![deny(warnings)]
#![deny(missing_docs)]

pub mod agent_memory;
pub mod embedder;
pub mod memory;

pub use agent_memory::VectorAgentMemoryStore;
pub use embedder::{Embedder, NoopEmbedder};
pub use memory::MemoryVectorBackend;
