//! Shared pitch-estimate type and the numeric core that both YIN variants use,
//! delegated to [`etalon_signal::pitch`].

/// One fundamental-frequency estimate for a single frame.
///
/// `f0_hz == 0.0` denotes an unvoiced / undetected frame (with `confidence`
/// near zero). Estimates are epistemic proposals, not ground truth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PitchEstimate {
    /// Estimated fundamental frequency in Hz (0.0 = unvoiced).
    pub f0_hz: f32,
    /// Voicing confidence in `[0, 1]` (1 = strongly periodic).
    pub confidence: f32,
}

impl PitchEstimate {
    /// The canonical unvoiced estimate.
    #[inline]
    pub const fn unvoiced() -> Self {
        Self {
            f0_hz: 0.0,
            confidence: 0.0,
        }
    }
}

/// Convert the raw difference function `diff[0..=max_lag]` into YIN's
/// cumulative-mean-normalised difference (CMND) **in place**.
#[inline]
pub(super) fn cmnd_in_place(diff: &mut [f32], max_lag: usize) {
    if diff.is_empty() {
        return;
    }
    let last = max_lag.min(diff.len() - 1);
    etalon_signal::pitch::yin_cmndf_in_place(&mut diff[..=last]);
}

/// Absolute-threshold lag selection over the CMND `cmnd[min_lag..=max_lag]`.
#[inline]
pub(super) fn absolute_threshold(
    cmnd: &[f32],
    min_lag: usize,
    max_lag: usize,
    threshold: f32,
) -> usize {
    etalon_signal::pitch::yin_absolute_threshold(cmnd, min_lag, max_lag, threshold)
}

/// Parabolic interpolation of the CMND minimum at integer lag `tau`.
#[inline]
pub(super) fn parabolic_min(
    cmnd: &[f32],
    tau: usize,
    min_lag: usize,
    max_lag: usize,
) -> (f32, f32) {
    etalon_signal::pitch::yin_parabolic_min(cmnd, tau, min_lag, max_lag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmnd_sets_dc_to_one() {
        let mut d = [0.0f32, 4.0, 8.0, 6.0];
        cmnd_in_place(&mut d, 3);
        assert_eq!(d[0], 1.0);
        assert!((d[1] - 1.0).abs() < 1e-6, "{}", d[1]);
        assert!((d[2] - 1.3333).abs() < 1e-3, "{}", d[2]);
    }

    #[test]
    fn threshold_picks_first_dip_below() {
        let cmnd = [1.0f32, 1.0, 0.9, 0.5, 0.05, 0.2, 0.8];
        let tau = absolute_threshold(&cmnd, 2, 6, 0.1);
        assert_eq!(tau, 4);
    }

    #[test]
    fn threshold_falls_back_to_global_min() {
        let cmnd = [1.0f32, 1.0, 0.9, 0.4, 0.6, 0.8];
        let tau = absolute_threshold(&cmnd, 2, 5, 0.1);
        assert_eq!(tau, 3, "global min lag");
    }

    #[test]
    fn parabolic_recovers_offset_vertex() {
        let f = |x: f32| (x - 4.3).powi(2) + 0.02;
        let cmnd: Vec<f32> = (0..8).map(|i| f(i as f32)).collect();
        let (better, minv) = parabolic_min(&cmnd, 4, 1, 7);
        assert!((better - 4.3).abs() < 1e-3, "tau={better}");
        assert!((minv - 0.02).abs() < 1e-3, "minv={minv}");
    }
}
