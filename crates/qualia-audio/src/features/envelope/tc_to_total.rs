//! TCToTotal — temporal centroid of the envelope, normalised to length.

use crate::types::AudioError;

/// Temporal centroid of `envelope` (the amplitude-weighted mean index),
/// normalised to `[0, 1)` by the total length. For a symmetric envelope the
/// centroid sits at the middle (≈ 0.5); an early-weighted envelope returns a
/// smaller value.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `envelope` is empty or its total
///   (absolute) weight is zero.
pub fn tc_to_total(envelope: &[f32]) -> Result<f32, AudioError> {
    if envelope.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    let mut weighted = 0.0f64;
    let mut total = 0.0f64;
    for (i, &v) in envelope.iter().enumerate() {
        let w = v.abs() as f64;
        weighted += i as f64 * w;
        total += w;
    }
    if total == 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    let centroid = weighted / total;
    Ok((centroid / envelope.len() as f64) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_degenerate() {
        assert_eq!(tc_to_total(&[]), Err(AudioError::InvalidParameter));
        assert_eq!(
            tc_to_total(&[0.0, 0.0, 0.0]),
            Err(AudioError::InvalidParameter)
        );
    }

    /// Golden: a symmetric triangular envelope has its centroid at ≈ 0.5.
    #[test]
    fn symmetric_centroid_is_middle() {
        let len = 1001usize; // odd -> exact centre index
        let mid = len / 2;
        let mut env = vec![0.0f32; len];
        for i in 0..len {
            env[i] = 1.0 - (i as f32 - mid as f32).abs() / mid as f32;
        }
        let r = tc_to_total(&env).expect("tc");
        // centroid index ≈ mid=500, /len=1001 -> ≈0.4995
        assert!((r - 0.5).abs() < 0.01, "tc_to_total={r}");
    }
}
