//! Log-magnitude spectrum into a caller buffer (dB).

use crate::types::AudioError;

/// Convert a one-sided magnitude spectrum `mag` to a log-magnitude (dB)
/// spectrum written into `out_log`.
///
/// Each bin becomes `20·log10(max(|X[k]|, floor_linear))` dB, where
/// `floor_linear = 10^(floor_db / 20)`. The floor bounds silent bins so their
/// dB value clamps to `floor_db` instead of `-inf`.
///
/// Zero-heap: single pass into the caller-supplied `out_log`.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `floor_db` is not finite.
/// - [`AudioError::OutputBufferTooSmall`] if `out_log` is shorter than `mag`.
pub fn log_spectrum(mag: &[f32], floor_db: f32, out_log: &mut [f32]) -> Result<usize, AudioError> {
    if !floor_db.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    if out_log.len() < mag.len() {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let floor_linear = 10.0f32.powf(floor_db / 20.0);
    for (o, m) in out_log.iter_mut().zip(mag.iter()) {
        *o = 20.0 * m.abs().max(floor_linear).log10();
    }
    Ok(mag.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_magnitude_is_zero_db() {
        let mag = [1.0f32, 1.0, 1.0];
        let mut out = [0.0f32; 3];
        let n = log_spectrum(&mag, -120.0, &mut out).expect("log");
        assert_eq!(n, 3);
        for o in out {
            assert!(o.abs() < 1e-4, "dB={o}");
        }
    }

    #[test]
    fn tenfold_is_twenty_db() {
        // 10.0 -> 20 dB, 0.1 -> -20 dB.
        let mag = [10.0f32, 0.1];
        let mut out = [0.0f32; 2];
        log_spectrum(&mag, -120.0, &mut out).expect("log");
        assert!((out[0] - 20.0).abs() < 1e-3, "{}", out[0]);
        assert!((out[1] + 20.0).abs() < 1e-3, "{}", out[1]);
    }

    #[test]
    fn floor_clamps_silence() {
        let mag = [0.0f32, 1.0];
        let mut out = [0.0f32; 2];
        log_spectrum(&mag, -80.0, &mut out).expect("log");
        assert!((out[0] + 80.0).abs() < 1e-3, "floored={}", out[0]);
        assert!(out[1].abs() < 1e-4, "unit={}", out[1]);
    }

    #[test]
    fn rejects_small_output() {
        let mag = [1.0f32; 4];
        let mut out = [0.0f32; 3];
        assert_eq!(
            log_spectrum(&mag, -120.0, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn rejects_nonfinite_floor() {
        let mag = [1.0f32; 2];
        let mut out = [0.0f32; 2];
        assert_eq!(
            log_spectrum(&mag, f32::INFINITY, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }
}
