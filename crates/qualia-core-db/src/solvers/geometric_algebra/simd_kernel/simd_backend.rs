//! Low-level Cl(3,0) array kernels.
//!
//! The `[f32; 8]` geometric / outer products, their basis sign tables, and the
//! AVX2+FMA SIMD fast path with a scalar fallback. Everything above this layer
//! (`types`, `operations`) is structured math built on these primitives.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use core::arch::x86_64::*;
use std::sync::OnceLock;

/// Basis mapping for Cl_3,0 (8 dimensions):
/// 0: 1 (Scalar), 1: e1, 2: e2, 3: e3 (Vector), 4: e12, 5: e13, 6: e23 (Bivector),
/// 7: e123 (Pseudoscalar).
pub static GA_SIMD_KERNEL: OnceLock<GaKernel> = OnceLock::new();

pub struct GaKernel {
    pub has_avx2: bool,
}

impl GaKernel {
    pub fn init() -> Self {
        Self {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            has_avx2: is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma"),
            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            has_avx2: false,
        }
    }
}

const GEOMETRIC_PRODUCT_TABLE: [[(usize, f32); 8]; 8] = [
    // e_j = 1
    [
        (0, 1.),
        (1, 1.),
        (2, 1.),
        (3, 1.),
        (4, 1.),
        (5, 1.),
        (6, 1.),
        (7, 1.),
    ],
    // e_j = e1
    [
        (1, 1.),
        (0, 1.),
        (4, -1.),
        (5, -1.),
        (2, -1.),
        (3, -1.),
        (7, 1.),
        (6, 1.),
    ],
    // e_j = e2
    [
        (2, 1.),
        (4, 1.),
        (0, 1.),
        (6, -1.),
        (1, 1.),
        (7, -1.),
        (3, -1.),
        (5, -1.),
    ],
    // e_j = e3
    [
        (3, 1.),
        (5, 1.),
        (6, 1.),
        (0, 1.),
        (7, 1.),
        (1, 1.),
        (2, 1.),
        (4, 1.),
    ],
    // e_j = e12
    [
        (4, 1.),
        (2, 1.),
        (1, -1.),
        (7, 1.),
        (0, -1.),
        (6, 1.),
        (5, -1.),
        (3, -1.),
    ],
    // e_j = e13
    [
        (5, 1.),
        (3, 1.),
        (7, -1.),
        (1, -1.),
        (6, -1.),
        (0, -1.),
        (4, 1.),
        (2, 1.),
    ],
    // e_j = e23
    [
        (6, 1.),
        (7, 1.),
        (3, 1.),
        (2, -1.),
        (5, 1.),
        (4, -1.),
        (0, -1.),
        (1, -1.),
    ],
    // e_j = e123
    [
        (7, 1.),
        (6, 1.),
        (5, -1.),
        (4, 1.),
        (3, -1.),
        (2, 1.),
        (1, -1.),
        (0, -1.),
    ],
];

const OUTER_PRODUCT_TABLE: [[(usize, f32); 8]; 8] = [
    // 1 wedge e_j = e_j
    [
        (0, 1.),
        (1, 1.),
        (2, 1.),
        (3, 1.),
        (4, 1.),
        (5, 1.),
        (6, 1.),
        (7, 1.),
    ],
    // e1 wedge e_j
    [
        (1, 1.),
        (0, 0.),
        (4, -1.),
        (5, -1.),
        (0, 0.),
        (0, 0.),
        (7, 1.),
        (0, 0.),
    ],
    // e2 wedge e_j
    [
        (2, 1.),
        (4, 1.),
        (0, 0.),
        (6, -1.),
        (0, 0.),
        (7, -1.),
        (0, 0.),
        (0, 0.),
    ],
    // e3 wedge e_j
    [
        (3, 1.),
        (5, 1.),
        (6, 1.),
        (0, 0.),
        (7, 1.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
    ],
    // e12 wedge e_j
    [
        (4, 1.),
        (0, 0.),
        (0, 0.),
        (7, 1.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
    ],
    // e13 wedge e_j
    [
        (5, 1.),
        (0, 0.),
        (7, -1.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
    ],
    // e23 wedge e_j
    [
        (6, 1.),
        (7, 1.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
    ],
    // e123 wedge e_j
    [
        (7, 1.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
        (0, 0.),
    ],
];

/// # Safety
/// This function requires the CPU to support the `AVX2` and `FMA` instruction sets.
/// Calling this on hardware without these features will result in undefined behavior (SIGILL).
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2,fma")]
pub unsafe fn geometric_product_avx2(a: __m256, b: __m256) -> __m256 {
    let mut result = _mm256_setzero_ps();
    // j = 0
    let b0 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(0));
    let perm0 = _mm256_set_epi32(7, 6, 5, 4, 3, 2, 1, 0);
    let a_shuf0 = _mm256_permutevar8x32_ps(a, perm0);
    result = _mm256_fmadd_ps(a_shuf0, b0, result);

    // j = 1
    let b1 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(1));
    let perm1 = _mm256_set_epi32(6, 7, 3, 2, 5, 4, 0, 1);
    let a_shuf1 = _mm256_permutevar8x32_ps(a, perm1);
    let sign1 = _mm256_castsi256_ps(_mm256_set_epi32(
        0,
        0,
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
        0,
        0,
    ));
    let a_signed1 = _mm256_xor_ps(a_shuf1, sign1);
    result = _mm256_fmadd_ps(a_signed1, b1, result);

    // j = 2
    let b2 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(2));
    let perm2 = _mm256_set_epi32(5, 3, 7, 1, 6, 0, 4, 2);
    let a_shuf2 = _mm256_permutevar8x32_ps(a, perm2);
    let sign2 = _mm256_castsi256_ps(_mm256_set_epi32(
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
        0,
        0x80000000_u32 as i32,
        0,
        0,
        0,
    ));
    let a_signed2 = _mm256_xor_ps(a_shuf2, sign2);
    result = _mm256_fmadd_ps(a_signed2, b2, result);

    // j = 3
    let b3 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(3));
    let perm3 = _mm256_set_epi32(4, 2, 1, 7, 0, 6, 5, 3);
    let a_shuf3 = _mm256_permutevar8x32_ps(a, perm3);
    result = _mm256_fmadd_ps(a_shuf3, b3, result);

    // j = 4
    let b4 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(4));
    let perm4 = _mm256_set_epi32(3, 5, 6, 0, 7, 1, 2, 4);
    let a_shuf4 = _mm256_permutevar8x32_ps(a, perm4);
    let sign4 = _mm256_castsi256_ps(_mm256_set_epi32(
        0,
        0,
        0x80000000_u32 as i32,
        0,
        0x80000000_u32 as i32,
        0,
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
    ));
    let a_signed4 = _mm256_xor_ps(a_shuf4, sign4);
    result = _mm256_fmadd_ps(a_signed4, b4, result);

    // j = 5
    let b5 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(5));
    let perm5 = _mm256_set_epi32(2, 4, 0, 6, 1, 7, 3, 5);
    let a_shuf5 = _mm256_permutevar8x32_ps(a, perm5);
    let sign5 = _mm256_castsi256_ps(_mm256_set_epi32(
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
        0,
        0,
        0,
        0,
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
    ));
    let a_signed5 = _mm256_xor_ps(a_shuf5, sign5);
    result = _mm256_fmadd_ps(a_signed5, b5, result);

    // j = 6
    let b6 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(6));
    let perm6 = _mm256_set_epi32(1, 0, 4, 5, 2, 3, 7, 6);
    let a_shuf6 = _mm256_permutevar8x32_ps(a, perm6);
    let sign6 = _mm256_castsi256_ps(_mm256_set_epi32(
        0,
        0,
        0,
        0x80000000_u32 as i32,
        0,
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
    ));
    let a_signed6 = _mm256_xor_ps(a_shuf6, sign6);
    result = _mm256_fmadd_ps(a_signed6, b6, result);

    // j = 7
    let b7 = _mm256_permutevar8x32_ps(b, _mm256_set1_epi32(7));
    let perm7 = _mm256_set_epi32(0, 1, 2, 3, 4, 5, 6, 7);
    let a_shuf7 = _mm256_permutevar8x32_ps(a, perm7);
    let sign7 = _mm256_castsi256_ps(_mm256_set_epi32(
        0,
        0,
        0x80000000_u32 as i32,
        0,
        0x80000000_u32 as i32,
        0,
        0x80000000_u32 as i32,
        0x80000000_u32 as i32,
    ));
    let a_signed7 = _mm256_xor_ps(a_shuf7, sign7);
    result = _mm256_fmadd_ps(a_signed7, b7, result);

    result
}

pub fn multivector_geometric_product(a: &[f32; 8], b: &[f32; 8]) -> [f32; 8] {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let kernel = GA_SIMD_KERNEL.get_or_init(GaKernel::init);
        if kernel.has_avx2 {
            unsafe {
                let a_simd = _mm256_loadu_ps(a.as_ptr());
                let b_simd = _mm256_loadu_ps(b.as_ptr());
                let res_simd = geometric_product_avx2(a_simd, b_simd);
                let mut out = [0.0; 8];
                _mm256_storeu_ps(out.as_mut_ptr(), res_simd);
                return out;
            }
        }
    }

    let mut out = [0.0; 8];
    for j in 0..8 {
        let b_j = b[j];
        if b_j == 0.0 {
            continue;
        }
        for i in 0..8 {
            let a_i = a[i];
            if a_i == 0.0 {
                continue;
            }
            let (res_idx, sign) = GEOMETRIC_PRODUCT_TABLE[j][i];
            out[res_idx] += a_i * b_j * sign;
        }
    }
    out
}

pub fn multivector_outer_product(a: &[f32; 8], b: &[f32; 8]) -> [f32; 8] {
    // For now, outer product is only scalar.
    let mut out = [0.0; 8];
    for j in 0..8 {
        let b_j = b[j];
        if b_j == 0.0 {
            continue;
        }
        for i in 0..8 {
            let a_i = a[i];
            if a_i == 0.0 {
                continue;
            }
            let (res_idx, sign) = OUTER_PRODUCT_TABLE[j][i];
            if sign != 0.0 {
                out[res_idx] += a_i * b_j * sign;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec3(x: f32, y: f32, z: f32) -> [f32; 8] {
        [0.0, x, y, z, 0.0, 0.0, 0.0, 0.0]
    }

    #[test]
    fn test_geometric_product_vectors() {
        let a = vec3(1.0, 2.0, 3.0);
        let b = vec3(4.0, 5.0, 6.0);
        let c = multivector_geometric_product(&a, &b);
        assert_eq!(c[0], 32.0, "Scalar part should equal the dot product");
        assert_eq!(c[4], -3.0);
        assert_eq!(c[5], -6.0);
        assert_eq!(c[6], -3.0);
        assert_eq!(c[1], 0.0);
        assert_eq!(c[2], 0.0);
        assert_eq!(c[3], 0.0);
        assert_eq!(c[7], 0.0);
    }

    #[test]
    fn test_outer_product_self_is_zero() {
        let a = vec3(2.0, -1.0, 4.0);
        let a_squared = multivector_geometric_product(&a, &a);
        assert_eq!(a_squared[0], 4.0 + 1.0 + 16.0);
        for i in 1..8 {
            assert_eq!(a_squared[i], 0.0, "Component {} must be zero", i);
        }
    }
}
