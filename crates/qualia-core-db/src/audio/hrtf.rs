//! HRTF — analytic + KemarLite embedded tables (zero-heap hot path).
//!
//! Full measured KEMAR datasets are optional cold assets; default hot path uses KemarLite
//! (8-azimuth ITD/ILD samples) with analytic fallback.

use core::f32::consts::PI;
use std::sync::atomic::{AtomicU8, Ordering};

/// HRTF synthesis profile (hot path).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HrtfProfile {
    Analytic = 0,
    KemarLite = 1,
}

static HRTF_PROFILE: AtomicU8 = AtomicU8::new(HrtfProfile::KemarLite as u8);

#[inline]
pub fn set_hrtf_profile(profile: HrtfProfile) {
    HRTF_PROFILE.store(profile as u8, Ordering::Relaxed);
}

#[inline]
pub fn hrtf_profile() -> HrtfProfile {
    match HRTF_PROFILE.load(Ordering::Relaxed) {
        0 => HrtfProfile::Analytic,
        _ => HrtfProfile::KemarLite,
    }
}

/// KemarLite: 8 azimuth samples (pan -1..1) — ITD µs and ILD dB-ish scalars (embedded cold asset).
const KEMAR_LITE_ITD_US: [f32; 8] = [-650.0, -480.0, -280.0, -80.0, 0.0, 80.0, 280.0, 480.0];
const KEMAR_LITE_ILD_DB: [f32; 8] = [6.0, 4.5, 2.5, 0.8, 0.0, -0.8, -2.5, -4.5];

/// Binaural staging gains + ITD for stereo worklet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinauralGains {
    pub gain_l: f32,
    pub gain_r: f32,
    pub itd_seconds: f32,
    pub azimuth_rad: f32,
    pub elevation_rad: f32,
    pub distance: f32,
}

impl Default for BinauralGains {
    fn default() -> Self {
        Self {
            gain_l: 0.707,
            gain_r: 0.707,
            itd_seconds: 0.0,
            azimuth_rad: 0.0,
            elevation_rad: 0.0,
            distance: 1.0,
        }
    }
}

/// Rotate source position into head frame (listener at origin, yaw around Y).
#[inline]
pub fn head_relative_position(source: [f32; 3], listener_yaw: f32) -> [f32; 3] {
    let c = listener_yaw.cos();
    let s = listener_yaw.sin();
    let x = source[0];
    let z = source[2];
    [x * c - z * s, source[1], x * s + z * c]
}

#[inline]
fn lerp_table(pan: f32, table: &[f32; 8]) -> f32 {
    let t = ((pan + 1.0) * 0.5 * 7.0).clamp(0.0, 7.0);
    let i = t.floor() as usize;
    let f = t - i as f32;
    let a = table[i.min(7)];
    let b = table[(i + 1).min(7)];
    a + (b - a) * f
}

/// KemarLite binaural — interpolated embedded ITD/ILD tables.
#[inline]
pub fn binaural_kemar_lite(source: [f32; 3], listener_yaw: f32) -> BinauralGains {
    let mut g = binaural_analytic(source, listener_yaw);
    let pan = (g.azimuth_rad / (PI * 0.5)).clamp(-1.0, 1.0);
    let itd_us = lerp_table(pan, &KEMAR_LITE_ITD_US);
    let ild_db = lerp_table(pan, &KEMAR_LITE_ILD_DB);
    let ild = (ild_db / 20.0).clamp(-0.45, 0.45);
    g.itd_seconds = itd_us * 1e-6;
    g.gain_l = (g.gain_l * (1.0 - ild)).clamp(0.05, 1.0);
    g.gain_r = (g.gain_r * (1.0 + ild)).clamp(0.05, 1.0);
    g
}

/// Analytic binaural gains — azimuth ∈ [-π, π], elevation ∈ [-π/2, π/2].
#[inline]
pub fn binaural_analytic(source: [f32; 3], listener_yaw: f32) -> BinauralGains {
    let rel = head_relative_position(source, listener_yaw);
    let x = rel[0];
    let y = rel[1];
    let z = (-rel[2]).max(0.05);
    let horiz = (x * x + z * z).sqrt();
    let dist = (x * x + y * y + rel[2] * rel[2]).sqrt().max(0.15);
    let azimuth = x.atan2(z);
    let elevation = y.atan2(horiz.max(1e-4));

    let pan = (azimuth / (PI * 0.5)).clamp(-1.0, 1.0);
    let ild = pan * 0.42;
    let gain_l = (0.707 * (1.0 - ild)).clamp(0.05, 1.0);
    let gain_r = (0.707 * (1.0 + ild)).clamp(0.05, 1.0);
    let itd_seconds = pan * 0.0006;
    let atten = (1.0 / dist).min(1.0);

    BinauralGains {
        gain_l: gain_l * atten,
        gain_r: gain_r * atten,
        itd_seconds,
        azimuth_rad: azimuth,
        elevation_rad: elevation,
        distance: dist,
    }
}

/// Dispatch binaural model from active `HrtfProfile`.
#[inline]
pub fn binaural_from_position(source: [f32; 3], listener_yaw: f32) -> BinauralGains {
    match hrtf_profile() {
        HrtfProfile::Analytic => binaural_analytic(source, listener_yaw),
        HrtfProfile::KemarLite => binaural_kemar_lite(source, listener_yaw),
    }
}

/// Manifold `w` biases room absorption (higher w → softer highs).
#[inline]
pub fn room_damp_from_manifold(manifold_w: f32) -> f32 {
    (1.0 - manifold_w * 0.08).clamp(0.55, 1.0)
}

// ---------------------------------------------------------------------------
// Real HRTF convolution (cold path; heap OK).
//
// Full measured KEMAR HRIRs remain an optional cold asset. The functions below
// SYNTHESIZE a physically-plausible HRIR directly from the ITD/ILD model in
// `BinauralGains` (interaural time difference as a fractional sample delay,
// interaural level difference as per-ear gain, and a one-pole head-shadow
// low-pass on the farther/contralateral ear). `binaural_render` then convolves a
// mono source with the left/right impulse responses to produce a binaural pair.
// ---------------------------------------------------------------------------

/// Direct linear (FIR) convolution: `y[i] = Σ_j signal[j]·h[i-j]`.
///
/// Output length is `signal.len() + h.len() - 1`. Either operand empty → empty.
pub fn convolve_fir(signal: &[f32], h: &[f32]) -> Vec<f32> {
    if signal.is_empty() || h.is_empty() {
        return Vec::new();
    }
    let out_len = signal.len() + h.len() - 1;
    let mut out = vec![0.0_f32; out_len];
    for (j, &s) in signal.iter().enumerate() {
        if s == 0.0 {
            continue;
        }
        for (m, &hm) in h.iter().enumerate() {
            out[j + m] += s * hm;
        }
    }
    out
}

/// Write a unit impulse scaled by `gain` at fractional `delay_samples` into `ir`,
/// splitting energy linearly across the two straddling integer taps.
#[inline]
fn place_fractional_impulse(ir: &mut [f32], delay_samples: f32, gain: f32) {
    if ir.is_empty() {
        return;
    }
    let d = delay_samples.max(0.0);
    let i0 = d.floor() as usize;
    let frac = d - i0 as f32;
    if i0 < ir.len() {
        ir[i0] += gain * (1.0 - frac);
    }
    if i0 + 1 < ir.len() {
        ir[i0 + 1] += gain * frac;
    }
}

/// One-pole low-pass `y[n] = y[n-1] + a·(x[n] - y[n-1])` applied in place
/// (head-shadow model; `a ∈ (0,1]`, smaller `a` = more high-frequency rolloff).
#[inline]
fn one_pole_lowpass_in_place(buf: &mut [f32], a: f32) {
    let a = a.clamp(0.0, 1.0);
    let mut y = 0.0_f32;
    for x in buf.iter_mut() {
        y += a * (*x - y);
        *x = y;
    }
}

/// Synthesize left/right HRIRs from `gains`.
///
/// Each ear is a unit impulse scaled by `gain_l`/`gain_r`, delayed by its ear
/// delay: the nearer ear sits at ~0, the farther ear is delayed by
/// `|itd_seconds|·sample_rate` samples (placed with linear-interpolated
/// fractional delay). A gentle one-pole low-pass (head shadow) is applied ONLY
/// to the contralateral (farther, quieter) ear so its highs are attenuated.
///
/// `itd_seconds < 0` ⇒ source to the left ⇒ right ear is the farther/contralateral
/// one; `itd_seconds > 0` ⇒ left ear is farther. Returns `(left_ir, right_ir)`,
/// both length `taps` (min 1).
pub fn synthesize_hrir(
    gains: &BinauralGains,
    sample_rate: f32,
    taps: usize,
) -> (Vec<f32>, Vec<f32>) {
    let n = taps.max(1);
    let mut left = vec![0.0_f32; n];
    let mut right = vec![0.0_f32; n];

    let delay = (gains.itd_seconds.abs() * sample_rate.max(1.0)).max(0.0);
    // Head-shadow low-pass coefficient for the contralateral ear (gentle).
    const SHADOW_A: f32 = 0.35;

    if gains.itd_seconds <= 0.0 {
        // Source to the LEFT: left ear nearer (no delay), right ear farther.
        place_fractional_impulse(&mut left, 0.0, gains.gain_l);
        place_fractional_impulse(&mut right, delay, gains.gain_r);
        one_pole_lowpass_in_place(&mut right, SHADOW_A);
    } else {
        // Source to the RIGHT: right ear nearer, left ear farther.
        place_fractional_impulse(&mut right, 0.0, gains.gain_r);
        place_fractional_impulse(&mut left, delay, gains.gain_l);
        one_pole_lowpass_in_place(&mut left, SHADOW_A);
    }

    (left, right)
}

/// Render a `mono` source at `source` (world space) into a binaural pair.
///
/// Pipeline: `binaural_from_position` (active `HrtfProfile`) →
/// `synthesize_hrir` → `convolve_fir` the mono signal with each ear's HRIR.
/// Returns `(left, right)`, each of length `mono.len() + taps - 1` (taps = 64).
pub fn binaural_render(
    mono: &[f32],
    source: [f32; 3],
    listener_yaw: f32,
    sample_rate: f32,
) -> (Vec<f32>, Vec<f32>) {
    const TAPS: usize = 64;
    let gains = binaural_from_position(source, listener_yaw);
    let (hl, hr) = synthesize_hrir(&gains, sample_rate, TAPS);
    let left = convolve_fir(mono, &hl);
    let right = convolve_fir(mono, &hr);
    (left, right)
}

/// First index where the running energy of `x` crosses `frac` of its total
/// energy (onset proxy for ITD comparison). Returns `x.len()` if all-zero.
#[cfg(test)]
fn energy_onset(x: &[f32], frac: f32) -> usize {
    let total: f32 = x.iter().map(|&v| v * v).sum();
    if total <= 0.0 {
        return x.len();
    }
    let threshold = total * frac;
    let mut acc = 0.0_f32;
    for (i, &v) in x.iter().enumerate() {
        acc += v * v;
        if acc >= threshold {
            return i;
        }
    }
    x.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_source_balanced() {
        let g = binaural_from_position([0.0, 0.0, 1.0], 0.0);
        assert!((g.gain_l - g.gain_r).abs() < 0.15);
        assert!(g.itd_seconds.abs() < 1e-4);
    }

    #[test]
    fn left_source_favors_left_ear() {
        let g = binaural_from_position([-1.0, 0.0, -1.0], 0.0);
        assert!(g.gain_l > g.gain_r);
        assert!(g.itd_seconds < 0.0);
    }

    #[test]
    fn right_source_favors_right_ear() {
        let g = binaural_from_position([1.0, 0.0, -1.0], 0.0);
        assert!(g.gain_r > g.gain_l);
        assert!(g.itd_seconds > 0.0);
    }

    #[test]
    fn kemar_lite_stronger_itd_than_analytic_at_side() {
        set_hrtf_profile(HrtfProfile::KemarLite);
        let k = binaural_from_position([-1.0, 0.0, 0.5], 0.0);
        set_hrtf_profile(HrtfProfile::Analytic);
        let a = binaural_from_position([-1.0, 0.0, 0.5], 0.0);
        assert!(k.itd_seconds.abs() >= a.itd_seconds.abs());
        set_hrtf_profile(HrtfProfile::KemarLite);
    }

    #[test]
    fn yaw_rotates_pan() {
        let g0 = binaural_from_position([1.0, 0.0, 1.0], 0.0);
        let g1 = binaural_from_position([1.0, 0.0, 1.0], PI * 0.5);
        assert!(g0.gain_r > g0.gain_l, "right-front source favors right ear");
        assert!(g1.gain_l > g1.gain_r, "90° yaw inverts pan to left ear");
        assert!(
            (g0.gain_r - g1.gain_l).abs() < 0.12,
            "yaw swap: g0.gain_r≈g1.gain_l"
        );
        assert!(
            (g0.gain_l - g1.gain_r).abs() < 0.12,
            "yaw swap: g0.gain_l≈g1.gain_r"
        );
    }

    #[test]
    fn convolve_identity_kernel_returns_input() {
        let x = [0.1_f32, -0.4, 0.7, 0.2, -0.9];
        let y = convolve_fir(&x, &[1.0]);
        assert_eq!(y.len(), x.len());
        for (a, b) in y.iter().zip(x.iter()) {
            assert!((a - b).abs() < 1e-6, "identity convolution");
        }
    }

    #[test]
    fn convolve_output_length() {
        let x = vec![0.5_f32; 17];
        let h = vec![0.25_f32; 9];
        let y = convolve_fir(&x, &h);
        assert_eq!(y.len(), x.len() + h.len() - 1);
    }

    #[test]
    fn convolve_known_result() {
        // [1,2,3] * [1,1] = [1, 3, 5, 3]
        let y = convolve_fir(&[1.0, 2.0, 3.0], &[1.0, 1.0]);
        assert_eq!(y.len(), 4);
        let expect = [1.0, 3.0, 5.0, 3.0];
        for (a, b) in y.iter().zip(expect.iter()) {
            assert!((a - b).abs() < 1e-6, "got {y:?}");
        }
    }

    #[test]
    fn hard_left_source_earlier_and_louder_on_left() {
        set_hrtf_profile(HrtfProfile::KemarLite);
        let sample_rate = 48_000.0_f32;
        // A unit click followed by silence.
        let mut click = vec![0.0_f32; 128];
        click[0] = 1.0;
        // Hard-left source (−X, in front).
        let (left, right) = binaural_render(&click, [-1.0, 0.0, -1.0], 0.0, sample_rate);

        // ITD: the left (nearer) ear's energy onset is EARLIER than the right.
        let onset_l = energy_onset(&left, 0.5);
        let onset_r = energy_onset(&right, 0.5);
        assert!(
            onset_l < onset_r,
            "left onset {onset_l} should precede right onset {onset_r} (ITD)"
        );

        // ILD: left energy >= right energy (nearer/louder ear).
        let energy_l: f32 = left.iter().map(|&v| v * v).sum();
        let energy_r: f32 = right.iter().map(|&v| v * v).sum();
        assert!(
            energy_l >= energy_r,
            "left energy {energy_l} should be >= right energy {energy_r} (ILD)"
        );
    }

    #[test]
    fn synthesize_hrir_delays_contralateral_ear() {
        let sample_rate = 48_000.0_f32;
        // itd < 0 ⇒ source left ⇒ right ear delayed.
        let g = BinauralGains {
            gain_l: 0.8,
            gain_r: 0.6,
            itd_seconds: -0.0006,
            ..Default::default()
        };
        let (left, right) = synthesize_hrir(&g, sample_rate, 64);
        assert_eq!(left.len(), 64);
        assert_eq!(right.len(), 64);
        // Left ear (near) has its first tap energetic; right ear's peak is later.
        let left_peak = left
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
            .unwrap()
            .0;
        let right_peak = right
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
            .unwrap()
            .0;
        assert_eq!(left_peak, 0, "near (left) ear impulse at tap 0");
        assert!(
            right_peak > 0,
            "contralateral (right) ear delayed, peak at {right_peak}"
        );
    }

    #[test]
    fn binaural_render_output_length() {
        let mono = vec![0.3_f32; 100];
        let (l, r) = binaural_render(&mono, [1.0, 0.0, -1.0], 0.0, 48_000.0);
        assert_eq!(l.len(), 100 + 64 - 1);
        assert_eq!(r.len(), 100 + 64 - 1);
    }
}
