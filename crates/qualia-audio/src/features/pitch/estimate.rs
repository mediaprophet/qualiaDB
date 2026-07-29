//! Shared pitch-estimate type and the numeric core that both YIN variants use:
//! cumulative-mean normalisation of the difference function, absolute-threshold
//! lag selection, and parabolic interpolation of the chosen lag.
//!
//! These helpers are `pub(super)` — they are the common spine of [`super::yin`]
//! and [`super::yin_fft`] and are not part of the folder's public surface (only
//! [`PitchEstimate`] is re-exported by `mod.rs`).

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
///
/// `diff[τ]` must hold `d(τ) = Σ_j (x[j] − x[j+τ])²`. On return `diff[0] = 1`
/// and `diff[τ] = d(τ) · τ / Σ_{k=1}^{τ} d(k)` for `τ ≥ 1`. Accumulation is in
/// `f64` for numerical stability over long lag ranges.
///
/// `diff.len()` must be at least `max_lag + 1`; excess entries are untouched.
pub(super) fn cmnd_in_place(diff: &mut [f32], max_lag: usize) {
    if diff.is_empty() {
        return;
    }
    diff[0] = 1.0;
    let mut running = 0.0f64;
    let last = max_lag.min(diff.len() - 1);
    for (tau, d) in diff.iter_mut().enumerate().take(last + 1).skip(1) {
        let raw = *d as f64;
        running += raw;
        *d = if running <= 0.0 {
            1.0
        } else {
            (raw * tau as f64 / running) as f32
        };
    }
}

/// Absolute-threshold lag selection over the CMND `cmnd[min_lag..=max_lag]`.
///
/// Returns the smallest lag whose CMND value first drops below `threshold`,
/// then descends to the local minimum of that dip (the classic YIN rule). If no
/// lag falls below the threshold, returns the lag of the global CMND minimum
/// over the search range — the caller decides voicing from the value there.
pub(super) fn absolute_threshold(
    cmnd: &[f32],
    min_lag: usize,
    max_lag: usize,
    threshold: f32,
) -> usize {
    let hi = max_lag.min(cmnd.len().saturating_sub(1));
    let lo = min_lag.max(1).min(hi);
    let mut best_tau = lo;
    let mut best_val = cmnd[lo];
    let mut tau = lo;
    while tau <= hi {
        let v = cmnd[tau];
        if v < best_val {
            best_val = v;
            best_tau = tau;
        }
        if v < threshold {
            // Descend into the dip to its local minimum.
            let mut t = tau;
            while t < hi && cmnd[t + 1] < cmnd[t] {
                t += 1;
            }
            return t;
        }
        tau += 1;
    }
    best_tau
}

/// Parabolic interpolation of the CMND minimum at integer lag `tau`.
///
/// Fits a parabola through `cmnd[tau-1], cmnd[tau], cmnd[tau+1]` and returns the
/// refined `(better_tau, min_value)` — a sub-sample lag and the interpolated
/// CMND value at the vertex (the frame's aperiodicity). Boundary lags fall back
/// to the integer value. This is what lifts F0 accuracy from whole-sample lag
/// steps to a few cents.
pub(super) fn parabolic_min(
    cmnd: &[f32],
    tau: usize,
    min_lag: usize,
    max_lag: usize,
) -> (f32, f32) {
    let hi = max_lag.min(cmnd.len().saturating_sub(1));
    if tau <= min_lag.max(1) || tau >= hi {
        return (tau as f32, cmnd[tau.min(cmnd.len() - 1)]);
    }
    let s0 = cmnd[tau - 1];
    let s1 = cmnd[tau];
    let s2 = cmnd[tau + 1];
    let denom = s0 - 2.0 * s1 + s2;
    if denom.abs() <= f32::EPSILON || !denom.is_finite() {
        return (tau as f32, s1);
    }
    // Vertex offset in (-1, 1); for a well-formed minimum |offset| < 0.5.
    let offset = (0.5 * (s0 - s2) / denom).clamp(-1.0, 1.0);
    let better_tau = tau as f32 + offset;
    // Interpolated vertex height.
    let min_val = s1 - 0.25 * (s0 - s2) * offset;
    (better_tau, min_val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmnd_sets_dc_to_one() {
        let mut d = [0.0f32, 4.0, 8.0, 6.0];
        cmnd_in_place(&mut d, 3);
        assert_eq!(d[0], 1.0);
        // d'(1) = 4*1/4 = 1.0
        assert!((d[1] - 1.0).abs() < 1e-6, "{}", d[1]);
        // d'(2) = 8*2 / (4+8) = 16/12 = 1.333
        assert!((d[2] - 1.3333).abs() < 1e-3, "{}", d[2]);
    }

    #[test]
    fn threshold_picks_first_dip_below() {
        // CMND with a dip below 0.1 at lag 4.
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
        // Parabola y = (x - 4.3)^2 + 0.02 sampled at integer lags.
        let f = |x: f32| (x - 4.3).powi(2) + 0.02;
        let cmnd: Vec<f32> = (0..8).map(|i| f(i as f32)).collect();
        let (better, minv) = parabolic_min(&cmnd, 4, 1, 7);
        assert!((better - 4.3).abs() < 1e-3, "tau={better}");
        assert!((minv - 0.02).abs() < 1e-3, "minv={minv}");
    }
}
