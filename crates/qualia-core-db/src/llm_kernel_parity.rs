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

/// Bytes for `n_elems` weights as ggml Q4_0 (18-byte blocks of 32: f16 scale + 16 nibble bytes).
pub fn q4_0_bytes(n_elems: usize) -> usize {
    n_elems.div_ceil(32) * 18
}

/// Quantize `weights` (f32) to ggml **Q4_0** into `out` (>= `q4_0_bytes`). Matches
/// `ggml_quants::dequant_q4_0` exactly: per 32-block `d = max_abs_signed / -8`, nibble
/// `q = clamp(round(x/d)+8, 0..15)`, dequant `x = (q-8)*d`; **interleaved** layout — block index
/// `k < 16` is the low nibble of byte `k`, `k >= 16` the high nibble of byte `k-16`.
pub fn quantize_q4_0_from_f32(weights: &[f32], out: &mut [u8]) -> bool {
    if out.len() < q4_0_bytes(weights.len()) {
        return false;
    }
    let n_blocks = weights.len().div_ceil(32);
    for b in 0..n_blocks {
        let start = b * 32;
        let end = (start + 32).min(weights.len());
        let mut amax = 0.0f32;
        let mut max_signed = 0.0f32;
        for &x in &weights[start..end] {
            if x.abs() > amax {
                amax = x.abs();
                max_signed = x;
            }
        }
        let d = max_signed / -8.0;
        let id = if d != 0.0 { 1.0 / d } else { 0.0 };
        let bs = b * 18;
        out[bs..bs + 2].copy_from_slice(&half::f16::from_f32(d).to_le_bytes());
        let q = |k: usize| -> u8 {
            let gk = start + k;
            if gk >= end {
                return 8; // (8-8)*d = 0 padding
            }
            (weights[gk] * id + 8.5).floor().clamp(0.0, 15.0) as u8
        };
        for j in 0..16 {
            out[bs + 2 + j] = (q(j) & 0x0F) | ((q(j + 16) & 0x0F) << 4);
        }
    }
    true
}

/// Bytes for `n_elems` weights as ggml Q4_K (144-byte super-blocks of 256).
pub fn q4_k_bytes(n_elems: usize) -> usize {
    n_elems.div_ceil(256) * 144
}

/// Quantize `weights` (f32) to ggml **Q4_K** into `out` (>= `q4_k_bytes`). Matches
/// `ggml_quants::dequant_q4_k`: super-block of 256 = 8 sub-blocks of 32, each with an asymmetric
/// scale+min — 6-bit sub-scales (`d*sc`) and mins (`dmin*m`) packed via `get_scale_min_k4`, 4-bit
/// quants (even sub-block = low nibble, odd = high nibble of the same `qs` byte). Dequant:
/// `x = d*sc[s]*q - dmin*m[s]`. Simplified vs ggml's iterative search (per-sub-block min clamped ≤ 0,
/// which holds for zero-centred weights); round-trip tested. Q4_K's 6-bit sub-scales make it markedly
/// more accurate per bit than Q4_0 — AWQ's intended 4-bit partner.
pub fn quantize_q4_k_from_f32(weights: &[f32], out: &mut [u8]) -> bool {
    if out.len() < q4_k_bytes(weights.len()) {
        return false;
    }
    let n_super = weights.len().div_ceil(256);
    for sb in 0..n_super {
        let base = sb * 256;
        let bb = sb * 144;
        let mut scale_s = [0f32; 8];
        let mut mt_s = [0f32; 8];
        for s in 0..8 {
            let s0 = base + s * 32;
            if s0 >= weights.len() {
                continue;
            }
            let s1 = (s0 + 32).min(weights.len());
            let mut mn = f32::INFINITY;
            let mut mx = f32::NEG_INFINITY;
            for &x in &weights[s0..s1] {
                mn = mn.min(x);
                mx = mx.max(x);
            }
            let eff_min = mn.min(0.0); // ≤ 0 so the shared dmin stays non-negative
            scale_s[s] = ((mx - eff_min) / 15.0).max(0.0);
            mt_s[s] = -eff_min; // ≥ 0
        }
        let d = scale_s.iter().cloned().fold(0.0f32, f32::max) / 63.0;
        let dmin = mt_s.iter().cloned().fold(0.0f32, f32::max) / 63.0;
        let idd = if d > 0.0 { 1.0 / d } else { 0.0 };
        let idm = if dmin > 0.0 { 1.0 / dmin } else { 0.0 };
        let mut sc = [0u8; 8];
        let mut m = [0u8; 8];
        for s in 0..8 {
            sc[s] = (scale_s[s] * idd).round().clamp(0.0, 63.0) as u8;
            m[s] = (mt_s[s] * idm).round().clamp(0.0, 63.0) as u8;
        }
        out[bb..bb + 2].copy_from_slice(&half::f16::from_f32(d).to_le_bytes());
        out[bb + 2..bb + 4].copy_from_slice(&half::f16::from_f32(dmin).to_le_bytes());
        // Pack sc/m into scales[12] — inverse of get_scale_min_k4.
        let mut scales = [0u8; 12];
        for j in 0..4 {
            scales[j] = sc[j] & 63;
            scales[j + 4] = m[j] & 63;
        }
        for j in 4..8 {
            scales[j + 4] = (sc[j] & 0xF) | ((m[j] & 0xF) << 4);
            scales[j - 4] |= (sc[j] >> 4) << 6;
            scales[j] |= (m[j] >> 4) << 6;
        }
        out[bb + 4..bb + 16].copy_from_slice(&scales);
        // Quantize + pack nibbles using the reconstructed grid (x = dq*q - mq).
        for s in 0..8 {
            let dq = d * sc[s] as f32;
            let mq = dmin * m[s] as f32;
            let idq = if dq > 0.0 { 1.0 / dq } else { 0.0 };
            for l in 0..32 {
                let gi = base + s * 32 + l;
                let q = if gi < weights.len() {
                    ((weights[gi] + mq) * idq).round().clamp(0.0, 15.0) as u8
                } else {
                    0
                };
                let byte = bb + 16 + (s / 2) * 32 + l;
                if s % 2 == 0 {
                    out[byte] = (out[byte] & 0xF0) | (q & 0x0F);
                } else {
                    out[byte] = (out[byte] & 0x0F) | ((q & 0x0F) << 4);
                }
            }
        }
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

    #[test]
    fn q4_0_roundtrip_within_one_step() {
        let w: Vec<f32> = (0..32).map(|i| (i as f32 - 16.0) / 8.0).collect(); // ~[-2.0, 1.875]
        let mut bytes = vec![0u8; q4_0_bytes(w.len())];
        assert!(quantize_q4_0_from_f32(&w, &mut bytes));
        let info = crate::gguf_sharder::GgufTensorInfo {
            dims: [32, 1, 1, 1],
            n_dims: 1,
            ggml_type: crate::ggml_quants::GGML_TYPE_Q4_0,
            byte_offset: 0,
        };
        let mut back = vec![0f32; 32];
        let n = crate::ggml_quants::dequant_matrix_row_into(&bytes, &info, 0, &mut back).unwrap();
        assert_eq!(n, 32);
        let absmax = w.iter().cloned().fold(0f32, |m, x| m.max(x.abs())); // 2.0
        let step = absmax / 8.0; // ~0.25 (one Q4_0 level)
        assert!(
            max_abs_err(&w, &back) <= step * 1.1,
            "q4_0 roundtrip err {} > step {step}",
            max_abs_err(&w, &back)
        );
    }

    #[test]
    fn q4_k_roundtrip_and_beats_q4_0() {
        // 256-element super-block, zero-centred spread.
        let w: Vec<f32> = (0..256).map(|i| (i as f32 - 128.0) / 64.0).collect(); // ~[-2.0, 1.98]
        let info_k = crate::gguf_sharder::GgufTensorInfo {
            dims: [256, 1, 1, 1],
            n_dims: 1,
            ggml_type: crate::ggml_quants::GGML_TYPE_Q4_K,
            byte_offset: 0,
        };
        let mut kbytes = vec![0u8; q4_k_bytes(w.len())];
        assert!(quantize_q4_k_from_f32(&w, &mut kbytes));
        let mut back_k = vec![0f32; 256];
        let nk = crate::ggml_quants::dequant_matrix_row_into(&kbytes, &info_k, 0, &mut back_k).unwrap();
        assert_eq!(nk, 256);
        let err_k = max_abs_err(&w, &back_k);

        // Same data through Q4_0 — Q4_K's 6-bit sub-scales should be at least as accurate.
        let info_0 = crate::gguf_sharder::GgufTensorInfo {
            dims: [256, 1, 1, 1],
            n_dims: 1,
            ggml_type: crate::ggml_quants::GGML_TYPE_Q4_0,
            byte_offset: 0,
        };
        let mut zbytes = vec![0u8; q4_0_bytes(w.len())];
        assert!(quantize_q4_0_from_f32(&w, &mut zbytes));
        let mut back_0 = vec![0f32; 256];
        crate::ggml_quants::dequant_matrix_row_into(&zbytes, &info_0, 0, &mut back_0).unwrap();
        let err_0 = max_abs_err(&w, &back_0);

        let range = 2.0 + 1.98; // ~3.98
        eprintln!("q4_k err {err_k:.4} vs q4_0 err {err_0:.4} (range {range:.2})");
        assert!(err_k.is_finite() && err_k <= range / 15.0 * 1.3, "q4_k roundtrip err {err_k} too high");
        assert!(err_k <= err_0 + 1e-4, "q4_k ({err_k}) should not be worse than q4_0 ({err_0})");
    }
}
