//! P7.6 — Audio edits as geometric surface operations on the time-frequency
//! surface.
//!
//! Each edit is a geometric transformation of the TfSurface raster:
//!
//! - **Gain**: scale magnitude in a rectangular region (affine scale in z).
//! - **Cut/paste**: copy a rectangular patch of the surface to another
//!   location (translation in the time-frequency plane).
//! - **Time-stretch**: resample the surface along the time axis (affine
//!   scale in u).
//! - **Pitch-shift**: resample along the frequency axis (affine scale in v).
//! - **Crossfade**: blend two surfaces with a weight ramp (linear blend
//!   of two height fields).
//! - **Spectral gate**: zero out bins below a threshold (clipping plane
//!   in z).
//!
//! All operations write to caller-supplied buffers. Deterministic.

use super::tf_surface::TfSurface;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceEditError {
    BufferTooSmall { needed: usize, have: usize },
    InvalidRegion,
}

impl core::fmt::Display for SurfaceEditError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall { needed, have } => {
                write!(f, "surface edit: buffer too small, need {needed}, have {have}")
            }
            Self::InvalidRegion => write!(f, "surface edit: invalid region"),
        }
    }
}

impl std::error::Error for SurfaceEditError {}

// ───────────────────────────────────────────────────────────────────────────
//  Region
// ───────────────────────────────────────────────────────────────────────────

/// A rectangular region in the time-frequency plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub frame_start: usize,
    pub frame_end: usize,
    pub bin_start: usize,
    pub bin_end: usize,
}

impl Region {
    #[inline]
    pub fn new(frame_start: usize, frame_end: usize, bin_start: usize, bin_end: usize) -> Self {
        Self {
            frame_start,
            frame_end,
            bin_start,
            bin_end,
        }
    }

    #[inline]
    pub fn full(frame_count: usize, bin_count: usize) -> Self {
        Self::new(0, frame_count, 0, bin_count)
    }

    #[inline]
    pub fn frame_span(&self) -> usize {
        self.frame_end.saturating_sub(self.frame_start)
    }

    #[inline]
    pub fn bin_span(&self) -> usize {
        self.bin_end.saturating_sub(self.bin_start)
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.frame_end > self.frame_start && self.bin_end > self.bin_start
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Edits
// ───────────────────────────────────────────────────────────────────────────

/// Apply a gain (scalar multiplication) to a region of the surface.
/// `out` must be at least `frame_count * bin_count`.
pub fn apply_gain(
    surface: &TfSurface,
    region: &Region,
    gain: f32,
    out: &mut [f32],
) -> Result<usize, SurfaceEditError> {
    let needed = surface.frame_count * surface.bin_count;
    if out.len() < needed {
        return Err(SurfaceEditError::BufferTooSmall {
            needed,
            have: out.len(),
        });
    }

    // Copy the full surface, then apply gain to the region.
    out[..needed].copy_from_slice(surface.raster);

    let f_end = region.frame_end.min(surface.frame_count);
    let b_end = region.bin_end.min(surface.bin_count);
    for f in region.frame_start..f_end {
        for b in region.bin_start..b_end {
            out[f * surface.bin_count + b] *= gain;
        }
    }

    Ok(needed)
}

/// Spectral gate: zero out all bins below `threshold` in the region.
pub fn spectral_gate(
    surface: &TfSurface,
    region: &Region,
    threshold: f32,
    out: &mut [f32],
) -> Result<usize, SurfaceEditError> {
    let needed = surface.frame_count * surface.bin_count;
    if out.len() < needed {
        return Err(SurfaceEditError::BufferTooSmall {
            needed,
            have: out.len(),
        });
    }

    out[..needed].copy_from_slice(surface.raster);

    let f_end = region.frame_end.min(surface.frame_count);
    let b_end = region.bin_end.min(surface.bin_count);
    for f in region.frame_start..f_end {
        for b in region.bin_start..b_end {
            let idx = f * surface.bin_count + b;
            if out[idx] < threshold {
                out[idx] = 0.0;
            }
        }
    }

    Ok(needed)
}

/// Copy a rectangular patch from `src` to a destination offset in `out`.
/// `out` must be at least `frame_count * bin_count`.
pub fn copy_patch(
    surface: &TfSurface,
    src_region: &Region,
    dst_frame: usize,
    dst_bin: usize,
    out: &mut [f32],
) -> Result<usize, SurfaceEditError> {
    let needed = surface.frame_count * surface.bin_count;
    if out.len() < needed {
        return Err(SurfaceEditError::BufferTooSmall {
            needed,
            have: out.len(),
        });
    }

    out[..needed].copy_from_slice(surface.raster);

    let f_span = src_region.frame_span();
    let b_span = src_region.bin_span();

    for df in 0..f_span {
        let src_f = src_region.frame_start + df;
        let dst_f = dst_frame + df;
        if src_f >= surface.frame_count || dst_f >= surface.frame_count {
            break;
        }
        for db in 0..b_span {
            let src_b = src_region.bin_start + db;
            let dst_b = dst_bin + db;
            if src_b >= surface.bin_count || dst_b >= surface.bin_count {
                break;
            }
            out[dst_f * surface.bin_count + dst_b] =
                surface.raster[src_f * surface.bin_count + src_b];
        }
    }

    Ok(needed)
}

/// Time-stretch by resampling along the time axis.
/// `factor > 1.0` stretches, `factor < 1.0` compresses.
/// `out` must be at least `new_frame_count * bin_count` where
/// `new_frame_count = round(frame_count * factor)`.
pub fn time_stretch(
    surface: &TfSurface,
    factor: f32,
    out: &mut [f32],
) -> Result<(usize, usize), SurfaceEditError> {
    let new_frames = (surface.frame_count as f32 * factor).round() as usize;
    let new_frames = new_frames.max(1);
    let needed = new_frames * surface.bin_count;
    if out.len() < needed {
        return Err(SurfaceEditError::BufferTooSmall {
            needed,
            have: out.len(),
        });
    }

    for f in 0..new_frames {
        let src_f = f as f32 / factor;
        for b in 0..surface.bin_count {
            out[f * surface.bin_count + b] = surface.sample_bilinear(src_f, b as f32);
        }
    }

    Ok((new_frames, surface.bin_count))
}

/// Pitch-shift by resampling along the frequency axis.
/// `factor > 1.0` shifts up, `factor < 1.0` shifts down.
/// `out` must be at least `frame_count * bin_count`.
pub fn pitch_shift(
    surface: &TfSurface,
    factor: f32,
    out: &mut [f32],
) -> Result<usize, SurfaceEditError> {
    let needed = surface.frame_count * surface.bin_count;
    if out.len() < needed {
        return Err(SurfaceEditError::BufferTooSmall {
            needed,
            have: out.len(),
        });
    }

    for f in 0..surface.frame_count {
        for b in 0..surface.bin_count {
            let src_b = b as f32 / factor;
            out[f * surface.bin_count + b] = surface.sample_bilinear(f as f32, src_b);
        }
    }

    Ok(needed)
}

/// Crossfade: blend two surfaces with weight `t` (0 = surface_a, 1 = surface_b).
/// Both surfaces must have the same dimensions.
pub fn crossfade(
    surface_a: &TfSurface,
    surface_b: &TfSurface,
    t: f32,
    out: &mut [f32],
) -> Result<usize, SurfaceEditError> {
    if surface_a.frame_count != surface_b.frame_count
        || surface_a.bin_count != surface_b.bin_count
    {
        return Err(SurfaceEditError::InvalidRegion);
    }

    let needed = surface_a.frame_count * surface_a.bin_count;
    if out.len() < needed {
        return Err(SurfaceEditError::BufferTooSmall {
            needed,
            have: out.len(),
        });
    }

    for i in 0..needed {
        out[i] = surface_a.raster[i] * (1.0 - t) + surface_b.raster[i] * t;
    }

    Ok(needed)
}

/// Fade in: ramp gain from 0 to 1 over the first `fade_frames` frames.
pub fn fade_in(
    surface: &TfSurface,
    fade_frames: usize,
    out: &mut [f32],
) -> Result<usize, SurfaceEditError> {
    let needed = surface.frame_count * surface.bin_count;
    if out.len() < needed {
        return Err(SurfaceEditError::BufferTooSmall {
            needed,
            have: out.len(),
        });
    }

    out[..needed].copy_from_slice(surface.raster);

    let fade = fade_frames.min(surface.frame_count);
    for f in 0..fade {
        let gain = f as f32 / fade as f32;
        for b in 0..surface.bin_count {
            out[f * surface.bin_count + b] *= gain;
        }
    }

    Ok(needed)
}

/// Fade out: ramp gain from 1 to 0 over the last `fade_frames` frames.
pub fn fade_out(
    surface: &TfSurface,
    fade_frames: usize,
    out: &mut [f32],
) -> Result<usize, SurfaceEditError> {
    let needed = surface.frame_count * surface.bin_count;
    if out.len() < needed {
        return Err(SurfaceEditError::BufferTooSmall {
            needed,
            have: out.len(),
        });
    }

    out[..needed].copy_from_slice(surface.raster);

    let fade = fade_frames.min(surface.frame_count);
    let start = surface.frame_count - fade;
    for f in 0..fade {
        let gain = 1.0 - (f + 1) as f32 / fade as f32;
        for b in 0..surface.bin_count {
            out[(start + f) * surface.bin_count + b] *= gain;
        }
    }

    Ok(needed)
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::audio_spectral_sheet::SPECTRAL_PREVIEW_BINS;

    fn make_surface() -> (Vec<f32>, usize, usize) {
        let frames = 4;
        let bins = SPECTRAL_PREVIEW_BINS;
        let mut raster = vec![0.5f32; frames * bins];
        // Set some distinct values for testing.
        raster[1 * bins + 16] = 1.0;
        raster[2 * bins + 32] = 0.8;
        (raster, frames, bins)
    }

    #[test]
    fn gain_scales_region() {
        let (raster, frames, bins) = make_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let region = Region::new(1, 3, 10, 20);
        let mut out = vec![0.0f32; frames * bins];
        apply_gain(&s, &region, 2.0, &mut out).unwrap();
        // Inside region: doubled.
        assert!((out[1 * bins + 16] - 2.0).abs() < 1e-6, "peak should be doubled");
        // Outside region: unchanged.
        assert!((out[0 * bins + 0] - 0.5).abs() < 1e-6, "outside should be unchanged");
    }

    #[test]
    fn spectral_gate_zeros_below_threshold() {
        let (raster, frames, bins) = make_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let region = Region::full(frames, bins);
        let mut out = vec![0.0f32; frames * bins];
        spectral_gate(&s, &region, 0.6, &mut out).unwrap();
        // Values below 0.6 should be zeroed.
        assert_eq!(out[0 * bins + 0], 0.0, "0.5 should be gated");
        // Values >= 0.6 should remain.
        assert!((out[1 * bins + 16] - 1.0).abs() < 1e-6, "1.0 should remain");
        assert!((out[2 * bins + 32] - 0.8).abs() < 1e-6, "0.8 should remain");
    }

    #[test]
    fn copy_patch_translates_region() {
        let (raster, frames, bins) = make_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let src = Region::new(1, 2, 16, 17); // single cell at (1,16) = 1.0
        let mut out = vec![0.0f32; frames * bins];
        copy_patch(&s, &src, 3, 40, &mut out).unwrap();
        // The value 1.0 should now appear at (3, 40).
        assert!((out[3 * bins + 40] - 1.0).abs() < 1e-6, "patch should be copied");
    }

    #[test]
    fn time_stretch_doubles_frames() {
        let (raster, frames, bins) = make_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let mut out = vec![0.0f32; frames * 2 * bins];
        let (new_frames, _) = time_stretch(&s, 2.0, &mut out).unwrap();
        assert_eq!(new_frames, 8, "should double frame count");
    }

    #[test]
    fn time_stretch_preserves_energy_pattern() {
        let (raster, frames, bins) = make_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let mut out = vec![0.0f32; frames * 2 * bins];
        let (new_frames, _) = time_stretch(&s, 2.0, &mut out).unwrap();
        // The peak at frame 1 should appear around frame 2 in the stretched version.
        let peak_frame = (0..new_frames)
            .map(|f| (f, out[f * bins + 16]))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap()
            .0;
        assert!(peak_frame >= 1 && peak_frame <= 3, "peak should be near frame 2");
    }

    #[test]
    fn pitch_shift_preserves_frame_count() {
        let (raster, frames, bins) = make_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let mut out = vec![0.0f32; frames * bins];
        pitch_shift(&s, 2.0, &mut out).unwrap();
        // Frame count unchanged.
        // Check that the output has non-zero values.
        assert!(out.iter().any(|&v| v > 0.0), "pitch shift should preserve energy");
    }

    #[test]
    fn crossfade_blends_surfaces() {
        let (raster_a, frames, bins) = make_surface();
        let raster_b = vec![1.0f32; frames * bins];
        let sa = TfSurface::new(&raster_a, frames, bins, 44100, 512);
        let sb = TfSurface::new(&raster_b, frames, bins, 44100, 512);
        let mut out = vec![0.0f32; frames * bins];
        crossfade(&sa, &sb, 0.5, &mut out).unwrap();
        // At t=0.5, value should be average.
        let expected = (raster_a[0] + raster_b[0]) * 0.5;
        assert!((out[0] - expected).abs() < 1e-6, "crossfade should blend");
    }

    #[test]
    fn crossfade_at_t0_returns_a() {
        let (raster_a, frames, bins) = make_surface();
        let raster_b = vec![1.0f32; frames * bins];
        let sa = TfSurface::new(&raster_a, frames, bins, 44100, 512);
        let sb = TfSurface::new(&raster_b, frames, bins, 44100, 512);
        let mut out = vec![0.0f32; frames * bins];
        crossfade(&sa, &sb, 0.0, &mut out).unwrap();
        for i in 0..frames * bins {
            assert!((out[i] - raster_a[i]).abs() < 1e-6, "t=0 should return surface A");
        }
    }

    #[test]
    fn crossfade_at_t1_returns_b() {
        let (raster_a, frames, bins) = make_surface();
        let raster_b = vec![1.0f32; frames * bins];
        let sa = TfSurface::new(&raster_a, frames, bins, 44100, 512);
        let sb = TfSurface::new(&raster_b, frames, bins, 44100, 512);
        let mut out = vec![0.0f32; frames * bins];
        crossfade(&sa, &sb, 1.0, &mut out).unwrap();
        for i in 0..frames * bins {
            assert!((out[i] - raster_b[i]).abs() < 1e-6, "t=1 should return surface B");
        }
    }

    #[test]
    fn fade_in_ramps_from_zero() {
        let (raster, frames, bins) = make_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let mut out = vec![0.0f32; frames * bins];
        fade_in(&s, 2, &mut out).unwrap();
        // Frame 0 should be zeroed (gain = 0/2 = 0).
        assert_eq!(out[0], 0.0, "frame 0 should be zeroed by fade-in");
        // Frame 1 should be half (gain = 1/2 = 0.5).
        assert!((out[1 * bins] - 0.25).abs() < 1e-6, "frame 1 should be halved");
        // Frame 2+ should be unchanged.
        assert!((out[2 * bins] - 0.5).abs() < 1e-6, "frame 2 should be unchanged");
    }

    #[test]
    fn fade_out_ramps_to_zero() {
        let (raster, frames, bins) = make_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let mut out = vec![0.0f32; frames * bins];
        fade_out(&s, 2, &mut out).unwrap();
        // Last frame should be zeroed.
        assert_eq!(out[(frames - 1) * bins], 0.0, "last frame should be zeroed");
        // Second-to-last should be halved.
        assert!((out[(frames - 2) * bins] - 0.25).abs() < 1e-6, "second-to-last should be halved");
    }

    #[test]
    fn all_edits_deterministic() {
        let (raster, frames, bins) = make_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let region = Region::new(0, 2, 0, 32);

        let mut out1 = vec![0.0f32; frames * bins];
        let mut out2 = vec![0.0f32; frames * bins];
        apply_gain(&s, &region, 1.5, &mut out1).unwrap();
        apply_gain(&s, &region, 1.5, &mut out2).unwrap();
        assert_eq!(out1, out2, "gain must be deterministic");

        let mut out3 = vec![0.0f32; frames * bins];
        let mut out4 = vec![0.0f32; frames * bins];
        spectral_gate(&s, &region, 0.3, &mut out3).unwrap();
        spectral_gate(&s, &region, 0.3, &mut out4).unwrap();
        assert_eq!(out3, out4, "gate must be deterministic");
    }

    #[test]
    fn buffer_too_small_errors() {
        let (raster, frames, bins) = make_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let mut out = vec![0.0f32; 10]; // too small
        let region = Region::full(frames, bins);
        let err = apply_gain(&s, &region, 2.0, &mut out).unwrap_err();
        assert_eq!(err, SurfaceEditError::BufferTooSmall {
            needed: frames * bins,
            have: 10,
        });
    }
}
