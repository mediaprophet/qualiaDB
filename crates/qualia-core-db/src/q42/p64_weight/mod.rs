//! Phase 4: AOT GGUF → `.p64` **LLM-weight container** compiler.
//!
//! A *sibling* of the semantic `.q42` graph format — it carries an **independent section magic** `b"p64\0"`
//! so the two never collide. Per the architectural decision, the weights are stored as **opaque,
//! cache-aligned (64-byte), contiguous quantized blobs**.
//!
//! The `NQuin` epistemic scaffold is removed from this probabilistic container. The 48-byte
//! declarative `q42` system manages truth, while this 64-byte aligned `p64` system manages
//! pure mathematical inference with zero-copy relative WASM pointers.
//!
//! Output is **little-endian** on every host via explicit serialization.
//!
//! Layout:
//! ```text
//! [ P64WeightHeader (64B) ]              magic, version, flags, 32-bit relative offsets
//! [ P64TensorEntry[] (64B each) ]        role, dtype, rank, dims, relative blob offsets
//! [ pad → 1<<page_log2 ]
//! [ tensor blob region ]                 quantized bytes; each tensor START page-aligned
//!                                        (default 16KB) for single-fetch mmap.
//! ```

mod compiler;
mod etalon_adapter;
mod layout;
#[cfg(test)]
mod migration_contract_tests;
mod reader;
#[cfg(test)]
mod tests;
mod transcode;
// Historical tests for the pre-P64 Q42W layout are retained as migration
// documentation only. They refer to the removed 144/80-byte API.
#[cfg(all(test, any()))]
mod legacy_tests;

pub use compiler::*;
pub use layout::*;
pub use reader::*;
pub use transcode::*;

// Shared, crate-internal helpers used across the submodules above. They live in the parent
// module so every child (compiler / transcode / reader / tests) can reach them unchanged,
// while delegating neutral `.p64` layout & serialization mechanics to `etalon_model::p64`.

#[inline]
#[allow(dead_code)]
fn write_manifold_coordinate(
    coordinate: &crate::modalities::manifold::ManifoldCoordinate10D,
    out: &mut [u8],
) {
    etalon_model::p64::P64ManifoldRecord::write_values_le(&coordinate.as_f32_array(), out);
}

#[inline]
#[allow(dead_code)]
fn write_tensor_entry(entry: &P64TensorEntry, out: &mut [u8]) {
    entry.write_le(out);
}
