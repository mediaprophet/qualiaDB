//! Cold-path STFT/CQT sidecar bake — preview bins → mmap-ready `AudioSpectralSidecarHeader` payload.
//!
//! Runs at ingest only (heap OK in caller buffer). Hot U3 reads mmap or preview bins.

use crate::audio::audio_spectral_sheet::{
    AudioSpectralSidecarHeader, SPECTRAL_PREVIEW_BINS, SPECTRAL_SIDECAR_MAGIC, SIDECAR_KIND_STFT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StftBakeError {
    OutputTooSmall,
    InvalidFrameCount,
}

/// Synthesize one STFT frame from preview bins with temporal phase evolution.
#[inline]
pub fn synthesize_stft_frame(
    preview: &[f32; SPECTRAL_PREVIEW_BINS],
    frame_index: u32,
    frame_count: u32,
) -> [f32; SPECTRAL_PREVIEW_BINS] {
    let mut out = [0.0_f32; SPECTRAL_PREVIEW_BINS];
    let phase = (frame_index as f32 / frame_count.max(1) as f32) * std::f32::consts::TAU;
    for (i, o) in out.iter_mut().enumerate() {
        let base = preview[i];
        let wobble = (phase * (i as f32 + 1.0) * 0.07).sin() * 0.12;
        *o = (base * (1.0 + wobble)).max(0.0);
    }
    out
}

/// Bake STFT sidecar bytes into caller `out` (header + `frame_count` × `bin_count` f32 raster).
pub fn bake_stft_sidecar_from_preview(
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
        _pad: SIDECAR_KIND_STFT,
        bin_count: SPECTRAL_PREVIEW_BINS as u32,
        frame_count,
        sample_rate,
    };
    let need = std::mem::size_of::<AudioSpectralSidecarHeader>()
        + header.payload_bytes();
    if out.len() < need {
        return Err(StftBakeError::OutputTooSmall);
    }
    out[..std::mem::size_of::<AudioSpectralSidecarHeader>()]
        .copy_from_slice(bytemuck::bytes_of(&header));
    let payload_off = std::mem::size_of::<AudioSpectralSidecarHeader>();
    for f in 0..frame_count {
        let frame = synthesize_stft_frame(preview, f, frame_count);
        let off = payload_off + f as usize * SPECTRAL_PREVIEW_BINS * 4;
        out[off..off + SPECTRAL_PREVIEW_BINS * 4]
            .copy_from_slice(bytemuck::cast_slice(&frame));
    }
    Ok(need)
}

/// Bake from `Tensor10D` preview channels (cold ingest linker).
#[inline]
pub fn bake_tensor_stft_sidecar(
    preview: &[f32; SPECTRAL_PREVIEW_BINS],
    frame_count: u32,
    out: &mut [u8],
) -> Result<usize, StftBakeError> {
    bake_stft_sidecar_from_preview(preview, frame_count, 48_000, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::audio_spectral_sheet::parse_sidecar_header;
    use crate::tensor::Tensor10D;
    use crate::audio::audio_spectral_sheet::preview_bins_from_tensor;

    #[test]
    fn bake_produces_valid_header() {
        let t = Tensor10D::new(0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.55);
        let preview = preview_bins_from_tensor(&t);
        const HEADER_BYTES: usize = 20;
        let mut buf = [0u8; HEADER_BYTES + 64 * 8 * 4];
        let n = bake_tensor_stft_sidecar(&preview, 8, &mut buf).unwrap();
        assert_eq!(n, HEADER_BYTES + 64 * 8 * 4);
        let h = parse_sidecar_header(&buf).unwrap();
        assert_eq!(h.frame_count, 8);
        assert_eq!(h.bin_count, 64);
    }

    #[test]
    fn frame_synthesis_nonzero() {
        let preview = [0.5_f32; SPECTRAL_PREVIEW_BINS];
        let f = synthesize_stft_frame(&preview, 3, 16);
        assert!(f.iter().any(|&v| v > 0.0));
    }
}