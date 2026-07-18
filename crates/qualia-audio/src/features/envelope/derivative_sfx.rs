//! DerivativeSFX — mean first difference of the envelope after its maximum.

use crate::types::AudioError;

/// DerivativeSFX: the mean of the first difference (`env[i+1] - env[i]`) of the
/// `envelope` taken over the region **after** the maximum. It characterises the
/// decay shape: a smooth, gentle decay yields a small negative value; a sharp
/// drop yields a larger-magnitude negative value.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if the envelope has fewer than two
///   samples after the maximum (no derivative is defined).
pub fn derivative_sfx(envelope: &[f32]) -> Result<f32, AudioError> {
    if envelope.len() < 2 {
        return Err(AudioError::InvalidParameter);
    }
    // Locate the maximum.
    let mut max_idx = 0usize;
    let mut best = f32::NEG_INFINITY;
    for (i, &v) in envelope.iter().enumerate() {
        if v > best {
            best = v;
            max_idx = i;
        }
    }
    // Need at least two samples in the post-max tail to form one difference.
    if max_idx + 1 >= envelope.len() {
        return Err(AudioError::InvalidParameter);
    }
    let tail = &envelope[max_idx..];
    let mut sum = 0.0f64;
    let mut count = 0u32;
    for w in tail.windows(2) {
        sum += (w[1] - w[0]) as f64;
        count += 1;
    }
    if count == 0 {
        return Err(AudioError::InvalidParameter);
    }
    Ok((sum / count as f64) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_no_tail() {
        // Maximum is the last element -> no post-max derivative.
        assert_eq!(
            derivative_sfx(&[0.0, 1.0, 2.0]),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(derivative_sfx(&[1.0]), Err(AudioError::InvalidParameter));
    }

    /// Golden: a linear decay of known slope returns that slope as the mean
    /// derivative.
    #[test]
    fn linear_decay_recovers_slope() {
        // Rise to a peak at index 2, then fall by exactly 0.25 per sample.
        let env = [0.0f32, 0.5, 1.0, 0.75, 0.5, 0.25, 0.0];
        let d = derivative_sfx(&env).expect("d");
        assert!((d - (-0.25)).abs() < 1e-6, "derivative={d}");
    }
}
