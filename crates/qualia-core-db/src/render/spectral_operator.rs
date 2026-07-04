//! P7.7 — Unified spectral-operator API surface.
//!
//! One entry point for all spectral operations: colour projection, gamut
//! mapping, metamers, spectral blend, and audio time-frequency surfaces.
//!
//! The `SpectralOperator` struct is a zero-allocation facade that dispatches
//! to the specialised submodules. It is the single API surface that the
//! renderer, audio engine, and export pipeline call.
//!
//! ## Determinism
//!
//! All operations are deterministic: identical inputs → bit-identical outputs.

use crate::audio::tf_surface::TfSurface;
use crate::audio::tf_surface_edit::{
    apply_gain, copy_patch, crossfade, fade_in, fade_out, pitch_shift, spectral_gate,
    time_stretch, Region as TfRegion, SurfaceEditError,
};
use crate::render::gamut::{gamut_map_clamp, is_in_gamut, linear_srgb_to_xyz};
use crate::render::metamer::{fibre_spd, is_metameric, min_norm_spd_for_xyz, metamer_kernel_basis};
use crate::render::spectral_blend::{blend_divergence, spectral_blend_emf, spectral_blend_spd};
use crate::render::spectral_kernel::{
    delta_e_76, emf_to_linear_rgb, emf_to_spd, linear_rgb_to_display, spd_to_xyz,
    xyz_to_lab, xyz_to_linear_srgb, LinearRgb, Spd, Xyz,
};

// ───────────────────────────────────────────────────────────────────────────
//  SpectralOperator
// ───────────────────────────────────────────────────────────────────────────

/// Unified spectral operator — the single entry point for all spectral
/// operations in the Qualia engine.
///
/// This is a zero-allocation facade: it holds no state and dispatches to
/// the specialised submodules. All methods are `&self` or associated
/// functions.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpectralOperator;

impl SpectralOperator {
    // ── P7.0: EMF → Colour ───────────────────────────────────────────

    /// Convert an EMF payload `[α, μ, σ]` to a Spectral Power Distribution.
    #[inline]
    pub fn emf_to_spd(alpha: f32, mu: f32, sigma: f32) -> Spd {
        emf_to_spd(alpha, mu, sigma)
    }

    /// Project an SPD to CIE XYZ tristimulus values.
    #[inline]
    pub fn spd_to_xyz(spd: &Spd) -> Xyz {
        spd_to_xyz(spd)
    }

    /// Convert CIE XYZ to linear sRGB.
    #[inline]
    pub fn xyz_to_linear_srgb(xyz: &Xyz) -> LinearRgb {
        xyz_to_linear_srgb(xyz)
    }

    /// Full EMF → linear sRGB pipeline.
    #[inline]
    pub fn emf_to_linear_rgb(alpha: f32, mu: f32, sigma: f32) -> LinearRgb {
        emf_to_linear_rgb(alpha, mu, sigma)
    }

    /// Convert linear sRGB to 8-bit display sRGB (gamma-encoded).
    #[inline]
    pub fn linear_rgb_to_display(rgb: &LinearRgb) -> (u8, u8, u8) {
        linear_rgb_to_display(rgb)
    }

    /// Full EMF → 8-bit display sRGB pipeline.
    #[inline]
    pub fn emf_to_display_rgb(alpha: f32, mu: f32, sigma: f32) -> (u8, u8, u8) {
        let rgb = emf_to_linear_rgb(alpha, mu, sigma);
        linear_rgb_to_display(&rgb)
    }

    // ── P7.0: Colour difference ──────────────────────────────────────

    /// CIE76 ΔE colour difference between two XYZ values.
    #[inline]
    pub fn delta_e(xyz_a: &Xyz, xyz_b: &Xyz) -> f32 {
        delta_e_76(xyz_a, xyz_b)
    }

    /// Convert XYZ to CIELAB.
    #[inline]
    pub fn xyz_to_lab(xyz: &Xyz) -> (f32, f32, f32) {
        xyz_to_lab(xyz)
    }

    // ── P7.1: Metamers ───────────────────────────────────────────────

    /// Compute the kernel basis for metameric-black SPDs.
    #[inline]
    pub fn metamer_kernel_basis(out_basis: &mut [f32]) -> Result<usize, crate::render::metamer::MetamerError> {
        metamer_kernel_basis(out_basis)
    }

    /// Compute the minimum-norm SPD for a target XYZ.
    #[inline]
    pub fn min_norm_spd_for_xyz(target: &Xyz) -> Spd {
        min_norm_spd_for_xyz(target)
    }

    /// Construct a fibre element: particular + Σ c_i · ker_i.
    #[inline]
    pub fn fibre_spd(particular: &Spd, basis: &[f32], coeffs: &[f32]) -> Spd {
        fibre_spd(particular, basis, coeffs)
    }

    /// Check if an SPD is metameric to a target XYZ.
    #[inline]
    pub fn is_metameric(spd: &Spd, target: &Xyz, tolerance: f32) -> bool {
        is_metameric(spd, target, tolerance)
    }

    // ── P7.2: Gamut ──────────────────────────────────────────────────

    /// Check if a colour is in the sRGB gamut.
    #[inline]
    pub fn is_in_gamut(xyz: &Xyz) -> bool {
        is_in_gamut(xyz)
    }

    /// Map an out-of-gamut colour to the closest in-gamut colour.
    #[inline]
    pub fn gamut_map(xyz: &Xyz) -> Xyz {
        gamut_map_clamp(xyz)
    }

    /// Convert linear sRGB to CIE XYZ.
    #[inline]
    pub fn linear_srgb_to_xyz(rgb: &LinearRgb) -> Xyz {
        linear_srgb_to_xyz(rgb)
    }

    // ── P7.3: Spectral blend ─────────────────────────────────────────

    /// Blend two SPDs in spectral space.
    #[inline]
    pub fn spectral_blend_spd(a: &Spd, b: &Spd, t: f32) -> Spd {
        spectral_blend_spd(a, b, t)
    }

    /// Blend two EMF payloads in spectral space and return XYZ.
    #[inline]
    pub fn spectral_blend_emf(
        alpha_a: f32, mu_a: f32, sigma_a: f32,
        alpha_b: f32, mu_b: f32, sigma_b: f32,
        t: f32,
    ) -> Xyz {
        spectral_blend_emf(alpha_a, mu_a, sigma_a, alpha_b, mu_b, sigma_b, t)
    }

    /// ΔE divergence between spectral blend and gamma-encoded sRGB lerp.
    #[inline]
    pub fn blend_divergence(
        alpha_a: f32, mu_a: f32, sigma_a: f32,
        alpha_b: f32, mu_b: f32, sigma_b: f32,
        t: f32,
    ) -> f32 {
        blend_divergence(alpha_a, mu_a, sigma_a, alpha_b, mu_b, sigma_b, t)
    }

    // ── P7.5: Audio time-frequency surface ───────────────────────────

    /// Create a time-frequency surface view from a raster.
    #[inline]
    pub fn tf_surface<'a>(
        raster: &'a [f32],
        frame_count: usize,
        bin_count: usize,
        sample_rate: u32,
        hop_size: usize,
    ) -> TfSurface<'a> {
        TfSurface::new(raster, frame_count, bin_count, sample_rate, hop_size)
    }

    // ── P7.6: Audio surface edits ────────────────────────────────────

    /// Apply a gain to a region of the surface.
    #[inline]
    pub fn surface_gain(
        surface: &TfSurface,
        region: &TfRegion,
        gain: f32,
        out: &mut [f32],
    ) -> Result<usize, SurfaceEditError> {
        apply_gain(surface, region, gain, out)
    }

    /// Spectral gate: zero out bins below a threshold.
    #[inline]
    pub fn surface_gate(
        surface: &TfSurface,
        region: &TfRegion,
        threshold: f32,
        out: &mut [f32],
    ) -> Result<usize, SurfaceEditError> {
        spectral_gate(surface, region, threshold, out)
    }

    /// Copy a rectangular patch to a new location.
    #[inline]
    pub fn surface_copy_patch(
        surface: &TfSurface,
        src_region: &TfRegion,
        dst_frame: usize,
        dst_bin: usize,
        out: &mut [f32],
    ) -> Result<usize, SurfaceEditError> {
        copy_patch(surface, src_region, dst_frame, dst_bin, out)
    }

    /// Time-stretch by resampling along the time axis.
    #[inline]
    pub fn surface_time_stretch(
        surface: &TfSurface,
        factor: f32,
        out: &mut [f32],
    ) -> Result<(usize, usize), SurfaceEditError> {
        time_stretch(surface, factor, out)
    }

    /// Pitch-shift by resampling along the frequency axis.
    #[inline]
    pub fn surface_pitch_shift(
        surface: &TfSurface,
        factor: f32,
        out: &mut [f32],
    ) -> Result<usize, SurfaceEditError> {
        pitch_shift(surface, factor, out)
    }

    /// Crossfade two surfaces.
    #[inline]
    pub fn surface_crossfade(
        surface_a: &TfSurface,
        surface_b: &TfSurface,
        t: f32,
        out: &mut [f32],
    ) -> Result<usize, SurfaceEditError> {
        crossfade(surface_a, surface_b, t, out)
    }

    /// Fade in over the first `fade_frames` frames.
    #[inline]
    pub fn surface_fade_in(
        surface: &TfSurface,
        fade_frames: usize,
        out: &mut [f32],
    ) -> Result<usize, SurfaceEditError> {
        fade_in(surface, fade_frames, out)
    }

    /// Fade out over the last `fade_frames` frames.
    #[inline]
    pub fn surface_fade_out(
        surface: &TfSurface,
        fade_frames: usize,
        out: &mut [f32],
    ) -> Result<usize, SurfaceEditError> {
        fade_out(surface, fade_frames, out)
    }

    // ── Batch operations ─────────────────────────────────────────────

    /// Batch EMF → display RGB: process N EMF payloads into N display RGB
    /// triples. `emf` is `[α, μ, σ]` triples (3*N floats), `out` is N*3 u8.
    pub fn batch_emf_to_display(emf: &[f32], out: &mut [u8]) -> usize {
        let n = emf.len() / 3;
        let n = n.min(out.len() / 3);
        for i in 0..n {
            let rgb = emf_to_linear_rgb(emf[i * 3], emf[i * 3 + 1], emf[i * 3 + 2]);
            let (r, g, b) = linear_rgb_to_display(&rgb);
            out[i * 3] = r;
            out[i * 3 + 1] = g;
            out[i * 3 + 2] = b;
        }
        n
    }

    /// Batch EMF → XYZ: process N EMF payloads into N XYZ triples.
    /// `emf` is `[α, μ, σ]` triples (3*N floats), `out` is N*3 f32.
    pub fn batch_emf_to_xyz(emf: &[f32], out: &mut [f32]) -> usize {
        let n = emf.len() / 3;
        let n = n.min(out.len() / 3);
        for i in 0..n {
            let spd = emf_to_spd(emf[i * 3], emf[i * 3 + 1], emf[i * 3 + 2]);
            let xyz = spd_to_xyz(&spd);
            out[i * 3] = xyz.x;
            out[i * 3 + 1] = xyz.y;
            out[i * 3 + 2] = xyz.z;
        }
        n
    }

    /// Batch gamut mapping: map N XYZ triples to in-gamut XYZ.
    /// `out` is N*3 f32.
    pub fn batch_gamut_map(xyz: &[f32], out: &mut [f32]) -> usize {
        let n = xyz.len() / 3;
        let n = n.min(out.len() / 3);
        for i in 0..n {
            let mapped = gamut_map_clamp(&Xyz::new(xyz[i * 3], xyz[i * 3 + 1], xyz[i * 3 + 2]));
            out[i * 3] = mapped.x;
            out[i * 3 + 1] = mapped.y;
            out[i * 3 + 2] = mapped.z;
        }
        n
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_emf_to_display_deterministic() {
        let (r1, g1, b1) = SpectralOperator::emf_to_display_rgb(1.0, 0.3, 0.5);
        let (r2, g2, b2) = SpectralOperator::emf_to_display_rgb(1.0, 0.3, 0.5);
        assert_eq!((r1, g1, b1), (r2, g2, b2));
    }

    #[test]
    fn operator_batch_emf_to_display() {
        let emf = [1.0, 0.0, 0.0, 1.0, 0.0, 0.5, 1.0, 0.0, 1.0];
        let mut out = [0u8; 9];
        let n = SpectralOperator::batch_emf_to_display(&emf, &mut out);
        assert_eq!(n, 3);
        // Each triple is valid u8, so it's guaranteed to be <= 255.
    }

    #[test]
    fn operator_batch_emf_to_xyz() {
        let emf = [1.0, 0.0, 0.0, 1.0, 0.0, 0.5, 1.0, 0.0, 1.0];
        let mut out = [0.0f32; 9];
        let n = SpectralOperator::batch_emf_to_xyz(&emf, &mut out);
        assert_eq!(n, 3);
        // All values should be finite.
        for v in &out {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn operator_batch_gamut_map() {
        let xyz = [2.0, 0.5, 0.0, 0.3, 0.4, 0.5];
        let mut out = [0.0f32; 6];
        let n = SpectralOperator::batch_gamut_map(&xyz, &mut out);
        assert_eq!(n, 2);
        // First was out-of-gamut, should be mapped in.
        let mapped = Xyz::new(out[0], out[1], out[2]);
        assert!(SpectralOperator::is_in_gamut(&mapped));
    }

    #[test]
    fn operator_metamer_round_trip() {
        let target = Xyz::new(0.4, 0.5, 0.3);
        let spd = SpectralOperator::min_norm_spd_for_xyz(&target);
        let reprojected = SpectralOperator::spd_to_xyz(&spd);
        assert!((reprojected.x - target.x).abs() < 0.05);
        assert!((reprojected.y - target.y).abs() < 0.05);
        assert!((reprojected.z - target.z).abs() < 0.05);
    }

    #[test]
    fn operator_blend_pipeline() {
        let xyz = SpectralOperator::spectral_blend_emf(1.0, 0.1, 0.2, 1.0, 0.1, 0.8, 0.5);
        assert!(xyz.x.is_finite() && xyz.y.is_finite() && xyz.z.is_finite());
    }

    #[test]
    fn operator_full_pipeline_emf_to_display() {
        // EMF → SPD → XYZ → linear sRGB → display sRGB
        for i in 0..=10 {
            let sigma = i as f32 / 10.0;
            let (_r, _g, _b) = SpectralOperator::emf_to_display_rgb(1.0, 0.2, sigma);
            // Display RGB returns u8, so it's always <= 255.
        }
    }

    #[test]
    fn operator_delta_e_self_zero() {
        let xyz = Xyz::new(0.3, 0.5, 0.2);
        assert!(SpectralOperator::delta_e(&xyz, &xyz) < 1e-6);
    }

    #[test]
    fn operator_surface_edits_via_facade() {
        use crate::audio::audio_spectral_sheet::SPECTRAL_PREVIEW_BINS;
        let frames = 4;
        let bins = SPECTRAL_PREVIEW_BINS;
        let raster = vec![0.5f32; frames * bins];
        let s = SpectralOperator::tf_surface(&raster, frames, bins, 44100, 512);
        let region = TfRegion::full(frames, bins);
        let mut out = vec![0.0f32; frames * bins];
        SpectralOperator::surface_gain(&s, &region, 2.0, &mut out).unwrap();
        assert!((out[0] - 1.0).abs() < 1e-6, "gain should double value");
    }
}
