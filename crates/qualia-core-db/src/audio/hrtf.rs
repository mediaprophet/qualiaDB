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
}