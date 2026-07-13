//! `.10d` Spatial-index section — BVH node array + kd-tree node array
//! serialized as a self-describing section (P3.7).
//!
//! Layout:
//! ```text
//! [SpatialIndexMiniHeader: 32 bytes]
//! [bvh_nodes:  B * 40 bytes  (BvhNode)]
//! [bvh_prim_indices: B * 4 bytes]
//! [kd_nodes:   K * 32 bytes  (KdNode)]
//! [kd_point_indices: K * 4 bytes]
//! ```
//!
//! All arrays are little-endian, 4-byte aligned. The section is deterministic:
//! identical input yields byte-identical output. Per-section CRC-32C is
//! handled by the container's section-table encoder.

use bytemuck::{bytes_of, cast_slice, from_bytes, Pod, Zeroable};

use crate::specialized_libs::computational_geometry::{
    BvhNode, KdNode, BVH_NODE_SIZE, KD_NODE_SIZE,
};

/// Mini-header size in bytes.
pub const SPATIAL_INDEX_MINI_HEADER_SIZE: usize = 32;

/// Maximum node count (bounded by 42MB Sentinel).
pub const MAX_BVH_NODE_COUNT: usize = 1_048_576; // 2^20
pub const MAX_KD_NODE_COUNT: usize = 1_048_576; // 2^20

/// The 32-byte SpatialIndex-section mini-header.
///
/// ```text
/// offset  size  field
/// 0       4     bvh_node_count:u32
/// 4       4     kd_node_count:u32
/// 8       4     bvh_root:u32
/// 12      4     kd_root:u32
/// 16      4     bvh_prim_count:u32   (number of BVH primitive indices, = number of AABBs)
/// 20      4     kd_point_count:u32  (number of kd-tree point indices, = number of points)
/// 24      4     reserved_u32        (must be zero)
/// 28      4     reserved_u32_2      (must be zero)
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SpatialIndexMiniHeader {
    pub bvh_node_count: u32,
    pub kd_node_count: u32,
    pub bvh_root: u32,
    pub kd_root: u32,
    pub bvh_prim_count: u32,
    pub kd_point_count: u32,
    pub reserved_u32: u32,
    pub reserved_u32_2: u32,
}

impl Default for SpatialIndexMiniHeader {
    fn default() -> Self {
        Self {
            bvh_node_count: 0,
            kd_node_count: 0,
            bvh_root: 0,
            kd_root: 0,
            bvh_prim_count: 0,
            kd_point_count: 0,
            reserved_u32: 0,
            reserved_u32_2: 0,
        }
    }
}

/// Spatial-index-section encode/decode error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpatialIndexSectionError {
    /// Payload too short for the mini-header.
    PayloadTooShort { got: usize, need: usize },
    /// Non-zero reserved field.
    NonZeroReserved,
    /// Node count exceeds the max.
    NodeCountTooLarge { got: u32, max: usize },
    /// Payload truncated relative to declared counts.
    PayloadTruncated { expected: usize, got: usize },
    /// Output buffer too small.
    OutputBufferTooSmall { needed: usize, have: usize },
}

impl std::fmt::Display for SpatialIndexSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooShort { got, need } => write!(
                f,
                "10d SPATIAL_INDEX payload too short: got {got}, need {need}"
            ),
            Self::NonZeroReserved => write!(f, "10d SPATIAL_INDEX non-zero reserved field"),
            Self::NodeCountTooLarge { got, max } => {
                write!(f, "10d SPATIAL_INDEX node_count {got} exceeds max {max}")
            }
            Self::PayloadTruncated { expected, got } => write!(
                f,
                "10d SPATIAL_INDEX payload truncated: expected {expected}, got {got}"
            ),
            Self::OutputBufferTooSmall { needed, have } => write!(
                f,
                "10d SPATIAL_INDEX output buffer too small: need {needed}, have {have}"
            ),
        }
    }
}

impl std::error::Error for SpatialIndexSectionError {}

/// Compute the encoded byte length for a spatial-index section.
#[inline]
pub fn encoded_len(
    bvh_node_count: u32,
    kd_node_count: u32,
    bvh_prim_count: u32,
    kd_point_count: u32,
) -> usize {
    let bc = bvh_node_count as usize;
    let kc = kd_node_count as usize;
    let pc = bvh_prim_count as usize;
    let qc = kd_point_count as usize;
    SPATIAL_INDEX_MINI_HEADER_SIZE
        + bc * BVH_NODE_SIZE       // bvh_nodes
        + pc * 4                   // bvh_prim_indices
        + kc * KD_NODE_SIZE        // kd_nodes
        + qc * 4 // kd_point_indices
}

/// Encode a spatial-index section from BVH and kd-tree node arrays.
///
/// Returns the number of bytes written.
pub fn encode_spatial_index_section(
    bvh_nodes: &[BvhNode],
    bvh_prim_indices: &[u32],
    bvh_root: u32,
    kd_nodes: &[KdNode],
    kd_point_indices: &[u32],
    kd_root: u32,
    out: &mut [u8],
) -> Result<usize, SpatialIndexSectionError> {
    let bc = bvh_nodes.len();
    let kc = kd_nodes.len();
    let pc = bvh_prim_indices.len();
    let qc = kd_point_indices.len();

    if bc > MAX_BVH_NODE_COUNT {
        return Err(SpatialIndexSectionError::NodeCountTooLarge {
            got: bc as u32,
            max: MAX_BVH_NODE_COUNT,
        });
    }
    if kc > MAX_KD_NODE_COUNT {
        return Err(SpatialIndexSectionError::NodeCountTooLarge {
            got: kc as u32,
            max: MAX_KD_NODE_COUNT,
        });
    }
    if bvh_prim_indices.len() < pc {
        return Err(SpatialIndexSectionError::PayloadTruncated {
            expected: pc,
            got: bvh_prim_indices.len(),
        });
    }
    if kd_point_indices.len() < qc {
        return Err(SpatialIndexSectionError::PayloadTruncated {
            expected: qc,
            got: kd_point_indices.len(),
        });
    }

    let need = encoded_len(bc as u32, kc as u32, pc as u32, qc as u32);
    if out.len() < need {
        return Err(SpatialIndexSectionError::OutputBufferTooSmall {
            needed: need,
            have: out.len(),
        });
    }

    let header = SpatialIndexMiniHeader {
        bvh_node_count: bc as u32,
        kd_node_count: kc as u32,
        bvh_root,
        kd_root,
        bvh_prim_count: pc as u32,
        kd_point_count: qc as u32,
        reserved_u32: 0,
        reserved_u32_2: 0,
    };

    let mut off = 0usize;
    out[off..off + SPATIAL_INDEX_MINI_HEADER_SIZE].copy_from_slice(bytes_of(&header));
    off += SPATIAL_INDEX_MINI_HEADER_SIZE;

    // BVH nodes.
    let bvh_bytes = cast_slice(bvh_nodes);
    out[off..off + bvh_bytes.len()].copy_from_slice(bvh_bytes);
    off += bc * BVH_NODE_SIZE;

    // BVH prim indices.
    let bvh_idx_bytes = cast_slice(&bvh_prim_indices[..pc]);
    out[off..off + bvh_idx_bytes.len()].copy_from_slice(bvh_idx_bytes);
    off += pc * 4;

    // kd-tree nodes.
    let kd_bytes = cast_slice(kd_nodes);
    out[off..off + kd_bytes.len()].copy_from_slice(kd_bytes);
    off += kc * KD_NODE_SIZE;

    // kd-tree point indices.
    let kd_idx_bytes = cast_slice(&kd_point_indices[..qc]);
    out[off..off + kd_idx_bytes.len()].copy_from_slice(kd_idx_bytes);
    off += qc * 4;

    debug_assert_eq!(off, need);
    Ok(off)
}

/// Decoded spatial-index section.
#[derive(Debug)]
pub struct DecodedSpatialIndex<'a> {
    pub header: SpatialIndexMiniHeader,
    pub bvh_nodes: &'a [BvhNode],
    pub bvh_prim_indices: &'a [u32],
    pub kd_nodes: &'a [KdNode],
    pub kd_point_indices: &'a [u32],
}

/// Decode a spatial-index section from a raw payload.
pub fn decode_spatial_index_section(
    payload: &[u8],
) -> Result<DecodedSpatialIndex<'_>, SpatialIndexSectionError> {
    if payload.len() < SPATIAL_INDEX_MINI_HEADER_SIZE {
        return Err(SpatialIndexSectionError::PayloadTooShort {
            got: payload.len(),
            need: SPATIAL_INDEX_MINI_HEADER_SIZE,
        });
    }

    let header: SpatialIndexMiniHeader = *from_bytes(&payload[..SPATIAL_INDEX_MINI_HEADER_SIZE]);

    if header.reserved_u32 != 0 || header.reserved_u32_2 != 0 {
        return Err(SpatialIndexSectionError::NonZeroReserved);
    }

    let bc = header.bvh_node_count as usize;
    let kc = header.kd_node_count as usize;
    let pc = header.bvh_prim_count as usize;
    let qc = header.kd_point_count as usize;

    if bc > MAX_BVH_NODE_COUNT {
        return Err(SpatialIndexSectionError::NodeCountTooLarge {
            got: header.bvh_node_count,
            max: MAX_BVH_NODE_COUNT,
        });
    }
    if kc > MAX_KD_NODE_COUNT {
        return Err(SpatialIndexSectionError::NodeCountTooLarge {
            got: header.kd_node_count,
            max: MAX_KD_NODE_COUNT,
        });
    }

    let need = encoded_len(
        header.bvh_node_count,
        header.kd_node_count,
        header.bvh_prim_count,
        header.kd_point_count,
    );
    if payload.len() < need {
        return Err(SpatialIndexSectionError::PayloadTruncated {
            expected: need,
            got: payload.len(),
        });
    }

    let mut off = SPATIAL_INDEX_MINI_HEADER_SIZE;

    let bvh_nodes: &[BvhNode] = cast_slice(&payload[off..off + bc * BVH_NODE_SIZE]);
    off += bc * BVH_NODE_SIZE;

    let bvh_prim_indices: &[u32] = cast_slice(&payload[off..off + pc * 4]);
    off += pc * 4;

    let kd_nodes: &[KdNode] = cast_slice(&payload[off..off + kc * KD_NODE_SIZE]);
    off += kc * KD_NODE_SIZE;

    let kd_point_indices: &[u32] = cast_slice(&payload[off..off + qc * 4]);
    off += qc * 4;

    debug_assert_eq!(off, need);

    Ok(DecodedSpatialIndex {
        header,
        bvh_nodes,
        bvh_prim_indices,
        kd_nodes,
        kd_point_indices,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::{
        build_bvh_recursive, build_kd_tree_3d, Aabb, Point3,
    };

    fn test_aabbs() -> Vec<Aabb> {
        (0..8)
            .map(|i| {
                let x = (i % 2) as f64;
                let y = ((i / 2) % 2) as f64;
                let z = (i / 4) as f64;
                Aabb::new(Point3::new(x, y, z), Point3::new(x + 1.0, y + 1.0, z + 1.0))
            })
            .collect()
    }

    fn test_points() -> Vec<[f64; 3]> {
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
        ]
    }

    #[test]
    fn round_trip_encode_decode() {
        let aabbs = test_aabbs();
        let n = aabbs.len();
        let mut bvh_nodes = vec![BvhNode::default(); 2 * n];
        let mut bvh_indices = vec![0u32; n];
        let mut bvh_codes = vec![0u64; n];
        let mut bvh_sort = vec![0u32; n];
        let (bvh_count, bvh_root) = build_bvh_recursive(
            &aabbs,
            &mut bvh_nodes,
            &mut bvh_indices,
            &mut bvh_codes,
            &mut bvh_sort,
        )
        .unwrap();

        let points = test_points();
        let np = points.len();
        let mut kd_nodes = vec![KdNode::default(); np];
        let mut kd_indices = vec![0u32; np];
        let mut kd_codes = vec![0u64; np];
        let mut kd_sort = vec![0u32; np];
        let (kd_count, kd_root) = build_kd_tree_3d(
            &points,
            &mut kd_nodes,
            &mut kd_indices,
            &mut kd_codes,
            &mut kd_sort,
        )
        .unwrap();

        let need = encoded_len(bvh_count as u32, kd_count as u32, n as u32, np as u32);
        let mut buf = vec![0u8; need];

        let written = encode_spatial_index_section(
            &bvh_nodes[..bvh_count],
            &bvh_indices,
            bvh_root as u32,
            &kd_nodes[..kd_count],
            &kd_indices,
            kd_root as u32,
            &mut buf,
        )
        .unwrap();
        assert_eq!(written, need);

        let decoded = decode_spatial_index_section(&buf).unwrap();
        assert_eq!(decoded.header.bvh_node_count, bvh_count as u32);
        assert_eq!(decoded.header.kd_node_count, kd_count as u32);
        assert_eq!(decoded.header.bvh_root, bvh_root as u32);
        assert_eq!(decoded.header.kd_root, kd_root as u32);
        assert_eq!(decoded.header.bvh_prim_count, n as u32);
        assert_eq!(decoded.header.kd_point_count, np as u32);
        assert_eq!(decoded.bvh_nodes.len(), bvh_count);
        assert_eq!(decoded.kd_nodes.len(), kd_count);
        assert_eq!(decoded.bvh_nodes, &bvh_nodes[..bvh_count]);
        assert_eq!(decoded.kd_nodes, &kd_nodes[..kd_count]);
        assert_eq!(decoded.bvh_prim_indices, &bvh_indices);
        assert_eq!(decoded.kd_point_indices, &kd_indices);
    }

    #[test]
    fn encode_twice_is_byte_identical() {
        let aabbs = test_aabbs();
        let n = aabbs.len();
        let mut bvh_nodes = vec![BvhNode::default(); 2 * n];
        let mut bvh_indices = vec![0u32; n];
        let mut bvh_codes = vec![0u64; n];
        let mut bvh_sort = vec![0u32; n];
        let (bvh_count, bvh_root) = build_bvh_recursive(
            &aabbs,
            &mut bvh_nodes,
            &mut bvh_indices,
            &mut bvh_codes,
            &mut bvh_sort,
        )
        .unwrap();

        let need = encoded_len(bvh_count as u32, 0, n as u32, 0);
        let mut buf_a = vec![0u8; need];
        let mut buf_b = vec![0u8; need];

        encode_spatial_index_section(
            &bvh_nodes[..bvh_count],
            &bvh_indices,
            bvh_root as u32,
            &[],
            &[],
            0,
            &mut buf_a,
        )
        .unwrap();
        encode_spatial_index_section(
            &bvh_nodes[..bvh_count],
            &bvh_indices,
            bvh_root as u32,
            &[],
            &[],
            0,
            &mut buf_b,
        )
        .unwrap();

        assert_eq!(buf_a, buf_b);
    }

    #[test]
    fn decode_rejects_nonzero_reserved() {
        let mut buf = vec![0u8; SPATIAL_INDEX_MINI_HEADER_SIZE];
        // Set reserved field (offset 28) to non-zero.
        buf[28] = 1;
        let result = decode_spatial_index_section(&buf);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            SpatialIndexSectionError::NonZeroReserved
        );
    }

    #[test]
    fn decode_rejects_truncated_payload() {
        let buf = vec![0u8; SPATIAL_INDEX_MINI_HEADER_SIZE - 1];
        let result = decode_spatial_index_section(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn decode_rejects_node_count_too_large() {
        let mut buf = vec![0u8; SPATIAL_INDEX_MINI_HEADER_SIZE];
        // Set bvh_node_count to a huge value.
        let header = SpatialIndexMiniHeader {
            bvh_node_count: MAX_BVH_NODE_COUNT as u32 + 1,
            kd_node_count: 0,
            bvh_root: 0,
            kd_root: 0,
            bvh_prim_count: 0,
            kd_point_count: 0,
            reserved_u32: 0,
            reserved_u32_2: 0,
        };
        buf.copy_from_slice(bytes_of(&header));
        let result = decode_spatial_index_section(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn empty_section_round_trip() {
        let need = encoded_len(0, 0, 0, 0);
        let mut buf = vec![0u8; need];
        encode_spatial_index_section(&[], &[], 0, &[], &[], 0, &mut buf).unwrap();

        let decoded = decode_spatial_index_section(&buf).unwrap();
        assert_eq!(decoded.header.bvh_node_count, 0);
        assert_eq!(decoded.header.kd_node_count, 0);
        assert!(decoded.bvh_nodes.is_empty());
        assert!(decoded.kd_nodes.is_empty());
    }

    #[test]
    fn header_is_pod() {
        assert_eq!(
            std::mem::size_of::<SpatialIndexMiniHeader>(),
            SPATIAL_INDEX_MINI_HEADER_SIZE
        );
    }
}
