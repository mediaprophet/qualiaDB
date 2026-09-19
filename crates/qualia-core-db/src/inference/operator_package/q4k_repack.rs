//! Streaming Q4_K tensor repacker for companion operator packages (W2: EOS-022).
//!
//! Decomposes GGML Q4_K superblocks (144 bytes / 256 weights) into:
//! - Retained source payload (exact source tiles per frozen decision D4).
//! - Separated scale/min planes (16 bytes per block: d, dmin, 12-byte packed subscales).
//! - Independent bit-plane / nibble tiles (128 bytes per block: qs).
//!
//! Guarantees:
//! - Source-byte preservation contract: exact bit-for-bit round-trip reconstruction.
//! - Exact quantization metadata preserved (no lossy re-quantization).
//! - Deterministic representation digests.

use std::fmt;
use qualia_inference_kernel::operators::{Q4K_SUPERBLOCK_BYTES, Q4K_SUPERBLOCK_ELEMS};
pub const Q4K_SCALE_PLANE_BYTES_PER_BLOCK: usize = 16;
pub const Q4K_BITPLANE_BYTES_PER_BLOCK: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepackError {
    EmptyInput,
    ZeroElements,
    TruncatedSource { expected: usize, actual: usize },
    InvalidBufferLength { expected: usize, actual: usize },
    ReconstructionMismatch,
}

impl fmt::Display for RepackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "empty input data for Q4_K repack"),
            Self::ZeroElements => write!(f, "zero elements specified for Q4_K repack"),
            Self::TruncatedSource { expected, actual } => {
                write!(f, "truncated source: expected at least {expected} bytes, got {actual}")
            }
            Self::InvalidBufferLength { expected, actual } => {
                write!(f, "buffer length mismatch: expected {expected} bytes, got {actual}")
            }
            Self::ReconstructionMismatch => {
                write!(f, "reconstructed source bytes did not match original source")
            }
        }
    }
}

impl std::error::Error for RepackError {}

/// Container holding the repacked Q4_K components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepackedQ4KTensor {
    pub n_elems: usize,
    pub n_blocks: usize,
    /// Retained source tiles (exact original bytes).
    pub source_bytes: Vec<u8>,
    /// Scale/min plane: 16 bytes per block (d, dmin, scales).
    pub scale_plane: Vec<u8>,
    /// Bit-plane nibble tiles: 128 bytes per block (qs).
    pub bitplane_tiles: Vec<u8>,
    /// Source content digest.
    pub source_digest: u64,
    /// Representation digest of the repacked planes.
    pub representation_digest: u64,
}

/// Repack a GGML Q4_K tensor into independent scale and bit-plane streams.
pub fn repack_q4k_tensor(raw_source: &[u8], n_elems: usize) -> Result<RepackedQ4KTensor, RepackError> {
    if n_elems == 0 {
        return Err(RepackError::ZeroElements);
    }
    let n_blocks = n_elems.div_ceil(Q4K_SUPERBLOCK_ELEMS as usize);
    let expected_bytes = n_blocks * Q4K_SUPERBLOCK_BYTES;
    if raw_source.len() < expected_bytes {
        return Err(RepackError::TruncatedSource {
            expected: expected_bytes,
            actual: raw_source.len(),
        });
    }

    let source_slice = &raw_source[..expected_bytes];
    let mut scale_plane = Vec::with_capacity(n_blocks * Q4K_SCALE_PLANE_BYTES_PER_BLOCK);
    let mut bitplane_tiles = Vec::with_capacity(n_blocks * Q4K_BITPLANE_BYTES_PER_BLOCK);

    for b in 0..n_blocks {
        let block_start = b * Q4K_SUPERBLOCK_BYTES;
        let block = &source_slice[block_start..block_start + Q4K_SUPERBLOCK_BYTES];

        // 16-byte scale plane: d (2B), dmin (2B), packed 6-bit scales (12B)
        scale_plane.extend_from_slice(&block[0..16]);
        // 128-byte nibble bit-plane
        bitplane_tiles.extend_from_slice(&block[16..144]);
    }

    let source_digest = compute_fnv1a_64(source_slice);

    // Representation digest combines scale plane and bitplane digests
    let scale_hash = compute_fnv1a_64(&scale_plane);
    let bit_hash = compute_fnv1a_64(&bitplane_tiles);
    let representation_digest = scale_hash ^ bit_hash.rotate_left(17);

    Ok(RepackedQ4KTensor {
        n_elems,
        n_blocks,
        source_bytes: source_slice.to_vec(),
        scale_plane,
        bitplane_tiles,
        source_digest,
        representation_digest,
    })
}

/// Reconstruct the exact source GGML Q4_K superblocks from separated scale and bitplane streams.
pub fn reconstruct_source_q4k(
    scale_plane: &[u8],
    bitplane_tiles: &[u8],
    n_blocks: usize,
    out_source: &mut [u8],
) -> Result<(), RepackError> {
    let expected_scale_len = n_blocks * Q4K_SCALE_PLANE_BYTES_PER_BLOCK;
    let expected_bit_len = n_blocks * Q4K_BITPLANE_BYTES_PER_BLOCK;
    let expected_out_len = n_blocks * Q4K_SUPERBLOCK_BYTES;

    if scale_plane.len() < expected_scale_len {
        return Err(RepackError::InvalidBufferLength {
            expected: expected_scale_len,
            actual: scale_plane.len(),
        });
    }
    if bitplane_tiles.len() < expected_bit_len {
        return Err(RepackError::InvalidBufferLength {
            expected: expected_bit_len,
            actual: bitplane_tiles.len(),
        });
    }
    if out_source.len() < expected_out_len {
        return Err(RepackError::InvalidBufferLength {
            expected: expected_out_len,
            actual: out_source.len(),
        });
    }

    for b in 0..n_blocks {
        let sc_start = b * Q4K_SCALE_PLANE_BYTES_PER_BLOCK;
        let bp_start = b * Q4K_BITPLANE_BYTES_PER_BLOCK;
        let out_start = b * Q4K_SUPERBLOCK_BYTES;

        out_source[out_start..out_start + 16]
            .copy_from_slice(&scale_plane[sc_start..sc_start + 16]);
        out_source[out_start + 16..out_start + 144]
            .copy_from_slice(&bitplane_tiles[bp_start..bp_start + 128]);
    }

    Ok(())
}

/// Verify exact source-byte preservation round-trip for a Q4_K tensor.
pub fn verify_q4k_repack_roundtrip(raw_source: &[u8], n_elems: usize) -> Result<(), RepackError> {
    let repacked = repack_q4k_tensor(raw_source, n_elems)?;
    let mut reconstructed = vec![0u8; repacked.n_blocks * Q4K_SUPERBLOCK_BYTES];
    reconstruct_source_q4k(
        &repacked.scale_plane,
        &repacked.bitplane_tiles,
        repacked.n_blocks,
        &mut reconstructed,
    )?;

    if reconstructed != repacked.source_bytes {
        return Err(RepackError::ReconstructionMismatch);
    }
    Ok(())
}

/// 64-bit FNV-1a hash.
fn compute_fnv1a_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_synthetic_q4k_block(block_idx: u8) -> [u8; Q4K_SUPERBLOCK_BYTES] {
        let mut blk = [0u8; Q4K_SUPERBLOCK_BYTES];
        blk[0] = 0x00; // d = 1.0 in f16
        blk[1] = 0x3c;
        blk[2] = 0x00; // dmin = 0.1 in f16
        blk[3] = 0x2e;
        // 12 bytes scales
        for i in 4..16 {
            blk[i] = (block_idx + i as u8) & 0x3f;
        }
        // 128 bytes qs nibbles
        for i in 16..144 {
            blk[i] = block_idx.wrapping_add(i as u8);
        }
        blk
    }

    #[test]
    fn repack_roundtrip_bit_exact() {
        let mut raw = Vec::new();
        for b in 0..4 {
            raw.extend_from_slice(&make_synthetic_q4k_block(b));
        }

        assert!(verify_q4k_repack_roundtrip(&raw, 4 * 256).is_ok());
    }

    #[test]
    fn partial_tile_n_elems_rounds_up_blocks() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&make_synthetic_q4k_block(1));
        raw.extend_from_slice(&make_synthetic_q4k_block(2));

        // 300 elems needs 2 blocks (512 capacity)
        let repacked = repack_q4k_tensor(&raw, 300).expect("repack should succeed");
        assert_eq!(repacked.n_blocks, 2);
        assert_eq!(repacked.scale_plane.len(), 2 * 16);
        assert_eq!(repacked.bitplane_tiles.len(), 2 * 128);
    }

    #[test]
    fn truncated_source_fails_closed() {
        let raw = vec![0u8; 100]; // less than 144 bytes
        assert!(matches!(
            repack_q4k_tensor(&raw, 256),
            Err(RepackError::TruncatedSource { .. })
        ));
    }
}
