//! LogAttackTime — Essentia-style log10 of the attack duration.

use crate::types::AudioError;

/// Fraction of the envelope maximum at which the attack is considered to start.
const START_THRESHOLD: f32 = 0.02;

/// Log (base 10) of the attack time of an amplitude `envelope`.
///
/// The attack runs from the first sample whose value exceeds
/// [`START_THRESHOLD`] × max up to the sample of maximum amplitude (the
/// "stop"). The returned value is `log10(attack_seconds)`. Following Essentia,
/// an effectively-zero attack is clamped to one sample so the log is finite.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `sample_rate <= 0`, the envelope is
///   empty, or its maximum is not positive.
pub fn log_attack_time(envelope: &[f32], sample_rate: f32) -> Result<f32, AudioError> {
    if sample_rate <= 0.0 || !sample_rate.is_finite() || envelope.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    let (max_idx, max_val) = arg_max(envelope);
    if !max_val.is_finite() || max_val <= 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    let thresh = START_THRESHOLD * max_val;
    let mut start_idx = 0usize;
    for (i, &v) in envelope.iter().enumerate() {
        if v >= thresh {
            start_idx = i;
            break;
        }
    }
    // Attack spans at least one sample.
    let span_samples = max_idx.saturating_sub(start_idx).max(1) as f32;
    let attack_seconds = span_samples / sample_rate;
    Ok(attack_seconds.log10())
}

#[inline]
fn arg_max(env: &[f32]) -> (usize, f32) {
    let mut idx = 0usize;
    let mut best = f32::NEG_INFINITY;
    for (i, &v) in env.iter().enumerate() {
        if v > best {
            best = v;
            idx = i;
        }
    }
    (idx, best)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_attack_decay(sr: f32, attack_s: f32, total_s: f32) -> (Vec<f32>, usize) {
        let attack_n = (attack_s * sr) as usize;
        let total_n = (total_s * sr) as usize;
        let mut env = vec![0.0f32; total_n];
        // Linear attack 0 -> 1.
        for i in 0..attack_n {
            env[i] = (i + 1) as f32 / attack_n as f32;
        }
        let peak = attack_n - 1;
        // Exponential decay after peak.
        let tau = (0.2 * sr).max(1.0);
        for i in attack_n..total_n {
            env[i] = (-((i - peak) as f32) / tau).exp();
        }
        (env, peak)
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(
            log_attack_time(&[], 8000.0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            log_attack_time(&[1.0, 2.0], 0.0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            log_attack_time(&[0.0, 0.0], 8000.0),
            Err(AudioError::InvalidParameter)
        );
    }

    /// Golden: a 100 ms linear attack recovers ≈ log10(0.1) = -1.
    #[test]
    fn recovers_hundred_ms_attack() {
        let sr = 10000.0f32;
        let (env, _peak) = build_attack_decay(sr, 0.1, 1.0);
        let lat = log_attack_time(&env, sr).expect("lat");
        let expected = 0.1f32.log10(); // -1.0
        assert!(
            (lat - expected).abs() < 0.05,
            "lat={lat} expected≈{expected}"
        );
    }
}
