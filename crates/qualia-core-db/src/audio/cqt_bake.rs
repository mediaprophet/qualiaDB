//! Constant-Q transform sidecar bake — log-spaced bins for timbral sovereignty (cold path).

use crate::audio::audio_spectral_sheet::{
    AudioSpectralSidecarHeader, SPECTRAL_PREVIEW_BINS, SPECTRAL_SIDECAR_MAGIC, SIDECAR_KIND_CQT,
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
        assert!(cqt[63] > 0.0, "high CQT bins sample preview[63] via log map");
    }
}