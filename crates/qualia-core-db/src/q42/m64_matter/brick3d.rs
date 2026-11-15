//! 3D Volumetric Bricks and Spatial Indexing for M64 Matter Containers.
//!
//! Provides 3D Hilbert curve spatial addressing for macro-level chunk clustering
//! and 3D Morton (Z-order) bit-interleaving for intra-brick `16×16×16` (4,096-voxel)
//! cache locality in L1/L2 caches.

/// Side dimension of an M64 3D volumetric brick (16 voxels along each axis).
pub const M64_BRICK3D_SIDE: usize = 16;

/// Total number of 64-byte records in one 16×16×16 brick (4,096 records).
pub const M64_BRICK3D_RECORDS: usize = 4096;

/// Total byte size of one 16×16×16 brick payload (256 KB = 262,144 bytes).
pub const M64_BRICK3D_BYTES: usize = 4096 * 64;

/// Transform in-place from geometrical axes to Hilbert transposed bits.
///
/// Implements John Skilling's compact algorithm (2004) for dimension N = 3.
#[inline]
fn axes_to_transpose_3d(x: &mut [u32; 3], b: u32) {
    if b == 0 {
        return;
    }
    let m = 1u32 << (b - 1);
    let mut q = m;
    while q > 1 {
        let p = q - 1;
        for i in 0..3 {
            if (x[i] & q) != 0 {
                x[0] ^= p;
            } else {
                let t = (x[0] ^ x[i]) & p;
                x[0] ^= t;
                x[i] ^= t;
            }
        }
        q >>= 1;
    }

    for i in 1..3 {
        x[i] ^= x[i - 1];
    }
    let mut t = 0u32;
    let mut q = m;
    while q > 1 {
        if (x[2] & q) != 0 {
            t ^= q - 1;
        }
        q >>= 1;
    }
    for i in 0..3 {
        x[i] ^= t;
    }
}

/// Transform in-place from Hilbert transposed bits back to geometrical axes.
///
/// Implements John Skilling's compact algorithm (2004) for dimension N = 3.
#[inline]
fn transpose_to_axes_3d(x: &mut [u32; 3], b: u32) {
    if b == 0 {
        return;
    }
    let z = 2u32 << (b - 1);
    let mut t = x[2] >> 1;
    for i in (1..3).rev() {
        x[i] ^= x[i - 1];
    }
    x[0] ^= t;

    let mut q = 2u32;
    while q != z {
        let p = q - 1;
        for i in (0..3).rev() {
            if (x[i] & q) != 0 {
                x[0] ^= p;
            } else {
                t = (x[0] ^ x[i]) & p;
                x[0] ^= t;
                x[i] ^= t;
            }
        }
        q <<= 1;
    }
}

/// Encode 3D brick coordinates `(bx, by, bz)` to a 64-bit Hilbert index.
///
/// Supports up to order 21 (21 bits per axis, $21 \times 3 = 63$ bits $\le 64$ bits).
#[inline]
pub fn encode_hilbert3d_u64(bx: u32, by: u32, bz: u32, order: u32) -> u64 {
    let order = order.min(21);
    if order == 0 {
        return 0;
    }
    let mask = (1u32 << order) - 1;
    let mut x = [bx & mask, by & mask, bz & mask];
    axes_to_transpose_3d(&mut x, order);

    let mut h = 0u64;
    for k in 0..order {
        let bit_base = (k * 3) as usize;
        h |= (((x[2] >> k) & 1) as u64) << bit_base;
        h |= (((x[1] >> k) & 1) as u64) << (bit_base + 1);
        h |= (((x[0] >> k) & 1) as u64) << (bit_base + 2);
    }
    h
}

/// Decode a 64-bit Hilbert index back to 3D brick coordinates `(bx, by, bz)`.
///
/// Supports up to order 21 (21 bits per axis).
#[inline]
pub fn decode_hilbert3d_u64(h: u64, order: u32) -> (u32, u32, u32) {
    let order = order.min(21);
    if order == 0 {
        return (0, 0, 0);
    }
    let mut x = [0u32; 3];
    for k in 0..order {
        let bit_base = (k * 3) as usize;
        x[2] |= (((h >> bit_base) & 1) as u32) << k;
        x[1] |= (((h >> (bit_base + 1)) & 1) as u32) << k;
        x[0] |= (((h >> (bit_base + 2)) & 1) as u32) << k;
    }
    transpose_to_axes_3d(&mut x, order);
    (x[0], x[1], x[2])
}

/// Computes the linear 1D index `0..4096` of a voxel inside a 16×16×16 brick.
///
/// Uses 3D Morton (Z-order) interleaving across the 4-bit local coordinates
/// (`lx, ly, lz` in `0..16`), maximizing cache-line spatial locality.
#[inline]
pub fn brick_voxel_linear_index(lx: u8, ly: u8, lz: u8) -> usize {
    let x = (lx as usize) & 0x0F;
    let y = (ly as usize) & 0x0F;
    let z = (lz as usize) & 0x0F;
    let mut morton = 0usize;
    for i in 0..4 {
        morton |= ((x >> i) & 1) << (3 * i);
        morton |= ((y >> i) & 1) << (3 * i + 1);
        morton |= ((z >> i) & 1) << (3 * i + 2);
    }
    morton
}

/// Computes local 3D coordinates `(lx, ly, lz)` from an intra-brick linear index `0..4096`.
#[inline]
pub fn brick_voxel_coords(idx: usize) -> (u8, u8, u8) {
    let mut x = 0u8;
    let mut y = 0u8;
    let mut z = 0u8;
    for i in 0..4 {
        x |= (((idx >> (3 * i)) & 1) as u8) << i;
        y |= (((idx >> (3 * i + 1)) & 1) as u8) << i;
        z |= (((idx >> (3 * i + 2)) & 1) as u8) << i;
    }
    (x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brick3d_constants() {
        assert_eq!(M64_BRICK3D_SIDE, 16);
        assert_eq!(M64_BRICK3D_RECORDS, 4096);
        assert_eq!(M64_BRICK3D_BYTES, 262_144);
    }

    #[test]
    fn test_brick_voxel_morton_roundtrip() {
        for lz in 0..16u8 {
            for ly in 0..16u8 {
                for lx in 0..16u8 {
                    let idx = brick_voxel_linear_index(lx, ly, lz);
                    assert!(idx < 4096);
                    let (rx, ry, rz) = brick_voxel_coords(idx);
                    assert_eq!((rx, ry, rz), (lx, ly, lz));
                }
            }
        }
    }

    #[test]
    fn test_hilbert3d_roundtrip_exhaustive() {
        for order in 1..=3 {
            let side = 1u32 << order;
            for z in 0..side {
                for y in 0..side {
                    for x in 0..side {
                        let h = encode_hilbert3d_u64(x, y, z, order);
                        let (rx, ry, rz) = decode_hilbert3d_u64(h, order);
                        assert_eq!((rx, ry, rz), (x, y, z));
                    }
                }
            }
        }
    }

    #[test]
    fn test_hilbert3d_roundtrip_samples() {
        for order in [4, 8, 16, 21] {
            let max_val = if order == 21 { 0x001F_FFFF } else { (1u32 << order) - 1 };
            let test_points = [
                (0, 0, 0),
                (1, 0, 0),
                (0, 1, 0),
                (0, 0, 1),
                (max_val, max_val, max_val),
                (max_val / 2, max_val / 3, max_val / 4),
            ];
            for &(x, y, z) in &test_points {
                let h = encode_hilbert3d_u64(x, y, z, order);
                let (rx, ry, rz) = decode_hilbert3d_u64(h, order);
                assert_eq!((rx, ry, rz), (x, y, z), "Failed for order {order}");
            }
        }
    }

    #[test]
    fn test_hilbert3d_continuity() {
        let order = 2;
        let total = 1u64 << (3 * order);
        let mut prev = decode_hilbert3d_u64(0, order);
        for h in 1..total {
            let curr = decode_hilbert3d_u64(h, order);
            let dist = curr.0.abs_diff(prev.0) + curr.1.abs_diff(prev.1) + curr.2.abs_diff(prev.2);
            assert_eq!(dist, 1, "Discontinuity at h={h}");
            prev = curr;
        }
    }
}
