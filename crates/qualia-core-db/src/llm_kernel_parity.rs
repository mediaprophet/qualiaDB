//! W3 — in-project GPU↔CPU kernel-parity oracle (no external LLM libs).
//!
//! The "no external libraries" rule (Prime Directive #4) means the only trustworthy reference for a
//! GPU kernel is an in-project CPU implementation of the identical math. This module supplies the
//! comparison metrics (max / mean absolute error + ULP distance) and helpers to synthesize valid
//! quantized weights, so a GPU shader can be checked against its CPU twin on random, controlled
//! inputs — fast, deterministic, and library-free. The first consumer is the GEMM parity test
//! (`QTensorEngine::gemm_parity_probe`: GPU `dispatch_gemm_raw_into` vs CPU `stack_gemm_quant`).
//!
//! Pure CPU + zero-heap on the metric paths (slices in, scalars out); safe on every target.

/// Maximum absolute error between two equal-length slices. Returns `+inf` on length mismatch so a
/// caller cannot silently pass a comparison of differently-shaped outputs.
pub fn max_abs_err(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

/// Mean absolute error (f64 accumulation to avoid catastrophic cancellation over long vectors).
pub fn mean_abs_err(a: &[f32], b: &[f32]) -> f64 {
    if a.is_empty() || a.len() != b.len() {
        return f64::INFINITY;
    }
    let s: f64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| (*x as f64 - *y as f64).abs())
        .sum();
    s / a.len() as f64
}

/// Monotone ordering key: integer distance between two keys == ULP distance for finite f32.
#[inline]
fn ulp_key(x: f32) -> i64 {
    let b = x.to_bits();
    if b & 0x8000_0000 == 0 {
        b as i64
    } else {
        -((b & 0x7fff_ffff) as i64)
    }
}

/// Maximum ULP (unit-in-the-last-place) distance over finite pairs; non-finite pairs are skipped.
/// Returns `u64::MAX` on length mismatch.
pub fn max_ulp_diff(a: &[f32], b: &[f32]) -> u64 {
    if a.len() != b.len() {
        return u64::MAX;
    }
    let mut m = 0u64;
    for (&x, &y) in a.iter().zip(b) {
        if x.is_finite() && y.is_finite() {
            let d = (ulp_key(x) - ulp_key(y)).unsigned_abs();
            if d > m {
                m = d;
            }
        }
    }
    m
}

const Q8_0_BLOCK_ELEMS: usize = 32;
const Q8_0_BLOCK_BYTES: usize = 34; // f16 scale (2) + 32 × i8

/// Bytes needed to hold `n_elems` weights in Q8_0 (ceil to whole 32-element blocks).
pub fn q8_0_bytes(n_elems: usize) -> usize {
    n_elems.div_ceil(Q8_0_BLOCK_ELEMS) * Q8_0_BLOCK_BYTES
}

/// Quantize `weights` (f32) into Q8_0 blocks in `out` (must be >= `q8_0_bytes(weights.len())`).
/// Standard ggml Q8_0: per 32-block, `scale = absmax / 127`, `q = round(w / scale)` clamped to i8,
/// stored as little-endian f16 scale followed by 32 signed bytes. Returns false if `out` is too small.
pub fn quantize_q8_0_from_f32(weights: &[f32], out: &mut [u8]) -> bool {
    if out.len() < q8_0_bytes(weights.len()) {
        return false;
    }
    let n_blocks = weights.len().div_ceil(Q8_0_BLOCK_ELEMS);
    for b in 0..n_blocks {
        let start = b * Q8_0_BLOCK_ELEMS;
        let end = (start + Q8_0_BLOCK_ELEMS).min(weights.len());
        let absmax = weights[start..end]
            .iter()
            .fold(0.0f32, |m, &w| m.max(w.abs()));
        let scale = if absmax > 0.0 { absmax / 127.0 } else { 1.0 };
        let inv = if scale > 0.0 { 1.0 / scale } else { 0.0 };
        let bs = b * Q8_0_BLOCK_BYTES;
        let s16 = half::f16::from_f32(scale).to_le_bytes();
        out[bs] = s16[0];
        out[bs + 1] = s16[1];
        for j in 0..Q8_0_BLOCK_ELEMS {
            let q = if start + j < end {
                (weights[start + j] * inv).round().clamp(-127.0, 127.0) as i8
            } else {
                0
            };
            out[bs + 2 + j] = q as u8;
        }
    }
    true
}

/// Bytes needed to hold `n_elems` weights as little-endian IEEE F16 (2 bytes each).
pub fn f16_bytes(n_elems: usize) -> usize {
    n_elems * 2
}

/// Encode `weights` (f32) as little-endian IEEE F16 into `out` (>= `f16_bytes(weights.len())`).
/// The exact byte layout `dequant_f16` / the GPU `unpack2x16float` path consume.
pub fn quantize_f16_from_f32(weights: &[f32], out: &mut [u8]) -> bool {
    if out.len() < f16_bytes(weights.len()) {
        return false;
    }
    for (i, &w) in weights.iter().enumerate() {
        let h = half::f16::from_f32(w).to_le_bytes();
        out[i * 2] = h[0];
        out[i * 2 + 1] = h[1];
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_zero_for_identical() {
        let a = [1.0f32, -2.0, 3.5, 0.0, -0.0];
        assert_eq!(max_abs_err(&a, &a), 0.0);
        assert_eq!(mean_abs_err(&a, &a), 0.0);
        assert_eq!(max_ulp_diff(&a, &a), 0);
    }

    #[test]
    fn ulp_one_step_is_one() {
        let x = 1.0f32;
        let y = f32::from_bits(x.to_bits() + 1);
        assert_eq!(max_ulp_diff(&[x], &[y]), 1);
        assert!(max_abs_err(&[x], &[y]) > 0.0);
    }

    #[test]
    fn q8_0_roundtrip_within_one_step() {
        let w: Vec<f32> = (0..32).map(|i| (i as f32 - 16.0) / 16.0).collect();
        let mut bytes = vec![0u8; q8_0_bytes(w.len())];
        assert!(quantize_q8_0_from_f32(&w, &mut bytes));
        let info = crate::gguf_sharder::GgufTensorInfo {
            dims: [32, 1, 1, 1],
            n_dims: 1,
            ggml_type: crate::ggml_quants::GGML_TYPE_Q8_0,
            byte_offset: 0,
        };
        let mut back = vec![0f32; 32];
        let n = crate::ggml_quants::dequant_matrix_row_into(&bytes, &info, 0, &mut back).unwrap();
        assert_eq!(n, 32);
        // absmax = 15/16 → scale ≈ (15/16)/127; round-trip error ≤ one quant step.
        let step = (15.0f32 / 16.0) / 127.0;
        assert!(max_abs_err(&w, &back) <= step + 1e-5);
    }
}
