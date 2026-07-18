//! MaxToTotal — position of the envelope maximum, normalised to length.

use crate::types::AudioError;

/// Position of the maximum of `envelope`, normalised to `[0, 1)` by the total
/// length (`argmax / len`). A value near 0 indicates an early peak (percussive
/// onset); near 1, a late peak.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `envelope` is empty.
pub fn max_to_total(envelope: &[f32]) -> Result<f32, AudioError> {
    if envelope.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    let mut idx = 0usize;
    let mut best = f32::NEG_INFINITY;
    for (i, &v) in envelope.iter().enumerate() {
        if v > best {
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
        assert_eq!(max_to_total(&[]), Err(AudioError::InvalidParameter));
    }

    /// Golden: a signal peaking at a known fraction returns that fraction.
    #[test]
    fn recovers_peak_fraction() {
        let len = 5000usize;
        let peak = 1000usize; // fraction 0.2
        let mut env = vec![0.0f32; len];
        for i in 0..len {
            // Rise to peak at `peak`, then fall — unique maximum at `peak`.
            env[i] = if i <= peak {
                i as f32 / peak as f32
            } else {
                (len - i) as f32 / (len - peak) as f32
            };
        }
        let r = max_to_total(&env).expect("mtt");
        assert!((r - 0.2).abs() < 1e-3, "max_to_total={r}");
    }
}
