//! PPQ (pulses-per-quarter-note) tick ↔ musical time ↔ wall-clock time.
//!
//! A MIDI sequence measures time in *ticks*: there are `ppq` ticks in one
//! quarter note (a "beat" in 4/4). At a tempo of `bpm` quarter notes per
//! minute, one quarter note lasts `60 / bpm` seconds, so
//!
//! ```text
//! seconds = (ticks / ppq) * (60 / bpm)
//! ticks   = seconds * (bpm / 60) * ppq
//! ```
//!
//! Both directions are pure functions with no allocation — safe on the hot
//! path. Values are validated: `ppq` and `bpm` must be strictly positive.

use crate::types::AudioError;

/// Convert a PPQ tick count to seconds of wall-clock time at a given tempo.
///
/// `ppq` = ticks per quarter note (e.g. 480). `bpm` = quarter notes/minute.
/// Returns [`AudioError::InvalidParameter`] if `ppq == 0` or `bpm <= 0`.
#[inline]
pub fn ticks_to_seconds(ticks: u64, ppq: u32, bpm: f64) -> Result<f64, AudioError> {
    if ppq == 0 || !(bpm > 0.0) || !bpm.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    Ok((ticks as f64 / ppq as f64) * (60.0 / bpm))
}

/// Convert seconds of wall-clock time to a fractional PPQ tick count.
///
/// Inverse of [`ticks_to_seconds`]. Returns a fractional tick position; callers
/// that need an integer tick can round/floor. Returns
/// [`AudioError::InvalidParameter`] if `ppq == 0`, `bpm <= 0`, or `seconds`
/// is not finite/negative.
#[inline]
pub fn seconds_to_ticks(seconds: f64, ppq: u32, bpm: f64) -> Result<f64, AudioError> {
    if ppq == 0 || !(bpm > 0.0) || !bpm.is_finite() || !seconds.is_finite() || seconds < 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    Ok(seconds * (bpm / 60.0) * ppq as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_480_ticks_is_half_second_at_120bpm() {
        let s = ticks_to_seconds(480, 480, 120.0).expect("valid");
        assert!((s - 0.5).abs() < 1e-6, "got {s}");
        let s2 = ticks_to_seconds(960, 480, 120.0).expect("valid");
        assert!((s2 - 1.0).abs() < 1e-6, "got {s2}");
    }

    #[test]
    fn round_trip_ticks_seconds() {
        let t = seconds_to_ticks(0.5, 480, 120.0).expect("valid");
        assert!((t - 480.0).abs() < 1e-6, "got {t}");
        let t2 = seconds_to_ticks(1.0, 480, 120.0).expect("valid");
        assert!((t2 - 960.0).abs() < 1e-6, "got {t2}");
    }

    #[test]
    fn rejects_bad_params() {
        assert_eq!(ticks_to_seconds(1, 0, 120.0), Err(AudioError::InvalidParameter));
        assert_eq!(ticks_to_seconds(1, 480, 0.0), Err(AudioError::InvalidParameter));
        assert_eq!(seconds_to_ticks(-1.0, 480, 120.0), Err(AudioError::InvalidParameter));
    }
}
