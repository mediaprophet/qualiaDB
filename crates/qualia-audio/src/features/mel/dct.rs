//! `dct2` — orthonormal Discrete Cosine Transform, type II (the cepstral kernel),
//! delegated to [`etalon_signal::loudness::dct_ii_into`].

use crate::types::AudioError;

/// Orthonormal DCT-II of `input` into `out` (same length `N`).
pub fn dct2(input: &[f32], out: &mut [f32]) -> Result<(), AudioError> {
    let n = input.len();
    if n == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if out.len() < n {
        return Err(AudioError::OutputBufferTooSmall);
    }
    etalon_signal::loudness::dct_ii_into(input, out);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dct2_matches_closed_form_reference() {
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
        assert!((out[0] - (8.0f32).sqrt() * 3.0).abs() < 1e-3);
        for &c in &out[1..] {
            assert!(c.abs() < 1e-4, "AC term nonzero: {c}");
        }
    }

    #[test]
    fn dct2_preserves_energy() {
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
