//! Finite Scalar Quantization (FSQ) — the codebook-free discrete token structure from
//! "Finite Scalar Quantization: VQ-VAE Made Simple" (Mentzer et al., 2023). Each latent
//! dimension is independently rounded to one of `L` evenly-spaced levels spanning a bounded
//! range `[lo, hi]`; the implicit codebook is the Cartesian product of the per-dim level grids,
//! so no codebook needs to be stored or learned. This is pure quantizer math — real and
//! buildable now. The neural encoder that produces the (bounded) latents is a separate,
//! `NeedsWeights` concern.
//!
//! Per-dimension level counts may differ (e.g. `[8, 5, 5, 5]`), matching the FSQ convention.
//! On-grid values (`lo + k·step`) round-trip exactly; off-grid values snap to the nearest level.
//! Zero-heap: caller supplies the level table and all buffers.

use crate::types::AudioError;

/// Quantize each component of `input` to its per-dimension level grid, writing level indices.
///
/// - `input`: at least `levels.len()` floats.
/// - `levels`: per-dimension level counts `L_d` (each `≥ 1`); `dim = levels.len()`.
/// - `lo`, `hi`: shared inclusive bounds for the grid (`hi > lo`).
/// - `out_tokens`: at least `dim` slots; slot `d` receives the level index in `0..L_d`.
///
/// For dim `d` with `L` levels the step is `(hi - lo)/(L - 1)`; the index is
/// `round((x - lo)/step)` clamped to `0..=L-1`. A degenerate `L == 1` collapses to index 0.
/// Returns the number of tokens written (`dim`). Zero-heap.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `levels` is empty, any `L_d == 0`, or `hi <= lo`.
/// - [`AudioError::OutputBufferTooSmall`] if `input` or `out_tokens` are shorter than `dim`.
pub fn fsq_quantize(
    input: &[f32],
    levels: &[u16],
    lo: f32,
    hi: f32,
    out_tokens: &mut [u16],
) -> Result<usize, AudioError> {
    let dim = levels.len();
    if dim == 0 || hi <= lo || levels.contains(&0) {
        return Err(AudioError::InvalidParameter);
    }
    if input.len() < dim || out_tokens.len() < dim {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for d in 0..dim {
        let l = levels[d];
        if l == 1 {
            out_tokens[d] = 0;
            continue;
        }
        let step = (hi - lo) / ((l - 1) as f32);
        // Nearest level index, clamped to the valid range.
        let raw = ((input[d] - lo) / step).round();
        let idx = if raw < 0.0 {
            0.0
        } else if raw > (l - 1) as f32 {
            (l - 1) as f32
        } else {
            raw
        };
        out_tokens[d] = idx as u16;
    }
    Ok(dim)
}

/// Reconstruct latent components from FSQ level indices: `out[d] = lo + tokens[d]·step_d`.
///
/// - `tokens`: at least `dim` level indices (each clamped into `0..L_d`).
/// - `levels`, `lo`, `hi`: same as [`fsq_quantize`].
/// - `out`: at least `dim` floats; fully overwritten.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `levels` is empty, any `L_d == 0`, or `hi <= lo`.
/// - [`AudioError::OutputBufferTooSmall`] if `tokens` or `out` are shorter than `dim`.
pub fn fsq_dequantize(
    tokens: &[u16],
    levels: &[u16],
    lo: f32,
    hi: f32,
    out: &mut [f32],
) -> Result<(), AudioError> {
    let dim = levels.len();
    if dim == 0 || hi <= lo || levels.contains(&0) {
        return Err(AudioError::InvalidParameter);
    }
    if tokens.len() < dim || out.len() < dim {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for d in 0..dim {
        let l = levels[d];
        if l == 1 {
            out[d] = lo;
            continue;
        }
        let step = (hi - lo) / ((l - 1) as f32);
        let idx = tokens[d].min(l - 1);
        out[d] = lo + (idx as f32) * step;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_grid_values_round_trip_exactly() {
        // dim 3, levels [5,5,5] over [-1,1] -> step 0.5, grid {-1,-0.5,0,0.5,1}.
        let levels = [5u16, 5, 5];
        let input = [-1.0f32, 0.0, 0.5]; // all on-grid
        let mut tokens = [0u16; 3];
        fsq_quantize(&input, &levels, -1.0, 1.0, &mut tokens).expect("quant");
        assert_eq!(tokens, [0, 2, 3]); // indices for -1, 0, 0.5

        let mut recon = [0.0f32; 3];
        fsq_dequantize(&tokens, &levels, -1.0, 1.0, &mut recon).expect("dequant");
        assert_eq!(recon, input, "on-grid values must reconstruct bit-exactly");
    }

    #[test]
    fn off_grid_snaps_to_nearest_level() {
        // step 0.5; 0.24 -> nearest level 0 (0.0), 0.26 -> level 0.5.
        let levels = [5u16];
        let mut tokens = [0u16; 1];

        fsq_quantize(&[0.24f32], &levels, -1.0, 1.0, &mut tokens).expect("q");
        assert_eq!(tokens[0], 2); // 0.0
        let mut r = [0.0f32; 1];
        fsq_dequantize(&tokens, &levels, -1.0, 1.0, &mut r).expect("d");
        assert!((r[0] - 0.0).abs() < 1e-6, "0.24 -> {}", r[0]);

        fsq_quantize(&[0.26f32], &levels, -1.0, 1.0, &mut tokens).expect("q");
        assert_eq!(tokens[0], 3); // 0.5
        fsq_dequantize(&tokens, &levels, -1.0, 1.0, &mut r).expect("d");
        assert!((r[0] - 0.5).abs() < 1e-6, "0.26 -> {}", r[0]);
    }

    #[test]
    fn out_of_range_input_clamps_to_end_levels() {
        let levels = [4u16];
        let mut tokens = [0u16; 1];
        fsq_quantize(&[10.0f32], &levels, -1.0, 1.0, &mut tokens).expect("q");
        assert_eq!(tokens[0], 3); // top level
        fsq_quantize(&[-10.0f32], &levels, -1.0, 1.0, &mut tokens).expect("q");
        assert_eq!(tokens[0], 0); // bottom level
    }

    #[test]
    fn per_dim_level_counts() {
        // Mixed levels [8,2]; check both dims quantize independently on their own grid.
        let levels = [8u16, 2];
        let input = [1.0f32, -1.0]; // top of dim0 grid, bottom of dim1 grid
        let mut tokens = [0u16; 2];
        fsq_quantize(&input, &levels, -1.0, 1.0, &mut tokens).expect("q");
        assert_eq!(tokens, [7, 0]);
        let mut recon = [0.0f32; 2];
        fsq_dequantize(&tokens, &levels, -1.0, 1.0, &mut recon).expect("d");
        assert_eq!(recon, input);
    }

    #[test]
    fn single_level_dim_collapses_to_lo() {
        let levels = [1u16];
        let mut tokens = [99u16; 1];
        fsq_quantize(&[0.7f32], &levels, -1.0, 1.0, &mut tokens).expect("q");
        assert_eq!(tokens[0], 0);
        let mut r = [0.0f32; 1];
        fsq_dequantize(&tokens, &levels, -1.0, 1.0, &mut r).expect("d");
        assert_eq!(r[0], -1.0);
    }

    #[test]
    fn rejects_bad_params() {
        let levels = [5u16];
        let mut tokens = [0u16; 1];
        assert_eq!(
            fsq_quantize(&[0.0], &[], 0.0, 1.0, &mut tokens),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            fsq_quantize(&[0.0], &levels, 1.0, 1.0, &mut tokens),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            fsq_quantize(&[0.0], &[0u16], -1.0, 1.0, &mut tokens),
            Err(AudioError::InvalidParameter)
        );
        let mut empty: [u16; 0] = [];
        assert_eq!(
            fsq_quantize(&[0.0], &levels, -1.0, 1.0, &mut empty),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
