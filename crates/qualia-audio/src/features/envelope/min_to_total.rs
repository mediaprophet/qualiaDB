//! MinToTotal — position of the envelope minimum, normalised to length.

use crate::types::AudioError;

/// Position of the minimum of `envelope`, normalised to `[0, 1)` by the total
/// length (`argmin / len`).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `envelope` is empty.
pub fn min_to_total(envelope: &[f32]) -> Result<f32, AudioError> {
    if envelope.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    let mut idx = 0usize;
    let mut best = f32::INFINITY;
    for (i, &v) in envelope.iter().enumerate() {
        if v < best {
            best = v;
            idx = i;
        }
    }
    Ok(idx as f32 / envelope.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert_eq!(min_to_total(&[]), Err(AudioError::InvalidParameter));
    }

    /// Golden: minimum at a known fraction is recovered.
    #[test]
    fn recovers_min_fraction() {
        let len = 4000usize;
        let trough = 3000usize; // fraction 0.75
        let mut env = vec![1.0f32; len];
        env[trough] = -0.5;
        let r = min_to_total(&env).expect("mtt");
        assert!((r - 0.75).abs() < 1e-3, "min_to_total={r}");
    }
}
