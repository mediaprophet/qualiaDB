//! `dct2` — orthonormal Discrete Cosine Transform, type II (the cepstral kernel).

use crate::types::AudioError;

/// Orthonormal DCT-II of `input` into `out` (same length `N`).
///
/// ```text
/// X[0]   = sqrt(1/N) * Σ_n x[n]
/// X[k>0] = sqrt(2/N) * Σ_n x[n] * cos(π/N * (n + 1/2) * k)
/// ```
///
/// This is the `norm="ortho"` convention (matches SciPy `dct(type=2, norm="ortho")`),
/// so the transform is its own inverse under DCT-III and preserves energy. Runs in
/// `O(N²)`; for the small `N` (mel/bark band counts, ≤ ~128) this is the right,
/// allocation-free choice.
///
/// Returns [`AudioError::InvalidParameter`] if `input` is empty and
/// [`AudioError::OutputBufferTooSmall`] if `out` is shorter than `input`.
pub fn dct2(input: &[f32], out: &mut [f32]) -> Result<(), AudioError> {
    let n = input.len();
    if n == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if out.len() < n {
        return Err(AudioError::OutputBufferTooSmall);
    }

    let inv_n = 1.0f32 / n as f32;
    let scale0 = inv_n.sqrt();
    let scale_k = (2.0 * inv_n).sqrt();
    let pi_over_n = core::f32::consts::PI * inv_n;

    for k in 0..n {
        let mut acc = 0.0f32;
        for (nn, &x) in input.iter().enumerate() {
            acc += x * (pi_over_n * (nn as f32 + 0.5) * k as f32).cos();
        }
        out[k] = if k == 0 { scale0 * acc } else { scale_k * acc };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dct2_matches_closed_form_reference() {
        // Reference: scipy.fft.dct([1,2,3,4], type=2, norm='ortho')
        //          = [5.0, -2.23044, 0.0, -0.15851]
        let input = [1.0f32, 2.0, 3.0, 4.0];
        let mut out = [0.0f32; 4];
        dct2(&input, &mut out).unwrap();
        let expected = [5.0f32, -2.23044, 0.0, -0.15851];
        for (i, (&g, &e)) in out.iter().zip(expected.iter()).enumerate() {
            assert!((g - e).abs() < 1e-3, "coeff {i}: got {g}, want {e}");
        }
    }

    #[test]
    fn dct2_of_constant_is_pure_dc() {
        let input = [3.0f32; 8];
        let mut out = [0.0f32; 8];
        dct2(&input, &mut out).unwrap();
        // c0 = sqrt(1/8)*24 = sqrt(8)*3 ≈ 8.485; all AC ≈ 0.
        assert!((out[0] - (8.0f32).sqrt() * 3.0).abs() < 1e-3);
        for &c in &out[1..] {
            assert!(c.abs() < 1e-4, "AC term nonzero: {c}");
        }
    }

    #[test]
    fn dct2_preserves_energy() {
        // Orthonormal ⇒ ||X||² == ||x||².
        let input = [0.5f32, -1.0, 2.0, 0.25, -0.75, 1.5];
        let mut out = [0.0f32; 6];
        dct2(&input, &mut out).unwrap();
        let e_in: f32 = input.iter().map(|v| v * v).sum();
        let e_out: f32 = out.iter().map(|v| v * v).sum();
        assert!((e_in - e_out).abs() < 1e-3, "energy {e_in} vs {e_out}");
    }

    #[test]
    fn rejects_short_out() {
        let input = [1.0f32, 2.0, 3.0];
        let mut out = [0.0f32; 2];
        assert_eq!(
            dct2(&input, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
