//! Residual Vector Quantizer (RVQ) — the codec **structure** used by discrete audio token
//! models (EnCodec / SoundStream / DAC family). This is pure codebook math: no learned weights
//! are required, so it is real and buildable now. The caller supplies the codebooks (each stage
//! a `[n_centroids × dim]` row-major block); the neural encoder/decoder that *produces* those
//! codebooks and the latent vectors is a separate, `NeedsWeights` concern (out of scope here).
//!
//! Encoding is greedy per stage: at stage `s` we pick the centroid nearest (squared L2) to the
//! *running residual* — the input minus the centroids already chosen at stages `0..s`. Decoding
//! sums the chosen centroids back. Zero-heap: no scratch is allocated; the running residual is
//! recomputed on the fly from the already-chosen tokens (stage/dim counts are small).

use crate::types::AudioError;

/// Row-major offset of `centroid`'s `d`-th component inside `stage`'s codebook block.
#[inline]
fn cb_index(stage: usize, centroid: usize, d: usize, n_centroids: usize, dim: usize) -> usize {
    ((stage * n_centroids) + centroid) * dim + d
}

/// Value of `input[d]` minus the reconstruction from tokens chosen at stages `0..up_to`.
#[inline]
fn residual_component(
    input: &[f32],
    codebooks: &[f32],
    tokens: &[u16],
    up_to: usize,
    d: usize,
    n_centroids: usize,
    dim: usize,
) -> f32 {
    let mut r = input[d];
    for p in 0..up_to {
        let c = tokens[p] as usize;
        r -= codebooks[cb_index(p, c, d, n_centroids, dim)];
    }
    r
}

/// Encode one `dim`-vector into `n_stages` residual token indices.
///
/// - `input`: at least `dim` floats (one latent vector).
/// - `codebooks`: `n_stages × n_centroids × dim` floats, row-major, stage-major:
///   stage `s`, centroid `c`, component `d` lives at `((s*n_centroids)+c)*dim + d`.
/// - `out_tokens`: at least `n_stages` slots; slot `s` receives the chosen centroid index
///   (`< n_centroids`, hence `≤ 65535`).
///
/// Returns the number of tokens written (`n_stages`). Zero-heap.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if any of `n_stages`, `n_centroids`, `dim` is 0, or
///   `n_centroids > 65536` (index would not fit `u16`).
/// - [`AudioError::MalformedAudio`] if `input` is shorter than `dim` or `codebooks` is shorter
///   than `n_stages * n_centroids * dim`.
/// - [`AudioError::OutputBufferTooSmall`] if `out_tokens` has fewer than `n_stages` slots.
pub fn rvq_quantize(
    input: &[f32],
    codebooks: &[f32],
    n_stages: usize,
    n_centroids: usize,
    dim: usize,
    out_tokens: &mut [u16],
) -> Result<usize, AudioError> {
    if n_stages == 0 || n_centroids == 0 || dim == 0 || n_centroids > 65_536 {
        return Err(AudioError::InvalidParameter);
    }
    if input.len() < dim || codebooks.len() < n_stages * n_centroids * dim {
        return Err(AudioError::MalformedAudio);
    }
    if out_tokens.len() < n_stages {
        return Err(AudioError::OutputBufferTooSmall);
    }

    for s in 0..n_stages {
        let mut best_c = 0usize;
        let mut best_dist = f32::INFINITY;
        for c in 0..n_centroids {
            let mut dist = 0.0f32;
            for d in 0..dim {
                let r = residual_component(input, codebooks, out_tokens, s, d, n_centroids, dim);
                let diff = r - codebooks[cb_index(s, c, d, n_centroids, dim)];
                dist += diff * diff;
            }
            if dist < best_dist {
                best_dist = dist;
                best_c = c;
            }
        }
        out_tokens[s] = best_c as u16;
    }
    Ok(n_stages)
}

/// Decode `n_stages` residual token indices back into a `dim`-vector by summing the chosen
/// centroids across all stages.
///
/// - `tokens`: at least `n_stages` centroid indices (each `< n_centroids`).
/// - `codebooks`: same layout as [`rvq_quantize`].
/// - `out`: at least `dim` floats; fully overwritten.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if any of `n_stages`, `n_centroids`, `dim` is 0.
/// - [`AudioError::MalformedAudio`] if `codebooks` is short, or a token indexes `≥ n_centroids`.
/// - [`AudioError::OutputBufferTooSmall`] if `tokens` or `out` are too short.
pub fn rvq_dequantize(
    tokens: &[u16],
    codebooks: &[f32],
    n_stages: usize,
    n_centroids: usize,
    dim: usize,
    out: &mut [f32],
) -> Result<(), AudioError> {
    if n_stages == 0 || n_centroids == 0 || dim == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if codebooks.len() < n_stages * n_centroids * dim {
        return Err(AudioError::MalformedAudio);
    }
    if tokens.len() < n_stages || out.len() < dim {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for d in 0..dim {
        out[d] = 0.0;
    }
    for (s, &tok) in tokens.iter().enumerate().take(n_stages) {
        let c = tok as usize;
        if c >= n_centroids {
            return Err(AudioError::MalformedAudio);
        }
        for d in 0..dim {
            out[d] += codebooks[cb_index(s, c, d, n_centroids, dim)];
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two stages, 2 centroids each, dim = 2.
    // Stage 0 codebook: c0 = (0,0), c1 = (1,1)  -> coarse
    // Stage 1 codebook: c0 = (-0.1,0.1), c1 = (0.1,-0.1) -> fine residual
    fn small_codebooks() -> (Vec<f32>, usize, usize, usize) {
        let cb = vec![
            // stage 0
            0.0, 0.0, // c0
            1.0, 1.0, // c1
            // stage 1
            -0.1, 0.1, // c0
            0.1, -0.1, // c1
        ];
        (cb, 2, 2, 2) // codebooks, n_stages, n_centroids, dim
    }

    #[test]
    fn centroid_sum_round_trips_exactly() {
        // Input exactly equal to c1(stage0) + c1(stage1) = (1.1, 0.9).
        let (cb, n_stages, n_centroids, dim) = small_codebooks();
        let input = [1.1f32, 0.9];
        let mut tokens = [0u16; 2];
        let n = rvq_quantize(&input, &cb, n_stages, n_centroids, dim, &mut tokens).expect("quant");
        assert_eq!(n, 2);
        assert_eq!(tokens, [1, 1], "should pick coarse c1 then fine c1");

        let mut recon = [0.0f32; 2];
        rvq_dequantize(&tokens, &cb, n_stages, n_centroids, dim, &mut recon).expect("dequant");
        assert!((recon[0] - 1.1).abs() < 1e-6, "recon0={}", recon[0]);
        assert!((recon[1] - 0.9).abs() < 1e-6, "recon1={}", recon[1]);
    }

    #[test]
    fn reconstruction_within_codebook_resolution() {
        // Arbitrary input near the (1,1) coarse cell; error must be within the fine step.
        let (cb, n_stages, n_centroids, dim) = small_codebooks();
        let input = [1.08f32, 0.92];
        let mut tokens = [0u16; 2];
        rvq_quantize(&input, &cb, n_stages, n_centroids, dim, &mut tokens).expect("quant");
        let mut recon = [0.0f32; 2];
        rvq_dequantize(&tokens, &cb, n_stages, n_centroids, dim, &mut recon).expect("dequant");
        let err = ((recon[0] - input[0]).powi(2) + (recon[1] - input[1]).powi(2)).sqrt();
        // Fine centroids have magnitude ~0.14; residual after two stages stays below it.
        assert!(
            err < 0.15,
            "reconstruction error {err} exceeded resolution bound"
        );
    }

    #[test]
    fn adding_stages_does_not_increase_error() {
        // One-stage reconstruction error >= two-stage error (greedy residual refinement).
        let (cb, _n_stages, n_centroids, dim) = small_codebooks();
        let input = [0.9f32, 1.05];
        let mut t1 = [0u16; 1];
        rvq_quantize(&input, &cb, 1, n_centroids, dim, &mut t1).expect("q1");
        let mut r1 = [0.0f32; 2];
        rvq_dequantize(&t1, &cb, 1, n_centroids, dim, &mut r1).expect("d1");
        let e1 = ((r1[0] - input[0]).powi(2) + (r1[1] - input[1]).powi(2)).sqrt();

        let mut t2 = [0u16; 2];
        rvq_quantize(&input, &cb, 2, n_centroids, dim, &mut t2).expect("q2");
        let mut r2 = [0.0f32; 2];
        rvq_dequantize(&t2, &cb, 2, n_centroids, dim, &mut r2).expect("d2");
        let e2 = ((r2[0] - input[0]).powi(2) + (r2[1] - input[1]).powi(2)).sqrt();

        assert!(
            e2 <= e1 + 1e-6,
            "two-stage error {e2} worse than one-stage {e1}"
        );
    }

    #[test]
    fn rejects_bad_shapes() {
        let (cb, n_stages, n_centroids, dim) = small_codebooks();
        let input = [0.0f32; 2];
        let mut tokens = [0u16; 2];
        assert_eq!(
            rvq_quantize(&input, &cb, 0, n_centroids, dim, &mut tokens),
            Err(AudioError::InvalidParameter)
        );
        // input too short
        let short = [0.0f32; 1];
        assert_eq!(
            rvq_quantize(&short, &cb, n_stages, n_centroids, dim, &mut tokens),
            Err(AudioError::MalformedAudio)
        );
        // out_tokens too small
        let mut one = [0u16; 1];
        assert_eq!(
            rvq_quantize(&input, &cb, n_stages, n_centroids, dim, &mut one),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn dequantize_rejects_out_of_range_token() {
        let (cb, n_stages, n_centroids, dim) = small_codebooks();
        let bad = [5u16, 0];
        let mut out = [0.0f32; 2];
        assert_eq!(
            rvq_dequantize(&bad, &cb, n_stages, n_centroids, dim, &mut out),
            Err(AudioError::MalformedAudio)
        );
    }
}
