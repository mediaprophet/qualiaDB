//! Stereo panning coefficient from a left/right pair (energy ratio).

use crate::types::AudioError;

/// Panning coefficient of a stereo pair from its channel energy balance.
///
/// `left` and `right` are matching-length slices — either time-domain samples
/// or magnitude-spectrum bins of the two channels. With
/// `E_L = Σ left[i]^2` and `E_R = Σ right[i]^2`, the coefficient is
///
/// ```text
/// pan = (E_R - E_L) / (E_R + E_L)
/// ```
///
/// It lies in `[-1, 1]`: `-1` = hard left, `0` = centred (equal energy),
/// `+1` = hard right.
///
/// Zero-heap: single pass, scalar result.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if the slices are empty, differ in
///   length, or carry no energy at all (silence — panning is undefined).
pub fn panning(left: &[f32], right: &[f32]) -> Result<f32, AudioError> {
    if left.is_empty() || left.len() != right.len() {
        return Err(AudioError::InvalidParameter);
    }
    let mut el = 0.0f64;
    let mut er = 0.0f64;
    for i in 0..left.len() {
        el += left[i] as f64 * left[i] as f64;
        er += right[i] as f64 * right[i] as f64;
    }
    let total = el + er;
    if total == 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    Ok(((er - el) / total) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centred_is_zero() {
        let l = [1.0f32, -1.0, 0.5];
        let r = [1.0f32, -1.0, 0.5];
        let p = panning(&l, &r).expect("pan");
        assert!(p.abs() < 1e-6, "pan={p}");
    }

    #[test]
    fn hard_left_is_minus_one() {
        let l = [1.0f32, 1.0, 1.0];
        let r = [0.0f32, 0.0, 0.0];
        let p = panning(&l, &r).expect("pan");
        assert!((p + 1.0).abs() < 1e-6, "pan={p}");
    }

    #[test]
    fn hard_right_is_plus_one() {
        let l = [0.0f32, 0.0, 0.0];
        let r = [2.0f32, 2.0, 2.0];
        let p = panning(&l, &r).expect("pan");
        assert!((p - 1.0).abs() < 1e-6, "pan={p}");
    }

    #[test]
    fn skewed_right_is_positive() {
        // E_L = 1, E_R = 4 -> (4-1)/(4+1) = 0.6
        let l = [1.0f32];
        let r = [2.0f32];
        let p = panning(&l, &r).expect("pan");
        assert!((p - 0.6).abs() < 1e-5, "pan={p}");
    }

    #[test]
    fn rejects_silence_and_mismatch() {
        assert_eq!(
            panning(&[0.0, 0.0], &[0.0, 0.0]),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(panning(&[1.0], &[1.0, 2.0]), Err(AudioError::InvalidParameter));
        assert_eq!(panning(&[], &[]), Err(AudioError::InvalidParameter));
    }
}
