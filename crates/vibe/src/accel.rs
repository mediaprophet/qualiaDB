//! Acceleration detection and fail-closed numeric kernels (P16.3).
//!
//! Default Vibe eval stays scalar AST. SIMD is used only by the numeric
//! slice kernel when the CPU advertises it. WASM stays sequential and
//! caps a cell cluster at 64.

use crate::bind::AccelerationTier;

/// Native ready-cluster ceiling (P16.2).
pub const CELL_BATCH_NATIVE: usize = 512;
/// WASM sequential chunk (P16.3).
pub const CELL_BATCH_WASM: usize = 64;

pub fn cell_batch_cap() -> usize {
    #[cfg(target_arch = "wasm32")]
    {
        CELL_BATCH_WASM
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        CELL_BATCH_NATIVE
    }
}

/// What the CPU/adapter could run. Not what this eval pass is using.
pub fn detect_available_tier() -> AccelerationTier {
    #[cfg(target_arch = "wasm32")]
    {
        AccelerationTier::ScalarCpu
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx512f") {
            return AccelerationTier::VectorSimd512;
        }
        AccelerationTier::ScalarCpu
    }
}

/// Elementwise `out[i] = a[i] + b[i]`. AVX-512 when present; else scalar.
pub fn add_f64_slices(a: &[f64], b: &[f64], out: &mut [f64]) {
    let n = a.len().min(b.len()).min(out.len());
    #[cfg(all(target_arch = "x86_64", not(target_arch = "wasm32")))]
    {
        if is_x86_feature_detected!("avx512f") {
            unsafe {
                add_avx512(a, b, out, n);
            }
            return;
        }
    }
    for i in 0..n {
        out[i] = a[i] + b[i];
    }
}

#[cfg(all(target_arch = "x86_64", not(target_arch = "wasm32")))]
#[target_feature(enable = "avx512f")]
unsafe fn add_avx512(a: &[f64], b: &[f64], out: &mut [f64], n: usize) {
    use std::arch::x86_64::*;
    let mut i = 0;
    while i + 8 <= n {
        let va = _mm512_loadu_pd(a.as_ptr().add(i));
        let vb = _mm512_loadu_pd(b.as_ptr().add(i));
        _mm512_storeu_pd(out.as_mut_ptr().add(i), _mm512_add_pd(va, vb));
        i += 8;
    }
    while i < n {
        out[i] = a[i] + b[i];
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_or_native_cap_is_bounded() {
        let cap = cell_batch_cap();
        assert!(cap == CELL_BATCH_NATIVE || cap == CELL_BATCH_WASM);
        assert!(cap <= CELL_BATCH_NATIVE);
    }

    #[test]
    fn scalar_add_matches_manual() {
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let b = [0.5; 9];
        let mut out = [0.0; 9];
        add_f64_slices(&a, &b, &mut out);
        for i in 0..9 {
            assert!((out[i] - (a[i] + b[i])).abs() < 1e-12);
        }
    }

    #[test]
    fn wasm_never_advertises_simd() {
        #[cfg(target_arch = "wasm32")]
        assert_eq!(detect_available_tier(), AccelerationTier::ScalarCpu);
    }
}
