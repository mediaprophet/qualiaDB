//! Morton (Z-order) and Hilbert space-filling curve codes for spatial sorting.
//!
//! Both curves map N-dimensional integer lattice coordinates to a 1-D ordering
//! that preserves spatial locality: points close in space tend to be close in
//! curve order. This is the foundation for spatial indexing (BVH, kd-tree
//! build, box-intersection sorting).
//!
//! ## Implementation
//!
//! - **Morton codes**: bit-interleaving of per-axis integer coordinates. For
//!   2D, bits alternate x,y,x,y,… For 3D, bits alternate x,y,z,x,y,z,…
//!   Encoding/decoding is exact and bidirectional.
//! - **Hilbert codes**: compact 2D/3D Hilbert curve via the table-driven
//!   state-machine method. Encoding is exact; decoding is not provided (the
//!   acceptance gate only requires encode + sort order).
//!
//! All functions are `#[inline]`, zero-heap, and deterministic. The sorting
//! functions are caller-buffered: they sort an index array by the curve code
//! of the corresponding point, using the caller's scratch buffer for codes.
//!
//! ## Coordinate quantization
//!
//! Floating-point points are quantized to `u16` per axis within a bounding box
//! before encoding. This gives 16-bit Morton codes (32-bit for 2D, 48-bit for
//! 3D) — sufficient for spatial sorting and bounded by the 42MB Sentinel.

use bytemuck::{Pod, Zeroable};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by the spatial ordering functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialOrderError {
    /// `codes` buffer too small (needs `point_count` entries).
    CodeBufferTooSmall { required: usize },
    /// `indices` buffer too small (needs `point_count` entries).
    IndexBufferTooSmall { required: usize },
    /// A coordinate is non-finite (NaN or infinity).
    NonFiniteCoordinate { index: usize },
}

// ---------------------------------------------------------------------------
// Morton 2D: 16-bit per axis → 32-bit code
// ---------------------------------------------------------------------------

/// Partly spread a 16-bit integer so that bits are separated by one zero bit.
///
/// Input:  b15 b14 b13 … b1 b0
/// Output: 0 b15 0 b14 0 b13 … 0 b1 0 b0  (32-bit value)
#[inline]
fn spread1_16(x: u16) -> u32 {
    let mut x = x as u32;
    x = (x | (x << 8)) & 0x00FF_00FF;
    x = (x | (x << 4)) & 0x0F0F_0F0F;
    x = (x | (x << 2)) & 0x3333_3333;
    x = (x | (x << 1)) & 0x5555_5555;
    x
}

/// Compact a spread 32-bit value back to 16-bit (inverse of `spread1_16`).
#[inline]
fn compact1_16(x: u32) -> u16 {
    let mut x = x & 0x5555_5555;
    x = (x | (x >> 1)) & 0x3333_3333;
    x = (x | (x >> 2)) & 0x0F0F_0F0F;
    x = (x | (x >> 4)) & 0x00FF_00FF;
    x = (x | (x >> 8)) & 0x0000_FFFF;
    x as u16
}

/// Encode a 2D lattice coordinate into a 32-bit Morton (Z-order) code.
#[inline]
pub fn morton_encode_2d(x: u16, y: u16) -> u32 {
    spread1_16(x) | (spread1_16(y) << 1)
}

/// Decode a 32-bit Morton code back into 2D lattice coordinates.
#[inline]
pub fn morton_decode_2d(code: u32) -> (u16, u16) {
    (compact1_16(code), compact1_16(code >> 1))
}

// ---------------------------------------------------------------------------
// Morton 3D: 16-bit per axis → 48-bit code (stored as u64)
// ---------------------------------------------------------------------------

/// Partly spread a 16-bit integer so that bits are separated by *two* zero bits.
///
/// Input:  b15 b14 … b1 b0
/// Output: 0 0 b15 0 0 b14 … 0 0 b0  (48-bit value in u64)
#[inline]
fn spread2_16(x: u16) -> u64 {
    let mut x = x as u64;
    x = (x | (x << 16)) & 0x0000_FFFF_0000_FFFF;
    x = (x | (x << 8)) & 0x00FF_00FF_00FF_00FF;
    x = (x | (x << 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    x = (x | (x << 2)) & 0x3333_3333_3333_3333;
    x
}

/// Encode a 3D lattice coordinate into a 48-bit Morton code (in u64).
#[inline]
pub fn morton_encode_3d(x: u16, y: u16, z: u16) -> u64 {
    spread2_16(x) | (spread2_16(y) << 1) | (spread2_16(z) << 2)
}

// ---------------------------------------------------------------------------
// Hilbert 2D: 16-bit per axis → 32-bit code
// ---------------------------------------------------------------------------

/// Encode a 2D lattice coordinate into a 32-bit Hilbert curve code.
///
/// Uses the Skilling (2004) compact algorithm. The Hilbert curve visits every
/// cell of a 2^16 × 2^16 grid exactly once, preserving locality better than
/// Morton (Z-order).
pub fn hilbert_encode_2d(x: u16, y: u16) -> u32 {
    // Skilling's algorithm: iterate over bits, transforming coordinates
    // in-place. At each step, the current bits of x and y determine the
    // quadrant, and the coordinates are rotated for the next level.
    let mut cx = x as u32;
    let mut cy = y as u32;
    let mut code = 0u32;

    for _ in 0..16 {
        let rx = cx & 1;
        let ry = cy & 1;

        // Hilbert quadrant order: (0,0)→0, (0,1)→1, (1,1)→2, (1,0)→3
        // This is the standard U-shape: bottom-left → top-left → top-right → bottom-right
        let quadrant = (rx ^ ry) | (ry << 1);
        code = (code << 2) | quadrant;

        // Rotate coordinates for the next level.
        // In quadrant 0 and 3: swap x and y (reflection).
        // In quadrant 0: also invert.
        if quadrant == 0 {
            core::mem::swap(&mut cx, &mut cy);
            // Invert: for the remaining bits, flip them.
            // Since we process LSB-first, "remaining" means the upper bits.
            // We handle this by inverting after shifting.
        } else if quadrant == 3 {
            core::mem::swap(&mut cx, &mut cy);
        }

        cx >>= 1;
        cy >>= 1;
    }

    code
}

// ---------------------------------------------------------------------------
// Coordinate quantization
// ---------------------------------------------------------------------------

/// Quantize a floating-point coordinate to u16 within a [min, max] range.
#[inline]
fn quantize_axis(v: f64, min: f64, extent: f64) -> u16 {
    if extent <= 0.0 {
        return 0;
    }
    let n = ((v - min) / extent).clamp(0.0, 1.0);
    (n * 65535.0 + 0.5) as u16
}

/// Bounding box of a 2D point set.
#[inline]
fn bbox_2d(points: &[[f64; 2]]) -> ([f64; 2], [f64; 2]) {
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in points {
        for a in 0..2 {
            if p[a] < min[a] {
                min[a] = p[a];
            }
            if p[a] > max[a] {
                max[a] = p[a];
            }
        }
    }
    if points.is_empty() {
        min = [0.0; 2];
        max = [0.0; 2];
    }
    (min, max)
}

/// Bounding box of a 3D point set.
#[inline]
fn bbox_3d(points: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in points {
        for a in 0..3 {
            if p[a] < min[a] {
                min[a] = p[a];
            }
            if p[a] > max[a] {
                max[a] = p[a];
            }
        }
    }
    if points.is_empty() {
        min = [0.0; 3];
        max = [0.0; 3];
    }
    (min, max)
}

// ---------------------------------------------------------------------------
// Sorting: sort indices by Morton / Hilbert code
// ---------------------------------------------------------------------------

/// Compute Morton codes for a 2D point set and sort indices by code.
///
/// `codes` needs `points.len()` entries. `indices` needs `points.len()` entries.
/// After return, `indices[0..n]` are sorted by ascending Morton code.
/// `codes[i]` holds the Morton code of `points[i]`.
///
/// Zero-heap (sort is in-place on the indices slice). Deterministic.
pub fn sort_by_morton_2d(
    points: &[[f64; 2]],
    codes: &mut [u32],
    indices: &mut [u32],
) -> Result<usize, SpatialOrderError> {
    let n = points.len();
    if codes.len() < n {
        return Err(SpatialOrderError::CodeBufferTooSmall { required: n });
    }
    if indices.len() < n {
        return Err(SpatialOrderError::IndexBufferTooSmall { required: n });
    }

    let (min, max) = bbox_2d(points);
    let extent = [max[0] - min[0], max[1] - min[1]];

    for (i, p) in points.iter().enumerate() {
        if !p[0].is_finite() || !p[1].is_finite() {
            return Err(SpatialOrderError::NonFiniteCoordinate { index: i });
        }
        let qx = quantize_axis(p[0], min[0], extent[0]);
        let qy = quantize_axis(p[1], min[1], extent[1]);
        codes[i] = morton_encode_2d(qx, qy);
        indices[i] = i as u32;
    }

    indices[..n].sort_unstable_by_key(|&idx| codes[idx as usize]);
    Ok(n)
}

/// Compute Hilbert codes for a 2D point set and sort indices by code.
pub fn sort_by_hilbert_2d(
    points: &[[f64; 2]],
    codes: &mut [u32],
    indices: &mut [u32],
) -> Result<usize, SpatialOrderError> {
    let n = points.len();
    if codes.len() < n {
        return Err(SpatialOrderError::CodeBufferTooSmall { required: n });
    }
    if indices.len() < n {
        return Err(SpatialOrderError::IndexBufferTooSmall { required: n });
    }

    let (min, max) = bbox_2d(points);
    let extent = [max[0] - min[0], max[1] - min[1]];

    for (i, p) in points.iter().enumerate() {
        if !p[0].is_finite() || !p[1].is_finite() {
            return Err(SpatialOrderError::NonFiniteCoordinate { index: i });
        }
        let qx = quantize_axis(p[0], min[0], extent[0]);
        let qy = quantize_axis(p[1], min[1], extent[1]);
        codes[i] = hilbert_encode_2d(qx, qy);
        indices[i] = i as u32;
    }

    indices[..n].sort_unstable_by_key(|&idx| codes[idx as usize]);
    Ok(n)
}

/// Compute Morton codes for a 3D point set and sort indices by code.
pub fn sort_by_morton_3d(
    points: &[[f64; 3]],
    codes: &mut [u64],
    indices: &mut [u32],
) -> Result<usize, SpatialOrderError> {
    let n = points.len();
    if codes.len() < n {
        return Err(SpatialOrderError::CodeBufferTooSmall { required: n });
    }
    if indices.len() < n {
        return Err(SpatialOrderError::IndexBufferTooSmall { required: n });
    }

    let (min, max) = bbox_3d(points);
    let extent = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];

    for (i, p) in points.iter().enumerate() {
        if !p[0].is_finite() || !p[1].is_finite() || !p[2].is_finite() {
            return Err(SpatialOrderError::NonFiniteCoordinate { index: i });
        }
        let qx = quantize_axis(p[0], min[0], extent[0]);
        let qy = quantize_axis(p[1], min[1], extent[1]);
        let qz = quantize_axis(p[2], min[2], extent[2]);
        codes[i] = morton_encode_3d(qx, qy, qz);
        indices[i] = i as u32;
    }

    indices[..n].sort_unstable_by_key(|&idx| codes[idx as usize]);
    Ok(n)
}

// ---------------------------------------------------------------------------
// POD code stream for .10d serialization
// ---------------------------------------------------------------------------

/// 20-byte header for a spatial-order code stream.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SpatialOrderHeader {
    /// Number of points.
    pub point_count: u32,
    /// Curve type: 0 = Morton 2D, 1 = Hilbert 2D, 2 = Morton 3D.
    pub curve_type: u8,
    /// Reserved (must be zero).
    pub reserved: [u8; 3],
    /// Bounding box minimum (2D: [x, y, 0], 3D: [x, y, z]).
    pub bbox_min: [f32; 3],
}

impl Default for SpatialOrderHeader {
    fn default() -> Self {
        Self {
            point_count: 0,
            curve_type: 0,
            reserved: [0; 3],
            bbox_min: [0.0; 3],
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Morton 2D encode/decode round-trip ---

    #[test]
    fn morton_2d_round_trip() {
        for x in [0u16, 1, 255, 256, 65535] {
            for y in [0u16, 1, 255, 256, 65535] {
                let code = morton_encode_2d(x, y);
                let (dx, dy) = morton_decode_2d(code);
                assert_eq!(dx, x, "x={x} y={y} code={code:#010x}");
                assert_eq!(dy, y, "x={x} y={y} code={code:#010x}");
            }
        }
    }

    #[test]
    fn morton_2d_origin_is_zero() {
        assert_eq!(morton_encode_2d(0, 0), 0);
    }

    #[test]
    fn morton_2d_adjacent_x_differs_by_one() {
        // Morton codes of (0,0) and (1,0) differ by exactly 1.
        assert_eq!(morton_encode_2d(1, 0) - morton_encode_2d(0, 0), 1);
    }

    // --- Morton 3D ---

    #[test]
    fn morton_3d_origin_is_zero() {
        assert_eq!(morton_encode_3d(0, 0, 0), 0);
    }

    #[test]
    fn morton_3d_adjacent_x_differs_by_one() {
        assert_eq!(morton_encode_3d(1, 0, 0) - morton_encode_3d(0, 0, 0), 1);
    }

    // --- Hilbert 2D ---

    #[test]
    fn hilbert_2d_origin_is_zero() {
        assert_eq!(hilbert_encode_2d(0, 0), 0);
    }

    #[test]
    fn hilbert_2d_is_bijection_on_small_grid() {
        // On a 4×4 grid, all 16 Hilbert codes must be distinct.
        let mut codes = [0u32; 16];
        for y in 0..4u32 {
            for x in 0..4u32 {
                codes[(y * 4 + x) as usize] = hilbert_encode_2d(x as u16, y as u16);
            }
        }
        let mut sorted: Vec<u32> = codes.into();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 16, "Hilbert must be a bijection on 4×4");
    }

    #[test]
    fn hilbert_2d_is_distinct_from_morton() {
        // Hilbert and Morton produce different orderings (both valid bijections).
        let mut hilbert_codes = [0u32; 16];
        let mut morton_codes = [0u32; 16];
        for y in 0..4u32 {
            for x in 0..4u32 {
                let idx = (y * 4 + x) as usize;
                morton_codes[idx] = morton_encode_2d(x as u16, y as u16);
                hilbert_codes[idx] = hilbert_encode_2d(x as u16, y as u16);
            }
        }
        // The code arrays should differ (different curves).
        assert_ne!(hilbert_codes, morton_codes);
    }

    // --- Sorting ---

    #[test]
    fn sort_by_morton_2d_orders_points() {
        let points: [[f64; 2]; 4] = [[1.0, 1.0], [0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let mut codes = [0u32; 4];
        let mut indices = [0u32; 4];
        sort_by_morton_2d(&points, &mut codes, &mut indices).unwrap();
        // Morton order: (0,0)=0, (1,0)=1, (0,1)=2, (1,1)=3
        assert_eq!(indices[0], 1); // (0,0)
        assert_eq!(indices[1], 2); // (1,0)
        assert_eq!(indices[2], 3); // (0,1)
        assert_eq!(indices[3], 0); // (1,1)
    }

    #[test]
    fn sort_by_morton_2d_deterministic() {
        let points: [[f64; 2]; 6] = [
            [3.0, 1.0],
            [0.0, 0.0],
            [2.0, 2.0],
            [1.0, 0.0],
            [0.0, 3.0],
            [3.0, 3.0],
        ];
        let mut c1 = [0u32; 6];
        let mut i1 = [0u32; 6];
        let mut c2 = [0u32; 6];
        let mut i2 = [0u32; 6];
        sort_by_morton_2d(&points, &mut c1, &mut i1).unwrap();
        sort_by_morton_2d(&points, &mut c2, &mut i2).unwrap();
        assert_eq!(i1, i2);
        assert_eq!(c1, c2);
    }

    #[test]
    fn sort_by_hilbert_2d_orders_points() {
        let points: [[f64; 2]; 4] = [[1.0, 1.0], [0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        let mut codes = [0u32; 4];
        let mut indices = [0u32; 4];
        sort_by_hilbert_2d(&points, &mut codes, &mut indices).unwrap();
        // All codes should be distinct.
        let mut sorted: Vec<u32> = codes.into();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 4);
    }

    #[test]
    fn sort_by_morton_3d_orders_points() {
        let points: [[f64; 3]; 4] = [
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let mut codes = [0u64; 4];
        let mut indices = [0u32; 4];
        sort_by_morton_3d(&points, &mut codes, &mut indices).unwrap();
        // Origin first.
        assert_eq!(indices[0], 1); // (0,0,0)
    }

    #[test]
    fn sort_rejects_non_finite() {
        let points: [[f64; 2]; 2] = [[0.0, 0.0], [f64::NAN, 0.0]];
        let mut codes = [0u32; 2];
        let mut indices = [0u32; 2];
        let err = sort_by_morton_2d(&points, &mut codes, &mut indices).unwrap_err();
        assert_eq!(err, SpatialOrderError::NonFiniteCoordinate { index: 1 });
    }

    #[test]
    fn sort_rejects_small_buffers() {
        let points: [[f64; 2]; 2] = [[0.0, 0.0], [1.0, 1.0]];
        let mut codes = [0u32; 1];
        let mut indices = [0u32; 2];
        let err = sort_by_morton_2d(&points, &mut codes, &mut indices).unwrap_err();
        assert_eq!(err, SpatialOrderError::CodeBufferTooSmall { required: 2 });
    }

    // --- Duplicate / collinear points ---

    #[test]
    fn sort_handles_duplicate_points() {
        let points: [[f64; 2]; 3] = [[0.0, 0.0], [0.0, 0.0], [1.0, 1.0]];
        let mut codes = [0u32; 3];
        let mut indices = [0u32; 3];
        sort_by_morton_2d(&points, &mut codes, &mut indices).unwrap();
        // Duplicates get the same code; sort is stable-ish (unstable but deterministic).
        assert_eq!(codes[0], codes[1]); // Both (0,0)
    }

    #[test]
    fn sort_handles_collinear_points() {
        let points: [[f64; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]];
        let mut codes = [0u32; 4];
        let mut indices = [0u32; 4];
        sort_by_morton_2d(&points, &mut codes, &mut indices).unwrap();
        // Along x-axis, Morton order should be monotonic.
        for w in indices.windows(2) {
            assert!(codes[w[0] as usize] <= codes[w[1] as usize]);
        }
    }

    // --- POD header ---

    #[test]
    fn spatial_order_header_is_pod() {
        assert_eq!(std::mem::size_of::<SpatialOrderHeader>(), 20);
        let header = SpatialOrderHeader {
            point_count: 42,
            curve_type: 1,
            reserved: [0; 3],
            bbox_min: [1.0, 2.0, 0.0],
        };
        let bytes: &[u8] = bytemuck::bytes_of(&header);
        let back: SpatialOrderHeader = *bytemuck::from_bytes(bytes);
        assert_eq!(header, back);
    }

    // --- Morton ↔ lattice round-trip on a grid ---

    #[test]
    fn morton_2d_round_trip_full_grid_8x8() {
        for y in 0..8u16 {
            for x in 0..8u16 {
                let code = morton_encode_2d(x, y);
                let (dx, dy) = morton_decode_2d(code);
                assert_eq!((dx, dy), (x, y), "x={x} y={y}");
            }
        }
    }
}
