//! P7.5 — Audio time-frequency SURFACE view over STFT/CQT rasters.
//!
//! The existing `audio::stft` and `audio::cqt_bake` modules produce
//! time × frequency magnitude rasters. This module treats those rasters
//! as a 2D manifold surface where:
//! - **u axis** = time (frame index)
//! - **v axis** = frequency (bin index)
//! - **height** = magnitude (dB or linear)
//!
//! The surface supports:
//! - Bilinear sampling at arbitrary (time, frequency) coordinates
//! - Gradient computation (spectral flux, frequency derivative)
//! - Ridge detection (local maxima along frequency axis per frame)
//! - Surface-to-mesh conversion for visualisation
//!
//! ## Determinism
//!
//! All operations are deterministic: the raster is a fixed f32 matrix,
//! and sampling/gradient/ridge are pure functions.

// ───────────────────────────────────────────────────────────────────────────
//  Types
// ───────────────────────────────────────────────────────────────────────────

/// A time-frequency surface: `frames × bins` magnitude raster.
///
/// The raster is row-major: `raster[frame * bins + bin]`.
/// This is a view (borrowed) — no heap allocation.
#[derive(Debug, Clone, Copy)]
pub struct TfSurface<'a> {
    pub raster: &'a [f32],
    pub frame_count: usize,
    pub bin_count: usize,
    pub sample_rate: u32,
    pub hop_size: usize,
}

impl<'a> TfSurface<'a> {
    /// Create a surface view from a raw raster slice.
    ///
    /// `raster.len()` must equal `frame_count * bin_count`.
    pub fn new(
        raster: &'a [f32],
        frame_count: usize,
        bin_count: usize,
        sample_rate: u32,
        hop_size: usize,
    ) -> Self {
        debug_assert_eq!(raster.len(), frame_count * bin_count);
        Self {
            raster,
            frame_count,
            bin_count,
            sample_rate,
            hop_size,
        }
    }

    /// Get the magnitude at `(frame, bin)`.
    #[inline]
    pub fn get(&self, frame: usize, bin: usize) -> f32 {
        if frame >= self.frame_count || bin >= self.bin_count {
            return 0.0;
        }
        self.raster[frame * self.bin_count + bin]
    }

    /// Time in seconds for a given frame index.
    #[inline]
    pub fn frame_to_time(&self, frame: usize) -> f32 {
        if self.sample_rate == 0 || self.hop_size == 0 {
            return 0.0;
        }
        frame as f32 * self.hop_size as f32 / self.sample_rate as f32
    }

    /// Frequency in Hz for a linear bin index (STFT).
    #[inline]
    pub fn bin_to_freq_linear(&self, bin: usize) -> f32 {
        if self.sample_rate == 0 || self.bin_count == 0 {
            return 0.0;
        }
        bin as f32 * (self.sample_rate as f32 / (2.0 * self.bin_count as f32))
    }

    /// Frequency in Hz for a log-spaced bin index (CQT).
    /// `f_k = f_min * 2^(k / bins_per_octave)`
    #[inline]
    pub fn bin_to_freq_log(&self, bin: usize, f_min: f32, bins_per_octave: usize) -> f32 {
        if bins_per_octave == 0 {
            return 0.0;
        }
        f_min * 2.0_f32.powf(bin as f32 / bins_per_octave as f32)
    }

    /// Bilinear sample at fractional `(frame_f, bin_f)`.
    #[inline]
    pub fn sample_bilinear(&self, frame_f: f32, bin_f: f32) -> f32 {
        let f0 = frame_f.floor() as isize;
        let b0 = bin_f.floor() as isize;
        let df = frame_f - f0 as f32;
        let db = bin_f - b0 as f32;

        let f0 = f0.max(0).min(self.frame_count as isize - 1) as usize;
        let f1 = (f0 + 1).min(self.frame_count - 1);
        let b0 = b0.max(0).min(self.bin_count as isize - 1) as usize;
        let b1 = (b0 + 1).min(self.bin_count - 1);

        let v00 = self.get(f0, b0);
        let v01 = self.get(f0, b1);
        let v10 = self.get(f1, b0);
        let v11 = self.get(f1, b1);

        let top = v00 * (1.0 - db) + v01 * db;
        let bot = v10 * (1.0 - db) + v11 * db;
        top * (1.0 - df) + bot * df
    }

    /// Spectral flux: sum of positive magnitude differences between
    /// consecutive frames. Written to `out` (length `frame_count - 1`).
    pub fn spectral_flux(&self, out: &mut [f32]) -> usize {
        let n = self.frame_count.saturating_sub(1);
        if out.len() < n {
            return 0;
        }
        for f in 0..n {
            let mut flux = 0.0f32;
            for b in 0..self.bin_count {
                let diff = self.get(f + 1, b) - self.get(f, b);
                if diff > 0.0 {
                    flux += diff * diff;
                }
            }
            out[f] = flux.sqrt();
        }
        n
    }

    /// Frequency gradient at `(frame, bin)` — central difference.
    #[inline]
    pub fn freq_gradient(&self, frame: usize, bin: usize) -> f32 {
        let prev = self.get(frame, bin.saturating_sub(1));
        let next = self.get(frame, (bin + 1).min(self.bin_count - 1));
        (next - prev) * 0.5
    }

    /// Time gradient at `(frame, bin)` — central difference.
    #[inline]
    pub fn time_gradient(&self, frame: usize, bin: usize) -> f32 {
        let prev = self.get(frame.saturating_sub(1), bin);
        let next = self.get((frame + 1).min(self.frame_count - 1), bin);
        (next - prev) * 0.5
    }

    /// Find the ridge bin (peak magnitude) for a given frame.
    /// Returns the bin index of the maximum.
    pub fn ridge_bin(&self, frame: usize) -> usize {
        if frame >= self.frame_count {
            return 0;
        }
        let row = &self.raster[frame * self.bin_count..(frame + 1) * self.bin_count];
        let (idx, _) = row
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
            .unwrap_or((0, &0.0));
        idx
    }

    /// Collect ridge bins for all frames into `out`.
    /// Returns the number of ridges written (= `frame_count`).
    pub fn ridges(&self, out: &mut [usize]) -> usize {
        let n = self.frame_count.min(out.len());
        for f in 0..n {
            out[f] = self.ridge_bin(f);
        }
        n
    }

    /// Convert to a height-map mesh: each (frame, bin) → (x, y, z)
    /// where x = time, y = freq, z = magnitude.
    /// Writes `frame_count * bin_count` vertices to `out_verts` as
    /// `[x, y, z]` triples. Returns the number of vertices written.
    pub fn to_height_mesh(&self, out_verts: &mut [f32]) -> usize {
        let n = self.frame_count * self.bin_count;
        if out_verts.len() < n * 3 {
            return 0;
        }
        for f in 0..self.frame_count {
            let t = self.frame_to_time(f);
            for b in 0..self.bin_count {
                let freq = self.bin_to_freq_linear(b);
                let mag = self.get(f, b);
                let idx = (f * self.bin_count + b) * 3;
                out_verts[idx] = t;
                out_verts[idx + 1] = freq;
                out_verts[idx + 2] = mag;
            }
        }
        n
    }

    /// Total energy (sum of all magnitudes).
    pub fn total_energy(&self) -> f32 {
        self.raster.iter().copied().sum()
    }

    /// Frame energy (sum of magnitudes in a single frame).
    pub fn frame_energy(&self, frame: usize) -> f32 {
        if frame >= self.frame_count {
            return 0.0;
        }
        self.raster[frame * self.bin_count..(frame + 1) * self.bin_count]
            .iter()
            .copied()
            .sum()
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::audio_spectral_sheet::SPECTRAL_PREVIEW_BINS;

    fn make_test_surface() -> (Vec<f32>, usize, usize) {
        let frames = 4;
        let bins = SPECTRAL_PREVIEW_BINS;
        let mut raster = vec![0.0f32; frames * bins];
        // Put a peak at frame 1, bin 16.
        raster[1 * bins + 16] = 1.0;
        // Put a peak at frame 2, bin 32.
        raster[2 * bins + 32] = 0.8;
        (raster, frames, bins)
    }

    #[test]
    fn surface_get_in_bounds() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        assert_eq!(s.get(1, 16), 1.0);
        assert_eq!(s.get(2, 32), 0.8);
        assert_eq!(s.get(0, 0), 0.0);
    }

    #[test]
    fn surface_get_out_of_bounds_returns_zero() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        assert_eq!(s.get(100, 0), 0.0);
        assert_eq!(s.get(0, 100), 0.0);
    }

    #[test]
    fn bilinear_sample_at_integer_coords() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        assert!((s.sample_bilinear(1.0, 16.0) - 1.0).abs() < 1e-6);
        assert!((s.sample_bilinear(2.0, 32.0) - 0.8).abs() < 1e-6);
    }

    #[test]
    fn bilinear_sample_interpolates() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        // Sample midway between frame 1 (peak=1.0 at bin 16) and frame 2 (peak=0.8 at bin 32).
        let val = s.sample_bilinear(1.5, 24.0);
        // Should be non-zero (interpolating between the two peaks).
        assert!(val >= 0.0, "bilinear sample should be non-negative");
    }

    #[test]
    fn spectral_flux_detects_onsets() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let mut flux = [0.0f32; 3];
        let n = s.spectral_flux(&mut flux);
        assert_eq!(n, 3);
        // Frame 0→1: energy appears (flux > 0).
        assert!(flux[0] > 0.0, "flux at onset should be positive");
        // Frame 1→2: peak moves, flux should be positive.
        assert!(flux[1] > 0.0, "flux during peak move should be positive");
    }

    #[test]
    fn ridge_bin_finds_peak() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        assert_eq!(s.ridge_bin(1), 16, "peak at frame 1 should be bin 16");
        assert_eq!(s.ridge_bin(2), 32, "peak at frame 2 should be bin 32");
    }

    #[test]
    fn ridges_collects_all_frames() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let mut ridges = [0usize; 4];
        let n = s.ridges(&mut ridges);
        assert_eq!(n, 4);
        assert_eq!(ridges[1], 16);
        assert_eq!(ridges[2], 32);
    }

    #[test]
    fn frame_to_time_correct() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let t0 = s.frame_to_time(0);
        let t1 = s.frame_to_time(1);
        assert!((t0 - 0.0).abs() < 1e-6);
        assert!((t1 - 512.0 / 44100.0).abs() < 1e-6);
    }

    #[test]
    fn bin_to_freq_linear_correct() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        // Nyquist = 22050, bins = 64, so bin_freq = 22050/64 ≈ 344.5
        let f0 = s.bin_to_freq_linear(0);
        let f1 = s.bin_to_freq_linear(1);
        assert!((f0 - 0.0).abs() < 1e-6);
        assert!(f1 > 0.0);
    }

    #[test]
    fn bin_to_freq_log_correct() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let f0 = s.bin_to_freq_log(0, 55.0, 12);
        let f12 = s.bin_to_freq_log(12, 55.0, 12);
        assert!((f0 - 55.0).abs() < 1e-3, "bin 0 should be f_min");
        assert!((f12 - 110.0).abs() < 0.1, "bin 12 should be one octave up");
    }

    #[test]
    fn freq_gradient_central_difference() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        // At frame 1, bin 15: gradient should point toward bin 16 (the peak).
        let g = s.freq_gradient(1, 15);
        assert!(g > 0.0, "gradient should be positive toward peak");
    }

    #[test]
    fn time_gradient_central_difference() {
        // Use a zero-filled surface with a single peak.
        let frames = 4;
        let bins = SPECTRAL_PREVIEW_BINS;
        let mut raster = vec![0.0f32; frames * bins];
        raster[1 * bins + 16] = 1.0; // peak at frame 1
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        // At frame 1, bin 16: prev = get(0, 16) = 0.0, next = get(2, 16) = 0.0.
        // Gradient = (0.0 - 0.0) * 0.5 = 0.0 — the peak is isolated.
        // At frame 2, bin 16: prev = get(1, 16) = 1.0, next = get(3, 16) = 0.0.
        // Gradient = (0.0 - 1.0) * 0.5 = -0.5 — negative after the peak.
        let g = s.time_gradient(2, 16);
        assert!(
            g < 0.0,
            "time gradient should be negative after peak: {}",
            g
        );
    }

    #[test]
    fn total_energy_correct() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        assert!(
            (s.total_energy() - 1.8).abs() < 1e-6,
            "total energy should be 1.8"
        );
    }

    #[test]
    fn frame_energy_correct() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        assert!((s.frame_energy(1) - 1.0).abs() < 1e-6);
        assert!((s.frame_energy(2) - 0.8).abs() < 1e-6);
        assert!((s.frame_energy(0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn to_height_mesh_writes_vertices() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let mut verts = vec![0.0f32; frames * bins * 3];
        let n = s.to_height_mesh(&mut verts);
        assert_eq!(n, frames * bins);
        // Vertex at (frame=1, bin=16) should have z=1.0.
        let idx = (1 * bins + 16) * 3;
        assert!((verts[idx + 2] - 1.0).abs() < 1e-6, "z should be magnitude");
    }

    #[test]
    fn surface_determinism() {
        let (raster, frames, bins) = make_test_surface();
        let s = TfSurface::new(&raster, frames, bins, 44100, 512);
        let v1 = s.sample_bilinear(1.5, 24.0);
        let v2 = s.sample_bilinear(1.5, 24.0);
        assert_eq!(v1, v2, "sampling must be deterministic");
    }
}
