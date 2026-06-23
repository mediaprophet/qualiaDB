//! BitNet b1.58 **ternary** quantization codec — STELLAR §A compression (task #12).
//!
//! Ternary packing of weights `∈ {-1, 0, +1}` with a per-tensor **absmean** scale (BitNet 1.58b):
//! it replaces fused multiply-adds with hardware adds/subtracts in the GEMM kernels and shrinks the
//! weights to ≈ **1.6 bits each**. This module is the reusable codec; it is applied *during*
//! transcode by [`crate::q42_weight::transcode_safetensor_to_q42_ternary`] so a `.q42` ships
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
    fn all_zero_and_empty_are_safe() {
        let (s, t) = quantize_ternary(&[0.0, 0.0, 0.0]);
        assert_eq!(s, 0.0);
        assert_eq!(t, vec![0, 0, 0]);
        let (s2, t2) = quantize_ternary(&[]);
        assert_eq!(s2, 0.0);
        assert!(t2.is_empty());
    }
}
