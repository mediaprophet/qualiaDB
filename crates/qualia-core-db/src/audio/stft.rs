//! Real forward STFT over actual audio samples — Hann-windowed framing through the
//! forge FFT (`crate::wgsl_forge::dispatch::fft_f32`, GPU best-path with CPU DFT floor).
//!
//! This is the genuine forward transform that the cold-path sidecar bake consumes:
//! [`bake_stft_sidecar_from_samples`] reduces the real magnitude spectrum to
//! `SPECTRAL_PREVIEW_BINS` and writes it through the same header machinery as the
//! preview-synthesis bake in [`crate::audio::stft_bake`] — so the sidecar derives
//! from real audio, not from a parametric preview.
//!
//! Heap allocation is fine here: STFT runs at ingest (cold path), never in the
//! zero-heap U3 hot worklet.

use crate::audio::audio_spectral_sheet::{
    AudioSpectralSidecarHeader, SIDECAR_KIND_STFT, SPECTRAL_PREVIEW_BINS, SPECTRAL_SIDECAR_MAGIC,
};
use crate::audio::stft_bake::StftBakeError;

/// Hann window coefficient `w[i] = 0.5*(1 - cos(2π·i/(N-1)))` for a length-`n` frame.
#[inline]
fn hann(i: usize, n: usize) -> f32 {
    if n <= 1 {
        return 1.0;
    }
    0.5 * (1.0 - (core::f32::consts::TAU * i as f32 / (n - 1) as f32).cos())
}

/// Real forward STFT over `samples`.
///
/// `frame_size` must be a power of two in `[2, 1024]` (the forge FFT's accelerated
/// window; the CPU DFT floor honours the same size), otherwise
/// [`StftBakeError::InvalidFrameCount`].
///
/// For each frame start `s = 0, hop, 2·hop, …` while `s + frame_size ≤ samples.len()`:
/// take `samples[s..s+frame_size]`, apply the Hann window, build the interleaved
/// complex buffer `[re = windowed, im = 0]`, run it through
/// [`crate::wgsl_forge::dispatch::fft_f32`], and collect the frame's `frame_size`
/// complex bins as `[re, im]`.
///
/// Returns one inner `Vec<[f32;2]>` (length `frame_size`) per frame.
pub fn forward_stft(
    samples: &[f32],
    frame_size: usize,
    hop: usize,
) -> Result<Vec<Vec<[f32; 2]>>, StftBakeError> {
    if !frame_size.is_power_of_two() || !(2..=1024).contains(&frame_size) {
        return Err(StftBakeError::InvalidFrameCount);
    }
    if hop == 0 {
        return Err(StftBakeError::InvalidFrameCount);
    }

    let mut frames: Vec<Vec<[f32; 2]>> = Vec::new();
    if samples.len() < frame_size {
        return Ok(frames);
    }

    // Reused per-frame interleaved-complex scratch (re, im, re, im, …).
    let mut interleaved = vec![0.0_f32; frame_size * 2];

    let mut s = 0usize;
    while s + frame_size <= samples.len() {
        let win = &samples[s..s + frame_size];
        for (i, &x) in win.iter().enumerate() {
            interleaved[2 * i] = x * hann(i, frame_size);
            interleaved[2 * i + 1] = 0.0;
        }
        // Forge FFT (GPU best-path, CPU DFT floor). On any forge error fft_f32
        // itself falls through to the CPU floor, so this only errors on a bad
        // (odd) length — which `interleaved` never has.
        let spectrum = crate::wgsl_forge::dispatch::fft_f32(&interleaved)
            .map_err(|_| StftBakeError::InvalidFrameCount)?;

        let mut frame = Vec::with_capacity(frame_size);
        for k in 0..frame_size {
            frame.push([spectrum[2 * k], spectrum[2 * k + 1]]);
        }
        frames.push(frame);

        s += hop;
    }

    Ok(frames)
}

/// Per-frame one-sided magnitude spectrum: the first `frame_size/2 + 1` bins
/// (DC … Nyquist) of `|X[k]| = sqrt(re² + im²)`.
pub fn stft_magnitudes(spec: &[Vec<[f32; 2]>]) -> Vec<Vec<f32>> {
    spec.iter()
        .map(|frame| {
            let half = frame.len() / 2 + 1;
            frame
                .iter()
                .take(half)
                .map(|&[re, im]| (re * re + im * im).sqrt())
                .collect()
        })
        .collect()
}

/// Reduce a single frame's one-sided magnitude spectrum to `SPECTRAL_PREVIEW_BINS`
/// by averaging contiguous source bins into each preview bin (group/average).
fn magnitudes_to_preview(mags: &[f32]) -> [f32; SPECTRAL_PREVIEW_BINS] {
    let mut out = [0.0_f32; SPECTRAL_PREVIEW_BINS];
    if mags.is_empty() {
        return out;
    }
    let n = mags.len();
    for (b, slot) in out.iter_mut().enumerate() {
        // Even split of [0, n) into SPECTRAL_PREVIEW_BINS contiguous groups.
        let lo = b * n / SPECTRAL_PREVIEW_BINS;
        let hi = ((b + 1) * n / SPECTRAL_PREVIEW_BINS).max(lo + 1).min(n);
        let mut sum = 0.0_f32;
        let mut cnt = 0u32;
        for &m in &mags[lo..hi] {
            sum += m;
            cnt += 1;
        }
        *slot = if cnt > 0 { sum / cnt as f32 } else { 0.0 };
    }
    out
}

/// Bake an STFT sidecar from REAL audio `samples`.
///
/// Computes the genuine forward STFT, reduces each frame's one-sided magnitude
/// spectrum to `SPECTRAL_PREVIEW_BINS` (group/average), and writes the sidecar
/// through the same [`AudioSpectralSidecarHeader`] machinery as
/// [`crate::audio::stft_bake::bake_stft_sidecar_from_preview`] — mirroring its
/// layout exactly (header + `frame_count` × `SPECTRAL_PREVIEW_BINS` f32 raster),
/// but with `_pad = SIDECAR_KIND_STFT` and frame data derived from real audio.
///
/// Returns the number of bytes written into `out`.
pub fn bake_stft_sidecar_from_samples(
    samples: &[f32],
    frame_size: usize,
    hop: usize,
    sample_rate: u32,
    out: &mut [u8],
) -> Result<usize, StftBakeError> {
    let spec = forward_stft(samples, frame_size, hop)?;
    let mags = stft_magnitudes(&spec);
    let frame_count = mags.len() as u32;
    if frame_count == 0 || frame_count > 4096 {
        return Err(StftBakeError::InvalidFrameCount);
    }

    let header = AudioSpectralSidecarHeader {
        magic: SPECTRAL_SIDECAR_MAGIC,
        version: AudioSpectralSidecarHeader::VERSION,
        _pad: SIDECAR_KIND_STFT,
        bin_count: SPECTRAL_PREVIEW_BINS as u32,
        frame_count,
        sample_rate,
    };
    let need = std::mem::size_of::<AudioSpectralSidecarHeader>() + header.payload_bytes();
    if out.len() < need {
        return Err(StftBakeError::OutputTooSmall);
    }
    out[..std::mem::size_of::<AudioSpectralSidecarHeader>()]
        .copy_from_slice(bytemuck::bytes_of(&header));
    let payload_off = std::mem::size_of::<AudioSpectralSidecarHeader>();
    for (f, frame_mags) in mags.iter().enumerate() {
        let preview = magnitudes_to_preview(frame_mags);
        let off = payload_off + f * SPECTRAL_PREVIEW_BINS * 4;
        out[off..off + SPECTRAL_PREVIEW_BINS * 4].copy_from_slice(bytemuck::cast_slice(&preview));
    }
    Ok(need)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::audio_spectral_sheet::parse_sidecar_header;

    /// A pure cosine at bin BIN must place the STFT magnitude peak at bin BIN.
    /// This also exercises the forge `fft_f32` end-to-end (GPU best-path / CPU floor).
    #[test]
    fn cosine_peaks_at_expected_bin() {
        const FRAME: usize = 64;
        const BIN: usize = 8;
        // Three full frames of a pure cosine at exactly `BIN` cycles/frame.
        let samples: Vec<f32> = (0..FRAME * 3)
            .map(|i| {
                (core::f32::consts::TAU * BIN as f32 * (i % FRAME) as f32 / FRAME as f32).cos()
            })
            .collect();
        let spec = forward_stft(&samples, FRAME, FRAME).expect("stft");
        let mags = stft_magnitudes(&spec);
        assert!(!mags.is_empty());
        // Peak bin of the first frame (one-sided spectrum has FRAME/2+1 = 33 bins).
        let first = &mags[0];
        let (peak, _) = first
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        assert!(
            peak.abs_diff(BIN) <= 1,
            "expected STFT peak near bin {BIN}, got {peak}"
        );
    }

    /// Frame count must equal (len - frame_size)/hop + 1.
    #[test]
    fn frame_count_matches_formula() {
        const FRAME: usize = 64;
        const HOP: usize = 16;
        let samples = vec![0.25_f32; 64 * 5 + 7];
        let spec = forward_stft(&samples, FRAME, HOP).expect("stft");
        let expected = (samples.len() - FRAME) / HOP + 1;
        assert_eq!(spec.len(), expected, "frame count mismatch");
    }

    #[test]
    fn rejects_non_power_of_two_frame() {
        let samples = vec![0.0_f32; 256];
        assert_eq!(
            forward_stft(&samples, 48, 16),
            Err(StftBakeError::InvalidFrameCount)
        );
        assert_eq!(
            forward_stft(&samples, 2048, 16),
            Err(StftBakeError::InvalidFrameCount)
        );
    }

    #[test]
    fn magnitudes_are_one_sided() {
        const FRAME: usize = 64;
        let samples = vec![0.1_f32; FRAME * 2];
        let spec = forward_stft(&samples, FRAME, FRAME).expect("stft");
        let mags = stft_magnitudes(&spec);
        assert_eq!(mags[0].len(), FRAME / 2 + 1);
    }

    #[test]
    fn bake_from_real_samples_produces_valid_stft_header() {
        const FRAME: usize = 64;
        const BIN: usize = 8;
        let samples: Vec<f32> = (0..FRAME * 4)
            .map(|i| {
                (core::f32::consts::TAU * BIN as f32 * (i % FRAME) as f32 / FRAME as f32).cos()
            })
            .collect();
        // Header (20 bytes) + up to 4 frames × 64 bins × 4 bytes.
        let mut buf = [0u8; 20 + 64 * 4 * 4];
        let n = bake_stft_sidecar_from_samples(&samples, FRAME, FRAME, 44_100, &mut buf)
            .expect("bake");
        let h = parse_sidecar_header(&buf).expect("valid header");
        assert_eq!(h.bin_count, SPECTRAL_PREVIEW_BINS as u32);
        assert_eq!(h._pad, SIDECAR_KIND_STFT);
        assert_eq!(h.sample_rate, 44_100);
        assert!(h.frame_count >= 1);
        assert_eq!(
            n,
            std::mem::size_of::<AudioSpectralSidecarHeader>()
                + h.frame_count as usize * SPECTRAL_PREVIEW_BINS * 4
        );
        // The baked preview carries real energy (cosine is not silence).
        let payload_off = std::mem::size_of::<AudioSpectralSidecarHeader>();
        let frame0: &[f32] =
            bytemuck::cast_slice(&buf[payload_off..payload_off + SPECTRAL_PREVIEW_BINS * 4]);
        assert!(frame0.iter().any(|&v| v > 0.0), "real STFT energy present");
    }
}
