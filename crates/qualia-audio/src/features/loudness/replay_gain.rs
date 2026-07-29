//! ReplayGain 2.0 track/album gain from integrated loudness.
//!
//! ReplayGain 2.0 normalises to a reference of **−18 LUFS**. Given a program's
//! measured integrated loudness (LUFS, e.g. from
//! [`integrated_lufs`](crate::features::loudness::r128::integrated_lufs)), the
//! recommended gain is `target − measured` dB. Pure scalar arithmetic, zero-heap.

/// ReplayGain 2.0 reference loudness (LUFS).
pub const REPLAY_GAIN_2_TARGET_LUFS: f32 = -18.0;

/// ReplayGain adjustment in dB for a given integrated loudness (LUFS).
///
/// `gain_dB = target − integrated`. Positive → the track is quieter than the
/// reference and should be turned up; negative → it should be turned down.
pub fn replay_gain_db(integrated_lufs: f32) -> f32 {
    REPLAY_GAIN_2_TARGET_LUFS - integrated_lufs
}

/// The same adjustment as a linear amplitude scale factor (`10^(gain/20)`).
///
/// Multiply sample values by this to reach the reference level.
pub fn replay_gain_scale(integrated_lufs: f32) -> f32 {
    10f32.powf(replay_gain_db(integrated_lufs) / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_reference_gain_is_zero() {
        assert!((replay_gain_db(-18.0) - 0.0).abs() < 1e-6);
        assert!((replay_gain_scale(-18.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn quiet_track_gets_positive_gain() {
        // −23 LUFS is 5 LU below reference → +5 dB.
        assert!((replay_gain_db(-23.0) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn loud_track_gets_negative_gain() {
        // −9 LUFS is 9 LU above reference → −9 dB.
        assert!((replay_gain_db(-9.0) - (-9.0)).abs() < 1e-6);
    }

    #[test]
    fn six_db_gain_doubles_amplitude() {
        // −24 LUFS → +6 dB → scale ≈ 2.0.
        assert!((replay_gain_scale(-24.0) - 1.9952624).abs() < 1e-4);
    }
}
