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
mod layout;
mod reader;
mod transcode;
#[cfg(test)]
mod tests;
// Historical tests for the pre-P64 Q42W layout are retained as migration
// documentation only. They refer to the removed 144/80-byte API.
#[cfg(all(test, any()))]
mod legacy_tests;

pub use compiler::*;
pub use layout::*;
pub use reader::*;
pub use transcode::*;

// Shared, crate-internal helpers used across the submodules above. They live in the parent
// module so every child (compiler / transcode / reader / tests) can reach them unchanged.

#[inline]
fn align_up(x: usize, a: usize) -> usize {
    debug_assert!(a.is_power_of_two());
    (x + a - 1) & !(a - 1)
}

fn write_manifold_coordinate(
    coordinate: &crate::modalities::manifold::ManifoldCoordinate10D,
    out: &mut [u8],
) {
    assert!(out.len() >= P64_MANIFOLD_ENTRY_BYTES);
    out[..P64_MANIFOLD_ENTRY_BYTES].fill(0);
    for (index, value) in coordinate.as_f32_array().iter().enumerate() {
        let start = index * 4;
        out[start..start + 4].copy_from_slice(&value.to_le_bytes());
    }
}

fn write_tensor_entry(entry: &P64TensorEntry, out: &mut [u8]) {
    assert!(out.len() >= P64_TENSOR_ENTRY_BYTES);
    out[..P64_TENSOR_ENTRY_BYTES].fill(0);
    out[0..4].copy_from_slice(&entry.name_offset.to_le_bytes());
    out[4..6].copy_from_slice(&entry.role_id.to_le_bytes());
    out[6..8].copy_from_slice(&entry.dtype.to_le_bytes());
    out[8..12].copy_from_slice(&entry.manifold_idx.to_le_bytes());
    out[12..16].copy_from_slice(&entry.rank.to_le_bytes());
    for (index, dimension) in entry.dimensions.iter().enumerate() {
        let start = 16 + index * 4;
        out[start..start + 4].copy_from_slice(&dimension.to_le_bytes());
    }
    out[32..36].copy_from_slice(&entry.blob_offset.to_le_bytes());
    out[36..40].copy_from_slice(&entry.blob_size.to_le_bytes());
    out[40..48].copy_from_slice(&entry.source_offset.to_le_bytes());
    out[48..56].copy_from_slice(&entry.source_name_hash.to_le_bytes());
    out[56..58].copy_from_slice(&entry.alt_dtype.to_le_bytes());
    out[58..60].copy_from_slice(&entry.precision_views_mask.to_le_bytes());
    out[60..64].copy_from_slice(&entry.alt_blob_offset.to_le_bytes());
}

/// GGUF tensor-name suffix for a per-layer P64 role (None for global tensors, named directly).
fn p64_role_suffix(role_id: u16) -> Option<&'static [u8]> {
    match role_id {
        P64_ROLE_ATTN_K => Some(b"attn_k.weight"),
        P64_ROLE_ATTN_V => Some(b"attn_v.weight"),
        P64_ROLE_ATTN_Q => Some(b"attn_q.weight"),
        P64_ROLE_ATTN_OUTPUT => Some(b"attn_output.weight"),
        P64_ROLE_FFN_GATE => Some(b"ffn_gate.weight"),
        P64_ROLE_FFN_UP => Some(b"ffn_up.weight"),
        P64_ROLE_FFN_DOWN => Some(b"ffn_down.weight"),
        P64_ROLE_ATTN_NORM => Some(b"attn_norm.weight"),
        P64_ROLE_FFN_NORM => Some(b"ffn_norm.weight"),
        _ => None,
    }
}
