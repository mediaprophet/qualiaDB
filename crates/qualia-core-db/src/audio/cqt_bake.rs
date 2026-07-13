//! Constant-Q transform sidecar bake — log-spaced bins for timbral integrity (cold path).

use crate::audio::audio_spectral_sheet::{
    AudioSpectralSidecarHeader, SIDECAR_KIND_CQT, SPECTRAL_PREVIEW_BINS, SPECTRAL_SIDECAR_MAGIC,
};
use crate::audio::stft_bake::StftBakeError;

/// Map linear preview energy into log-spaced CQT bins (MIDI-ish spacing across 64 bins).
#[inline]
pub fn preview_to_cqt_frame(
    preview: &[f32; SPECTRAL_PREVIEW_BINS],
    frame_index: u32,
    frame_count: u32,
) -> [f32; SPECTRAL_PREVIEW_BINS] {
    let mut out = [0.0_f32; SPECTRAL_PREVIEW_BINS];
    let phase = (frame_index as f32 / frame_count.max(1) as f32) * core::f32::consts::TAU;
    for (i, o) in out.iter_mut().enumerate() {
        let log_i = (i as f32 + 1.0).ln() / (SPECTRAL_PREVIEW_BINS as f32 + 1.0).ln();
        let src_idx = (log_i * (SPECTRAL_PREVIEW_BINS - 1) as f32).round() as usize;
        let base = preview[src_idx.min(SPECTRAL_PREVIEW_BINS - 1)];
        let shimmer = (phase * (i as f32 + 1.0) * 0.05).sin() * 0.08;
        *o = (base * (1.0 + shimmer)).max(0.0);
    }
    out
}

/// Bake CQT sidecar into caller buffer (`_pad` = `SIDECAR_KIND_CQT`).
pub fn bake_cqt_sidecar_from_preview(
    preview: &[f32; SPECTRAL_PREVIEW_BINS],
    frame_count: u32,
    sample_rate: u32,
    out: &mut [u8],
) -> Result<usize, StftBakeError> {
    if frame_count == 0 || frame_count > 4096 {
        return Err(StftBakeError::InvalidFrameCount);
    }
    let header = AudioSpectralSidecarHeader {
        magic: SPECTRAL_SIDECAR_MAGIC,
        version: AudioSpectralSidecarHeader::VERSION,
        _pad: SIDECAR_KIND_CQT,
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
    for f in 0..frame_count {
        let frame = preview_to_cqt_frame(preview, f, frame_count);
        let off = payload_off + f as usize * SPECTRAL_PREVIEW_BINS * 4;
        out[off..off + SPECTRAL_PREVIEW_BINS * 4].copy_from_slice(bytemuck::cast_slice(&frame));
    }
    Ok(need)
}

/// Real forward constant-Q transform over actual audio `samples` — direct
/// constant-Q (one complex inner product per log-spaced bin; CPU is fine, this
/// is a cold-path ingest transform).
///
/// Geometric bin centres `f_k = f_min · 2^(k/bins_per_octave)`, constant quality
/// factor `Q = 1 / (2^(1/bins_per_octave) − 1)`. Each bin uses a Hann-windowed
/// kernel of length `n_k = round(Q · sample_rate / f_k)` (clamped to
/// `[1, samples.len()]`):
///
/// `X_k = (1/n_k) · Σ_{j<n_k} samples[j] · hann(j, n_k) · exp(−2πi · f_k · j / sample_rate)`
///
/// and the returned vector holds `|X_k|` for `k ∈ [0, n_bins)`.
pub fn forward_cqt(
    samples: &[f32],
    sample_rate: f32,
    f_min: f32,
    bins_per_octave: usize,
    n_bins: usize,
) -> Vec<f32> {
    let mut out = Vec::with_capacity(n_bins);
    if samples.is_empty() || bins_per_octave == 0 || sample_rate <= 0.0 || f_min <= 0.0 {
        out.resize(n_bins, 0.0);
        return out;
    }
    // Constant quality factor for the chosen resolution.
    let q = 1.0_f32 / (2.0_f32.powf(1.0 / bins_per_octave as f32) - 1.0);

    for k in 0..n_bins {
        let f_k = f_min * 2.0_f32.powf(k as f32 / bins_per_octave as f32);
        // Window length tracks the bin frequency (constant-Q: more cycles at HF
        // would need fewer samples; here longer windows at LF).
        let nk = ((q * sample_rate / f_k).round() as usize).clamp(1, samples.len());

        let mut acc_re = 0.0_f32;
        let mut acc_im = 0.0_f32;
        for j in 0..nk {
            let w = hann(j, nk);
            // exp(-2πi · f_k · j / sample_rate)
            let theta = -core::f32::consts::TAU * f_k * j as f32 / sample_rate;
            let s = samples[j] * w;
            acc_re += s * theta.cos();
            acc_im += s * theta.sin();
        }
        let inv = 1.0 / nk as f32;
        acc_re *= inv;
        acc_im *= inv;
        out.push((acc_re * acc_re + acc_im * acc_im).sqrt());
    }
    out
}

/// Hann window `w[i] = 0.5*(1 - cos(2π·i/(n-1)))` over a length-`n` kernel.
#[inline]
fn hann(i: usize, n: usize) -> f32 {
    if n <= 1 {
        return 1.0;
    }
    0.5 * (1.0 - (core::f32::consts::TAU * i as f32 / (n - 1) as f32).cos())
}

/// Reduce `n_bins` CQT magnitudes to `SPECTRAL_PREVIEW_BINS` by group-averaging
/// contiguous bins (or copy directly when `mags.len() == SPECTRAL_PREVIEW_BINS`).
fn cqt_to_preview(mags: &[f32]) -> [f32; SPECTRAL_PREVIEW_BINS] {
    let mut out = [0.0_f32; SPECTRAL_PREVIEW_BINS];
    if mags.is_empty() {
        return out;
    }
    if mags.len() == SPECTRAL_PREVIEW_BINS {
        out.copy_from_slice(mags);
        return out;
    }
    let n = mags.len();
    for (b, slot) in out.iter_mut().enumerate() {
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

/// Bake a CQT sidecar from REAL audio `samples`: compute the genuine forward CQT,
/// reduce `n_bins` → `SPECTRAL_PREVIEW_BINS`, and write a single-frame sidecar
/// through the same header machinery as [`bake_cqt_sidecar_from_preview`]
/// (`_pad = SIDECAR_KIND_CQT`). When `n_bins == SPECTRAL_PREVIEW_BINS` the
/// magnitudes are used directly.
pub fn bake_cqt_sidecar_from_samples(
    samples: &[f32],
    sample_rate: f32,
    f_min: f32,
    bins_per_octave: usize,
    n_bins: usize,
    out: &mut [u8],
) -> Result<usize, StftBakeError> {
    let mags = forward_cqt(samples, sample_rate, f_min, bins_per_octave, n_bins);
    let preview = cqt_to_preview(&mags);

    let header = AudioSpectralSidecarHeader {
        magic: SPECTRAL_SIDECAR_MAGIC,
        version: AudioSpectralSidecarHeader::VERSION,
        _pad: SIDECAR_KIND_CQT,
        bin_count: SPECTRAL_PREVIEW_BINS as u32,
        frame_count: 1,
        sample_rate: sample_rate.round().max(0.0) as u32,
    };
    let need = std::mem::size_of::<AudioSpectralSidecarHeader>() + header.payload_bytes();
    if out.len() < need {
        return Err(StftBakeError::OutputTooSmall);
    }
    out[..std::mem::size_of::<AudioSpectralSidecarHeader>()]
        .copy_from_slice(bytemuck::bytes_of(&header));
    let payload_off = std::mem::size_of::<AudioSpectralSidecarHeader>();
    out[payload_off..payload_off + SPECTRAL_PREVIEW_BINS * 4]
        .copy_from_slice(bytemuck::cast_slice(&preview));
    Ok(need)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::audio_spectral_sheet::parse_sidecar_header;

    #[test]
    fn cqt_header_kind_flag() {
        let preview = [0.4_f32; SPECTRAL_PREVIEW_BINS];
        let mut buf = [0u8; 20 + 64 * 4 * 4];
        let n = bake_cqt_sidecar_from_preview(&preview, 4, 48_000, &mut buf).unwrap();
        assert!(n > 20);
        let h = parse_sidecar_header(&buf).unwrap();
        assert_eq!(h._pad, SIDECAR_KIND_CQT);
    }

    #[test]
    fn cqt_bins_log_spread_nonzero() {
        let mut preview = [0.0_f32; SPECTRAL_PREVIEW_BINS];
        preview[0] = 1.0;
        preview[63] = 0.5;
        let cqt = preview_to_cqt_frame(&preview, 0, 8);
        assert!(cqt[0] > 0.0, "low CQT bins sample preview[0]");
        assert!(
            cqt[63] > 0.0,
            "high CQT bins sample preview[63] via log map"
        );
    }

    /// A 440 Hz tone must peak at the CQT bin nearest 12·log2(440/55) = 36.
    #[test]
    fn forward_cqt_tone_lands_on_expected_bin() {
        let sample_rate = 44_100.0_f32;
        let f_min = 55.0_f32;
        let bins_per_octave = 12usize;
        let n_bins = 48usize;
        let freq = 440.0_f32;
        // ~0.25 s of a pure 440 Hz tone — long enough for the LF kernels.
        let n = 11_025usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| (core::f32::consts::TAU * freq * i as f32 / sample_rate).sin())
            .collect();
        let mags = forward_cqt(&samples, sample_rate, f_min, bins_per_octave, n_bins);
        assert_eq!(mags.len(), n_bins);
        let (peak, _) = mags
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        let expected = (bins_per_octave as f32 * (freq / f_min).log2()).round() as usize; // 36
        assert!(
            peak.abs_diff(expected) <= 1,
            "expected CQT peak near bin {expected}, got {peak}"
        );
    }

    #[test]
    fn bake_cqt_from_samples_valid_header() {
        let sample_rate = 44_100.0_f32;
        let n = 8_192usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| (core::f32::consts::TAU * 220.0 * i as f32 / sample_rate).sin())
            .collect();
        let mut buf = [0u8; 20 + 64 * 4];
        let written = bake_cqt_sidecar_from_samples(&samples, sample_rate, 55.0, 12, 48, &mut buf)
            .expect("bake cqt");
        let h = parse_sidecar_header(&buf).expect("valid header");
        assert_eq!(h._pad, SIDECAR_KIND_CQT);
        assert_eq!(h.bin_count, SPECTRAL_PREVIEW_BINS as u32);
        assert_eq!(h.frame_count, 1);
        assert_eq!(h.sample_rate, 44_100);
        assert_eq!(
            written,
            std::mem::size_of::<AudioSpectralSidecarHeader>() + SPECTRAL_PREVIEW_BINS * 4
        );
    }

    /// When n_bins == SPECTRAL_PREVIEW_BINS the preview is the magnitudes verbatim.
    #[test]
    fn cqt_preview_passthrough_when_64_bins() {
        let mags: Vec<f32> = (0..SPECTRAL_PREVIEW_BINS)
            .map(|i| i as f32 * 0.01)
            .collect();
        let preview = cqt_to_preview(&mags);
        for (a, b) in preview.iter().zip(mags.iter()) {
            assert!((a - b).abs() < 1e-9);
        }
    }
}
