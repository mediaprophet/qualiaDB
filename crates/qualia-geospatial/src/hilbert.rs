//! 2D Hilbert Space-Filling Curve Mapping.
//!
//! Maps 2D spatial coordinates $(x, y)$ to a 1D scalar index $d$ and vice-versa,
//! preserving spatial proximity for memory-mapped chunk caching and GPU spatial lookups.

/// Converts 2D grid coordinates $(x, y)$ to a 1D Hilbert index $d$.
///
/// `order`: Bits per dimension (e.g. order 6 for $64 \times 64$ grid, order 16 for $65536 \times 65536$).
pub fn xy_to_hilbert(mut x: u32, mut y: u32, order: usize) -> u64 {
    let mut d = 0u64;
    let mut s = 1u32 << (order - 1);

    while s > 0 {
        let rx = if (x & s) > 0 { 1u32 } else { 0u32 };
        let ry = if (y & s) > 0 { 1u32 } else { 0u32 };

        d += (s as u64) * (s as u64) * ((3 * rx) ^ ry) as u64;

        rot(s, &mut x, &mut y, rx, ry);
        s >>= 1;
    }

    d
}

/// Converts a 1D Hilbert index $d$ back to 2D grid coordinates $(x, y)$.
pub fn hilbert_to_xy(mut d: u64, order: usize) -> (u32, u32) {
    let mut x = 0u32;
    let mut y = 0u32;
    let mut s = 1u32;

    while s < (1u32 << order) {
        let rx = 1 & (d / 2) as u32;
        let ry = 1 & (d as u32 ^ rx);

        rot(s, &mut x, &mut y, rx, ry);
        x += s * rx;
        y += s * ry;
        d /= 4;
        s <<= 1;
    }

    (x, y)
}

/// Rotation / reflection transform for Hilbert quadrants.
#[inline]
fn rot(n: u32, x: &mut u32, y: &mut u32, rx: u32, ry: u32) {
    if ry == 0 {
        if rx == 1 {
            *x = (n - 1).wrapping_sub(*x);
            *y = (n - 1).wrapping_sub(*y);
        }
        std::mem::swap(x, y);
    }
}
