//! BitNet b1.58 **ternary** quantization codec — STELLAR §A compression (task #12).
//!
//! Ternary packing of weights `∈ {-1, 0, +1}` with a per-tensor **absmean** scale (BitNet 1.58b):
//! it replaces fused multiply-adds with hardware adds/subtracts in the GEMM kernels and shrinks the
//! weights to ≈ **1.6 bits each**. This module is the reusable codec; it is applied *during*
//! transcode by [`crate::p64_weight::transcode_safetensor_to_p64_ternary`] so a P64 image ships
//! compressed, not as a verbatim blob.
//!
//! ## Encoding
//! * **Quantize** (per tensor): `scale = mean(|w|)`; `t_i = clamp(round(w_i / scale), -1, +1)`.
//! * **Dequantize:** `w_i ≈ scale · t_i`.
//! * **Packing:** five trits per byte in base-3 (`3⁵ = 243 ≤ 256`) → `8 bits / 5 trits = 1.6
//!   bits/weight`. A trit `{-1,0,+1}` is offset to a digit `{0,1,2}`; a byte is
//!   `d₀ + 3·d₁ + 9·d₂ + 27·d₃ + 81·d₄`.
//!
//! The hot-path dequant ([`dequantize_ternary`]) is **zero-heap** (it streams base-3 digits straight
//! into the caller's `f32` buffer — no intermediate trit `Vec`). The encode path runs at ingest
//! (cold), where a working `Vec` is acceptable.

/// Trits packed per byte (`3⁵ = 243 ≤ 256`).
pub const TRITS_PER_BYTE: usize = 5;

/// Engine element-type code for a BitNet-1.58b ternary tensor (well outside the GGML code range,
/// so it never collides with `F32=0 / F16=1 / Q8_0=8 / Q4_K=12 / BF16=30`). Mnemonic: "1.58b".
pub const GGML_TYPE_TERNARY_158: u32 = 1158;

/// Bytes needed to pack `count` trits (5 per byte).
#[inline]
pub fn packed_trit_len(count: usize) -> usize {
    count.div_ceil(TRITS_PER_BYTE)
}

/// Total ternary-blob length for `count` weights: a 4-byte `f32` scale + the packed trits.
#[inline]
pub fn ternary_blob_len(count: usize) -> usize {
    4 + packed_trit_len(count)
}

/// BitNet 1.58b quantize: per-tensor absmean `scale` + ternary values `∈ {-1,0,+1}`.
/// A zero (or empty) tensor yields `scale = 0.0` and all-zero trits.
pub fn quantize_ternary(weights: &[f32]) -> (f32, Vec<i8>) {
    let n = weights.len();
    if n == 0 {
        return (0.0, Vec::new());
    }
    let absmean = weights.iter().map(|w| w.abs()).sum::<f32>() / n as f32;
    if absmean == 0.0 {
        return (0.0, vec![0i8; n]);
    }
    let trits = weights
        .iter()
        .map(|&w| (w / absmean).round().clamp(-1.0, 1.0) as i8)
        .collect();
    (absmean, trits)
}

/// Pack ternary values (`i8 ∈ {-1,0,+1}`) into bytes (5 trits/byte, base-3). The final partial group
/// is zero-padded (decodes back to the requested `count` via [`unpack_trits_into`]).
pub fn pack_trits(trits: &[i8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(packed_trit_len(trits.len()));
    for chunk in trits.chunks(TRITS_PER_BYTE) {
        let mut byte = 0u16;
        let mut mul = 1u16;
        for &t in chunk {
            byte += ((t + 1) as u16) * mul; // {-1,0,1} -> {0,1,2}
            mul *= 3;
        }
        out.push(byte as u8);
    }
    out
}

/// Unpack `out.len()` trits from packed bytes into the caller's buffer (zero-heap). Returns the
/// number written (`min(out.len(), packed capacity)`).
pub fn unpack_trits_into(packed: &[u8], out: &mut [i8]) -> usize {
    let mut i = 0;
    'outer: for &byte in packed {
        let mut b = byte;
        for _ in 0..TRITS_PER_BYTE {
            if i >= out.len() {
                break 'outer;
            }
            out[i] = (b % 3) as i8 - 1; // {0,1,2} -> {-1,0,1}
            b /= 3;
            i += 1;
        }
    }
    i
}

/// Dequantize packed ternary → `f32` weights (`scale · trit`) into `out`. **Zero-heap**: digits are
/// streamed straight from the bytes (no intermediate trit allocation). Writes `out.len()` values.
pub fn dequantize_ternary(scale: f32, packed: &[u8], out: &mut [f32]) {
    let mut i = 0;
    'outer: for &byte in packed {
        let mut b = byte;
        for _ in 0..TRITS_PER_BYTE {
            if i >= out.len() {
                break 'outer;
            }
            out[i] = scale * ((b % 3) as f32 - 1.0);
            b /= 3;
            i += 1;
        }
    }
}

/// Encode a weight tensor to a self-describing ternary blob: `[scale: f32 LE][packed trits]`.
/// (The element count is recovered from the tensor's shape in the container manifest.)
pub fn ternary_blob(weights: &[f32]) -> Vec<u8> {
    let (scale, trits) = quantize_ternary(weights);
    let mut out = Vec::with_capacity(ternary_blob_len(weights.len()));
    out.extend_from_slice(&scale.to_le_bytes());
    out.extend_from_slice(&pack_trits(&trits));
    out
}

// ── GPU ternary GEMM kernel + its CPU oracle ─────────────────────────────────────────────────────

/// The WGSL ternary-GEMM compute kernel (STELLAR §A). Its CPU parity reference is
/// [`ternary_gemm_cpu`], which mirrors it byte-for-byte (same trit extraction, add/subtract,
/// end-scale). The GPU pipeline binds: `0` activations (`f32`), `1` packed trits (`u32` words),
/// `2` `TernaryParams` uniform, `3` output (`f32`).
pub const TERNARY_GEMM_WGSL: &str = include_str!("../shaders/ternary_gemm.wgsl");

/// Extract the ternary value `{-1,0,+1}` at linear weight index `k` from packed trits — the exact
/// operation `ternary_gemm.wgsl::trit_at` performs (5 trits/byte, base-3).
#[inline]
pub fn trit_at(packed: &[u8], k: usize) -> i32 {
    let byte = packed[k / TRITS_PER_BYTE];
    let pos = k % TRITS_PER_BYTE;
    let mut b = byte;
    for _ in 0..pos {
        b /= 3;
    }
    (b % 3) as i32 - 1
}

/// **CPU oracle for `ternary_gemm.wgsl`.** Computes `out[m][i] = scale · Σ_j trit(W[i][j])·act[m][j]`
/// where `packed` holds the row-major trits of an `(n_out × n_in)` weight matrix. The weight
/// contributes by add/subtract only (the BitNet win); the per-tensor `scale` is applied once per
/// output element. Zero-heap. Strides default to dense (`n_in` / `n_out`) when `0`.
#[allow(clippy::too_many_arguments)]
pub fn ternary_gemm_cpu(
    activations: &[f32],
    packed: &[u8],
    scale: f32,
    n_in: usize,
    n_out: usize,
    n_batch: usize,
    in_row_stride: usize,
    out_row_stride: usize,
    out: &mut [f32],
) {
    let in_stride = if in_row_stride > 0 {
        in_row_stride
    } else {
        n_in
    };
    let out_stride = if out_row_stride > 0 {
        out_row_stride
    } else {
        n_out
    };
    for m in 0..n_batch.max(1) {
        let in_base = m * in_stride;
        for i in 0..n_out {
            let row0 = i * n_in;
            let mut acc = 0.0f32;
            for j in 0..n_in {
                let x = activations[in_base + j];
                match trit_at(packed, row0 + j) {
                    t if t > 0 => acc += x,
                    t if t < 0 => acc -= x,
                    _ => {}
                }
            }
            out[m * out_stride + i] = scale * acc;
        }
    }
}

// ── 2-bit (pow-2) packing + branchless GEMM — GPU-optimal (external-review-driven) ───────────────
//
// Base-3 packing (above) is densest (1.6 bit) but the GPU kernel must unpack it with integer `/3`
// and `%3` — dozens of cycles on Ampere — and a `trit>0/<0` branch causes warp divergence. The
// 2-bit layout below trades 25% more bandwidth (2.0 bit) for **shift/mask unpack + fully branchless
// math**. On a GPU the per-weight multiply is free (FMA), so the ternary win is *bandwidth +
// occupancy*, not MAC-elimination — making this the right layout for the GPU resident path. (Base-3
// remains the better on-disk/distribution format; the two can coexist — base-3 cold, 2-bit hot.)

/// Trits packed 4-per-byte, 2 bits each: `0b00 = 0`, `0b01 = +1`, `0b10 = -1` (`0b11` unused).
pub const TRITS_PER_BYTE_2BIT: usize = 4;

/// The 2-bit code for a trit (matches `ternary_gemm_2bit.wgsl`).
#[inline]
fn trit_code_2bit(t: i8) -> u8 {
    if t > 0 {
        1
    } else if t < 0 {
        2
    } else {
        0
    }
}

/// Bytes to pack `count` trits at 2 bits each (4/byte).
#[inline]
pub fn packed_trit_len_2bit(count: usize) -> usize {
    count.div_ceil(TRITS_PER_BYTE_2BIT)
}

/// Pack ternary values into 2-bit codes, 4 per byte.
pub fn pack_trits_2bit(trits: &[i8]) -> Vec<u8> {
    let mut out = vec![0u8; packed_trit_len_2bit(trits.len())];
    for (k, &t) in trits.iter().enumerate() {
        out[k / TRITS_PER_BYTE_2BIT] |= trit_code_2bit(t) << ((k % TRITS_PER_BYTE_2BIT) * 2);
    }
    out
}

/// Trit value `{-1,0,+1}` at linear index `k` from 2-bit packing — the **branchless** mirror of
/// `ternary_gemm_2bit.wgsl::pair_at` (`(code==1) - (code==2)`).
#[inline]
pub fn trit_at_2bit(packed: &[u8], k: usize) -> i32 {
    let code = (packed[k / TRITS_PER_BYTE_2BIT] >> ((k % TRITS_PER_BYTE_2BIT) * 2)) & 3;
    (code == 1) as i32 - (code == 2) as i32
}

/// The branchless 2-bit WGSL ternary-GEMM kernel; CPU oracle is [`ternary_gemm_cpu_2bit`].
pub const TERNARY_GEMM_2BIT_WGSL: &str = include_str!("../shaders/ternary_gemm_2bit.wgsl");

/// CPU oracle for `ternary_gemm_2bit.wgsl` — same math as [`ternary_gemm_cpu`], 2-bit packing +
/// branchless accumulation.
#[allow(clippy::too_many_arguments)]
pub fn ternary_gemm_cpu_2bit(
    activations: &[f32],
    packed: &[u8],
    scale: f32,
    n_in: usize,
    n_out: usize,
    n_batch: usize,
    in_row_stride: usize,
    out_row_stride: usize,
    out: &mut [f32],
) {
    let in_stride = if in_row_stride > 0 {
        in_row_stride
    } else {
        n_in
    };
    let out_stride = if out_row_stride > 0 {
        out_row_stride
    } else {
        n_out
    };
    for m in 0..n_batch.max(1) {
        let in_base = m * in_stride;
        for i in 0..n_out {
            let row0 = i * n_in;
            let mut acc = 0.0f32;
            for j in 0..n_in {
                // branchless: trit ∈ {-1,0,+1} as f32, then FMA
                acc += trit_at_2bit(packed, row0 + j) as f32 * activations[in_base + j];
            }
            out[m * out_stride + i] = scale * acc;
        }
    }
}

/// Decode a [`ternary_blob`] of `count` weights into `out` (zero-heap dequant).
pub fn dequantize_blob(blob: &[u8], out: &mut [f32]) {
    if blob.len() < 4 {
        for o in out.iter_mut() {
            *o = 0.0;
        }
        return;
    }
    let scale = f32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]);
    dequantize_ternary(scale, &blob[4..], out);
}

/// Rebake an on-disk base-3 [`ternary_blob`] (`[scale f32 LE][5-trits/byte]`) into the runtime
/// **2-bit branchless** VRAM layout consumed by [`ternary_gemm_2bit.wgsl`] / [`ternary_gemm_cpu_2bit`].
///
/// D1 (STELLAR §A, measured on A2000): base-3 is the *archive/distribution* layout (1.6 bit, densest)
/// but on the GPU its `/3`,`%3` unpack makes it **0.85× — slower than F16**; the 2-bit branchless layout
/// (2.0 bit, shift/mask, divergence-free) is the **1.77×** win. So the live FFN-ternary path rebakes each
/// base-3 FFN blob to 2-bit **once at resident load** (heap is the sanctioned load-time path; the hot
/// loop stays zero-heap). `count` = the tensor's element count (from the manifest shape). Returns
/// `(scale, packed_2bit)`; the dequantized values are bit-identical to the base-3 source.
pub fn rebake_ternary_blob_to_2bit(blob: &[u8], count: usize) -> (f32, Vec<u8>) {
    if blob.len() < 4 || count == 0 {
        return (0.0, Vec::new());
    }
    let scale = f32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]);
    let mut trits = vec![0i8; count];
    unpack_trits_into(&blob[4..], &mut trits);
    (scale, pack_trits_2bit(&trits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantize_absmean_and_trits() {
        // absmean = (2 + 2 + 0.1 + 0.1 + 0)/5 = 0.84
        let w = [2.0_f32, -2.0, 0.1, -0.1, 0.0];
        let (scale, trits) = quantize_ternary(&w);
        assert!((scale - 0.84).abs() < 1e-5, "scale {scale}");
        assert_eq!(trits, vec![1, -1, 0, 0, 0]);
    }

    #[test]
    fn pack_unpack_round_trips_exactly() {
        let trits: Vec<i8> = [1, -1, 0, 0, 0, 1, 1, -1, 0, 1, -1]
            .iter()
            .copied()
            .collect();
        let packed = pack_trits(&trits);
        assert_eq!(packed.len(), packed_trit_len(trits.len())); // 11 trits -> 3 bytes
        let mut back = vec![0i8; trits.len()];
        let n = unpack_trits_into(&packed, &mut back);
        assert_eq!(n, trits.len());
        assert_eq!(back, trits);
        // the known base-3 byte for [1,-1,0,0,0] = digits [2,0,1,1,1] = 2+9+27+81 = 119
        assert_eq!(packed[0], 119);
    }

    #[test]
    fn rebake_base3_to_2bit_preserves_gemm() {
        // The on-disk base-3 blob rebaked to the 2-bit runtime layout must yield bit-identical trits,
        // the same scale, and a byte-identical GEMM (A1b: this conversion is the make-or-break design
        // fact — base-3 on GPU is slower than F16; 2-bit is the win, and it must be lossless).
        let (n_out, n_in) = (3usize, 8usize);
        let weights: Vec<f32> = (0..n_out * n_in).map(|i| (i as f32 * 0.37).sin()).collect();
        let act: Vec<f32> = (0..n_in).map(|i| i as f32 * 0.5 - 1.0).collect();

        let base3 = ternary_blob(&weights);
        let base3_scale = f32::from_le_bytes([base3[0], base3[1], base3[2], base3[3]]);
        let base3_packed = &base3[4..];

        let (scale2, packed2) = rebake_ternary_blob_to_2bit(&base3, weights.len());
        assert_eq!(scale2, base3_scale, "scale must be preserved");
        for k in 0..weights.len() {
            assert_eq!(
                trit_at(base3_packed, k),
                trit_at_2bit(&packed2, k),
                "trit {k} mismatch"
            );
        }

        let mut out_base3 = vec![0f32; n_out];
        let mut out_2bit = vec![0f32; n_out];
        ternary_gemm_cpu(
            &act,
            base3_packed,
            base3_scale,
            n_in,
            n_out,
            1,
            0,
            0,
            &mut out_base3,
        );
        ternary_gemm_cpu_2bit(&act, &packed2, scale2, n_in, n_out, 1, 0, 0, &mut out_2bit);
        for i in 0..n_out {
            assert!(
                (out_base3[i] - out_2bit[i]).abs() < 1e-6,
                "row {i}: base3 {} vs 2bit {}",
                out_base3[i],
                out_2bit[i]
            );
        }
    }

    #[test]
    fn blob_round_trips_with_scale() {
        let w = [2.0_f32, -2.0, 0.1, -0.1, 0.0, 1.5, -1.5];
        let blob = ternary_blob(&w);
        assert_eq!(blob.len(), ternary_blob_len(w.len()));
        let mut out = vec![0.0_f32; w.len()];
        dequantize_blob(&blob, &mut out);
        // reconstruction is scale * trit; for the strong weights it recovers ±scale.
        let (scale, _) = quantize_ternary(&w);
        assert!((out[0] - scale).abs() < 1e-5);
        assert!((out[1] + scale).abs() < 1e-5);
        assert_eq!(out[4], 0.0); // a near-zero weight quantizes to 0
    }

    #[test]
    fn compression_ratio_is_about_1_6_bits_per_weight() {
        let count = 4096;
        let f32_bytes = count * 4;
        let ternary_bytes = ternary_blob_len(count); // 4 + ceil(4096/5) = 4 + 820 = 824
                                                     // ~1.6 bits/weight => ~20x smaller than f32, ~10x smaller than f16.
        let ratio = f32_bytes as f64 / ternary_bytes as f64;
        assert!(ratio > 19.0 && ratio < 21.0, "ratio {ratio}");
        let bits_per_weight = (ternary_bytes as f64 * 8.0) / count as f64;
        assert!(bits_per_weight < 1.7, "bits/weight {bits_per_weight}");
    }

    #[test]
    fn ternary_gemm_cpu_matches_hand_computation() {
        // W (n_out=2 × n_in=3) trits row-major: [[1,-1,0],[0,1,1]]; scale = 2.0; act = [1,2,3].
        let trits: [i8; 6] = [1, -1, 0, 0, 1, 1];
        let packed = pack_trits(&trits);
        let act = [1.0_f32, 2.0, 3.0];
        let mut out = [0.0_f32; 2];
        ternary_gemm_cpu(&act, &packed, 2.0, 3, 2, 1, 0, 0, &mut out);
        // out[0] = 2*(1*1 + -1*2 + 0*3) = -2 ; out[1] = 2*(0*1 + 1*2 + 1*3) = 10
        assert_eq!(out, [-2.0, 10.0]);
    }

    #[test]
    fn ternary_gemm_equals_dense_matmul_of_dequantized_weights() {
        // ternary GEMM must equal a plain f32 matmul over the dequantized weights (scale·trit),
        // since scale·Σ trit·x == Σ (scale·trit)·x.
        let (n_in, n_out) = (7usize, 5usize);
        let scale = 0.37_f32;
        // arbitrary deterministic trits
        let trits: Vec<i8> = (0..n_in * n_out).map(|k| (k % 3) as i8 - 1).collect();
        let packed = pack_trits(&trits);
        let act: Vec<f32> = (0..n_in).map(|j| (j as f32) * 0.5 - 1.0).collect();

        let mut got = vec![0.0_f32; n_out];
        ternary_gemm_cpu(&act, &packed, scale, n_in, n_out, 1, 0, 0, &mut got);

        for i in 0..n_out {
            let mut dense = 0.0_f32;
            for j in 0..n_in {
                let w = scale * trits[i * n_in + j] as f32; // dequantized weight
                dense += w * act[j];
            }
            assert!(
                (got[i] - dense).abs() < 1e-5,
                "row {i}: {} vs {}",
                got[i],
                dense
            );
        }
    }

    #[test]
    fn ternary_gemm_wgsl_parses() {
        // naga parse-smoke (same gate render::contract uses for the viewport shaders): catches
        // syntax/type regressions in the kernel on native CI. GPU pipeline validation is the
        // wasm `portal`/`wasm-full` build when the kernel is wired into a dispatch.
        naga::front::wgsl::Frontend::new()
            .parse(TERNARY_GEMM_WGSL)
            .unwrap_or_else(|e| panic!("ternary_gemm.wgsl parse failed: {e:?}"));
    }

    #[test]
    fn all_zero_and_empty_are_safe() {
        let (s, t) = quantize_ternary(&[0.0, 0.0, 0.0]);
        assert_eq!(s, 0.0);
        assert_eq!(t, vec![0, 0, 0]);
        let (s2, t2) = quantize_ternary(&[]);
        assert_eq!(s2, 0.0);
        assert!(t2.is_empty());
    }
}
