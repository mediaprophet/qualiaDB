//! Autocorrelation-based tempogram: a tempo-salience curve over a BPM range,
//! computed from an onset-strength envelope. One public function; caller
//! supplies both the salience output and an autocorrelation scratch buffer
//! (zero-heap — no internal allocation).

use crate::features::peaks::autocorrelation;
use crate::types::AudioError;

/// Map bin `b` of an `n`-bin salience curve to its BPM.
///
/// Bins are spaced linearly and inclusively over `[bpm_min, bpm_max]`:
/// `bpm(b) = bpm_min + (bpm_max - bpm_min) * b / (n - 1)` for `n > 1`, and
/// `bpm_min` for `n == 1`.
#[inline]
pub fn bpm_for_bin(bin: usize, n_bins: usize, bpm_min: f32, bpm_max: f32) -> f32 {
    if n_bins <= 1 {
        return bpm_min;
    }
    bpm_min + (bpm_max - bpm_min) * (bin as f32) / ((n_bins - 1) as f32)
}

/// Linear interpolation of an autocorrelation buffer at a (possibly fractional)
/// lag. Out-of-range lags return `0.0`.
#[inline]
fn acf_at(acf: &[f32], lag: f32) -> f32 {
    if lag < 0.0 {
        return 0.0;
    }
    let i = lag.floor() as usize;
    if i + 1 >= acf.len() {
        return 0.0;
    }
    let frac = lag - i as f32;
    acf[i] * (1.0 - frac) + acf[i + 1] * frac
}

/// Compute a tempo-salience curve of `onset_env` across `[bpm_min, bpm_max]`.
///
/// The onset envelope's normalised autocorrelation is evaluated at the frame lag
/// implied by each candidate tempo — a periodic beat at `bpm` recurs every
/// `lag = frame_rate_hz * 60 / bpm` frames, so `salience[b] = acf(lag(bpm_b))`.
/// Peaks in the returned curve mark the tempi (and their sub/harmonics) the
/// envelope most strongly supports. Bin→BPM spacing is [`bpm_for_bin`].
///
/// `acf_scratch` receives the autocorrelation and must be long enough to reach
/// the slowest tempo's lag: `len >= ceil(frame_rate_hz*60/bpm_min) + 2`. The
/// autocorrelation is delegated to [`autocorrelation`] (no reimplementation);
/// nothing here allocates.
///
/// Returns the number of salience bins written (`= out_salience.len()`).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `onset_env` is empty, either BPM bound
///   is non-positive, `bpm_max <= bpm_min`, `frame_rate_hz <= 0`, or
///   `out_salience` is empty.
/// - [`AudioError::WorkspaceTooSmall`] if `acf_scratch` cannot reach the lag for
///   `bpm_min`.
pub fn tempogram(
    onset_env: &[f32],
    frame_rate_hz: f32,
    bpm_min: f32,
    bpm_max: f32,
    out_salience: &mut [f32],
    acf_scratch: &mut [f32],
) -> Result<usize, AudioError> {
    if onset_env.is_empty()
        || out_salience.is_empty()
        || !(frame_rate_hz > 0.0)
        || !(bpm_min > 0.0)
        || !(bpm_max > bpm_min)
    {
        return Err(AudioError::InvalidParameter);
    }

    // Slowest tempo needs the longest lag; require scratch to cover it.
    let max_lag = (frame_rate_hz * 60.0 / bpm_min).ceil() as usize + 2;
    if acf_scratch.len() < max_lag.min(onset_env.len()) + 1 {
        return Err(AudioError::WorkspaceTooSmall);
    }

    // Only lags we will actually sample are needed; cap at the envelope length
    // (autocorrelation writes 0 for lags >= N anyway).
    let want_lags = (max_lag + 1).min(acf_scratch.len());
    let acf = &mut acf_scratch[..want_lags];
    let lags = autocorrelation(onset_env, acf)?;
    let acf = &acf[..lags];

    let n = out_salience.len();
    for (b, s) in out_salience.iter_mut().enumerate() {
        let bpm = bpm_for_bin(b, n, bpm_min, bpm_max);
        let lag = frame_rate_hz * 60.0 / bpm;
        // Half-wave rectify: negative correlation is not tempo evidence.
        *s = acf_at(acf, lag).max(0.0);
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Impulse train at a known frame period → salience peaks at the true tempo.
    #[test]
    fn peaks_at_true_tempo() {
        // 100 Hz frame rate, beat every 50 frames -> 120 BPM.
        let frame_rate = 100.0f32;
        let period = 50usize;
        let n = 1000usize;
        let mut env = vec![0.0f32; n];
        let mut i = 0;
        while i < n {
            env[i] = 1.0;
            i += period;
        }

        let bpm_min = 40.0f32;
        let bpm_max = 240.0f32;
        let mut sal = [0.0f32; 201]; // 1 BPM resolution
        let mut scratch = [0.0f32; 256];
        let bins = tempogram(&env, frame_rate, bpm_min, bpm_max, &mut sal, &mut scratch)
            .expect("tempogram");
        assert_eq!(bins, 201);

        // Argmax bin -> BPM.
        let mut best = 0usize;
        for b in 1..bins {
            if sal[b] > sal[best] {
                best = b;
            }
        }
        let bpm = bpm_for_bin(best, bins, bpm_min, bpm_max);
        assert!((bpm - 120.0).abs() <= 120.0 * 0.05, "peak BPM = {bpm}");

        // 120 BPM (lag 50) must beat its half-tempo 60 BPM (lag 100).
        let b60 = ((60.0 - bpm_min) / (bpm_max - bpm_min) * (bins - 1) as f32).round() as usize;
        let b120 = ((120.0 - bpm_min) / (bpm_max - bpm_min) * (bins - 1) as f32).round() as usize;
        assert!(sal[b120] >= sal[b60], "120 BPM salience {} vs 60 BPM {}", sal[b120], sal[b60]);
    }

    #[test]
    fn rejects_bad_params() {
        let env = [1.0f32, 0.0, 1.0, 0.0];
        let mut sal = [0.0f32; 4];
        let mut scratch = [0.0f32; 64];
        assert_eq!(
            tempogram(&[], 100.0, 40.0, 240.0, &mut sal, &mut scratch),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            tempogram(&env, 0.0, 40.0, 240.0, &mut sal, &mut scratch),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            tempogram(&env, 100.0, 240.0, 40.0, &mut sal, &mut scratch),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn scratch_too_small_errors() {
        let env = vec![1.0f32; 200];
        let mut sal = [0.0f32; 16];
        // bpm_min 40 @ 100Hz -> lag 150; scratch of 8 cannot reach it.
        let mut scratch = [0.0f32; 8];
        assert_eq!(
            tempogram(&env, 100.0, 40.0, 240.0, &mut sal, &mut scratch),
            Err(AudioError::WorkspaceTooSmall)
        );
    }
}
