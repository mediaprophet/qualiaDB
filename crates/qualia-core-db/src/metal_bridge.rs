//! Metal / Accelerate inference bridge (macOS and iOS — Apple Silicon).
//!
//! Two compute paths, used in priority order by `gguf_bridge::QTensorEngine`:
//!
//!   1. Accelerate BLAS — `cblas_sgemm` via the Accelerate.framework.
//!      On M-series chips this dispatches to the Apple AMX coprocessor
//!      (dedicated matrix-multiply hardware, zero GPU scheduling overhead).
//!      Fastest for small-to-mid matrix sizes typical in Q4_K transformer blocks.
//!
//!   2. wgpu / Metal compute shaders — handled by `QTensorEngine`'s wgpu device,
//!      which selects Metal as its backend on macOS automatically.
//!      Use for very large tensors where GPU parallelism wins.
//!
//! Q4_K dequantization is identical to `directml_bridge` — kept self-contained
//! so each platform bridge compiles independently.
//!
//! On non-Apple targets this entire module is compiled out via lib.rs.

#![cfg(any(target_os = "macos", target_os = "ios"))]

// ─── Q4_K dequantization ──────────────────────────────────────────────────────

/// Bytes per Q4_K block: 2 × f16 (scale, min) + 16 nibble bytes = 20 bytes, 32 weights.
pub const Q4_K_BLOCK_BYTES: usize = 20;
/// Weights per Q4_K block.
pub const Q4_K_BLOCK_SIZE:  usize = 32;

/// Dequantize one Q4_K block into 32 f32 values.
///
/// Layout: [scale_f16(2)] [min_f16(2)] [nibble_pairs(16)]
#[inline]
pub fn dequantize_q4_k_block(block: &[u8; Q4_K_BLOCK_BYTES], out: &mut [f32; Q4_K_BLOCK_SIZE]) {
    let scale = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
    let min   = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
    for i in 0..16 {
        let byte = block[4 + i];
        out[i * 2]     = (byte & 0x0F) as f32 * scale + min;
        out[i * 2 + 1] = (byte >> 4)   as f32 * scale + min;
    }
}

/// Dequantize a full Q4_K tensor slice to flat f32.
pub fn dequantize_q4_k_tensor(q4k_bytes: &[u8], num_elements: usize) -> Vec<f32> {
    let num_blocks = q4k_bytes.len() / Q4_K_BLOCK_BYTES;
    let mut out = vec![0f32; num_blocks * Q4_K_BLOCK_SIZE];
    for b in 0..num_blocks {
        let block: &[u8; Q4_K_BLOCK_BYTES] = q4k_bytes[b * Q4_K_BLOCK_BYTES..][..Q4_K_BLOCK_BYTES]
            .try_into().unwrap();
        let mut deq = [0f32; Q4_K_BLOCK_SIZE];
        dequantize_q4_k_block(block, &mut deq);
        out[b * Q4_K_BLOCK_SIZE..(b + 1) * Q4_K_BLOCK_SIZE].copy_from_slice(&deq);
    }
    out.truncate(num_elements);
    out
}

// ─── Accelerate BLAS (cblas_sgemm) ───────────────────────────────────────────
//
// Accelerate.framework is linked by build.rs (`cargo:rustc-link-lib=framework=Accelerate`).
// On M1/M2/M3/M4 the BLAS implementation routes to the AMX coprocessor —
// a dedicated fixed-function matrix-multiply unit that is faster than Metal
// compute shaders for transformer-sized (4096×4096) GEMM at Q4_K bit-width.

/// cblas_sgemm from Accelerate.framework.
///
/// Order constants (row-major / no-transpose) are passed inline; the header
/// is not needed because the function is stable ABI across all macOS versions.
#[link(name = "Accelerate", kind = "framework")]
extern "C" {
    fn cblas_sgemm(
        order:  i32,       // 101 = CblasRowMajor
        transa: i32,       // 111 = CblasNoTrans
        transb: i32,       // 111 = CblasNoTrans
        m:      i32,       // rows of A and C
        n:      i32,       // cols of B and C
        k:      i32,       // cols of A / rows of B
        alpha:  f32,
        a:      *const f32,
        lda:    i32,
        b:      *const f32,
        ldb:    i32,
        beta:   f32,
        c:      *mut f32,
        ldc:    i32,
    );
}

const CBLAS_ROW_MAJOR: i32 = 101;
const CBLAS_NO_TRANS:  i32 = 111;

/// Execute C = A(m×k) × B(k×n) using Accelerate BLAS.
///
/// On Apple Silicon the AMX coprocessor handles this without occupying
/// the GPU compute engine, leaving Metal free for attention / softmax.
pub fn accelerate_sgemm(m: usize, k: usize, n: usize, a: &[f32], b: &[f32]) -> Vec<f32> {
    debug_assert_eq!(a.len(), m * k, "A shape mismatch");
    debug_assert_eq!(b.len(), k * n, "B shape mismatch");
    let mut c = vec![0f32; m * n];
    unsafe {
        cblas_sgemm(
            CBLAS_ROW_MAJOR, CBLAS_NO_TRANS, CBLAS_NO_TRANS,
            m as i32, n as i32, k as i32,
            1.0,
            a.as_ptr(), k as i32,
            b.as_ptr(), n as i32,
            0.0,
            c.as_mut_ptr(), n as i32,
        );
    }
    c
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dequantize_q4k_midpoint() {
        let mut block = [0u8; Q4_K_BLOCK_BYTES];
        // scale = f16(1.0) = 0x3C00, min = f16(0.0) = 0x0000, all nibbles = 7
        block[0] = 0x00; block[1] = 0x3C;
        block[2] = 0x00; block[3] = 0x00;
        for i in 4..Q4_K_BLOCK_BYTES { block[i] = 0x77; }
        let mut out = [0f32; Q4_K_BLOCK_SIZE];
        dequantize_q4_k_block(&block, &mut out);
        for v in &out { assert!((v - 7.0).abs() < 1e-3, "got {v}"); }
    }

    #[test]
    fn accelerate_identity_2x2() {
        // A = I(2×2), B = [[1,2],[3,4]] → C should equal B
        let a = [1f32, 0., 0., 1.];
        let b = [1f32, 2., 3., 4.];
        let c = accelerate_sgemm(2, 2, 2, &a, &b);
        assert!((c[0] - 1.).abs() < 1e-5, "c[0]={}", c[0]);
        assert!((c[1] - 2.).abs() < 1e-5, "c[1]={}", c[1]);
        assert!((c[2] - 3.).abs() < 1e-5, "c[2]={}", c[2]);
        assert!((c[3] - 4.).abs() < 1e-5, "c[3]={}", c[3]);
    }

    #[test]
    fn accelerate_smoke_4096x4096() {
        // Validates the AMX path doesn't panic on transformer-sized input.
        // Uses zeros so the result is trivially correct.
        let a = vec![0f32; 4096 * 4096];
        let b = vec![0f32; 4096 * 4096];
        let c = accelerate_sgemm(4096, 4096, 4096, &a, &b);
        assert_eq!(c.len(), 4096 * 4096);
    }
}
