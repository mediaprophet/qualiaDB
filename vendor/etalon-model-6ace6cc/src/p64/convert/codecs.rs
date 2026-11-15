pub const TRITS_PER_BYTE: usize = 5;

/// Engine element-type code for a BitNet-1.58b ternary tensor (well outside the GGML code range,
/// so it never collides with `F32=0 / F16=1 / Q8_0=8 / Q4_K=12 / BF16=30`). Mnemonic: "1.58b".
pub use crate::ggml::GGML_TYPE_TERNARY_158;

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
