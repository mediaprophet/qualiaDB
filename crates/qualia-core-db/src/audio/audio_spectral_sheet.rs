//! Spectral-first audio truth layer — mmap sidecar headers + zero-heap hot views.
//!
//! Cold path: STFT/CQT sheets under `{storage}/spectral/audio/{hash}.bin`.
//! Hot path: fixed `SPECTRAL_PREVIEW_BINS` stack preview derived from `Tensor10D`.

use bytemuck::{Pod, Zeroable};

use crate::tensor::Tensor10D;

pub const SPECTRAL_SIDECAR_MAGIC: u32 = 0x5134_4155; // "Q4AU"
pub const SPECTRAL_PREVIEW_BINS: usize = 64;

/// Sidecar transform kind stored in `AudioSpectralSidecarHeader._pad`.
pub const SIDECAR_KIND_STFT: u16 = 0;
pub const SIDECAR_KIND_CQT: u16 = 1;

/// Cold mmap header for STFT/CQT sidecars (frames follow as `f32` raster).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct AudioSpectralSidecarHeader {
    pub magic: u32,
    pub version: u16,
    pub _pad: u16,
    pub bin_count: u32,
    pub frame_count: u32,
    pub sample_rate: u32,
}

impl AudioSpectralSidecarHeader {
    pub const VERSION: u16 = 1;

    #[inline]
    pub const fn empty() -> Self {
        Self {
            magic: SPECTRAL_SIDECAR_MAGIC,
            version: Self::VERSION,
            _pad: 0,
            bin_count: 0,
            frame_count: 0,
            sample_rate: 48_000,
        }
    }

    #[inline]
    pub fn payload_bytes(&self) -> usize {
        (self.bin_count as usize)
            .saturating_mul(self.frame_count as usize)
            .saturating_mul(4)
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.magic == SPECTRAL_SIDECAR_MAGIC
            && self.version == Self::VERSION
            && self.bin_count > 0
            && self.frame_count > 0
    }
}

/// Parse sidecar header from a byte slice (cold ingest / mmap pin).
#[inline]
pub fn parse_sidecar_header(bytes: &[u8]) -> Option<AudioSpectralSidecarHeader> {
    if bytes.len() < std::mem::size_of::<AudioSpectralSidecarHeader>() {
        return None;
    }
    let header = bytemuck::pod_read_unaligned::<AudioSpectralSidecarHeader>(
        &bytes[..std::mem::size_of::<AudioSpectralSidecarHeader>()],
    );
    header.is_valid().then_some(header)
}

/// Zero-heap generative sheet view for U3 worklet / parametric DSP.
#[derive(Debug, Clone, Copy)]
pub struct AudioSpectralSheetView<'a> {
    pub alpha: f32,
    pub mu: f32,
    pub bins: &'a [f32],
    pub position: [f32; 3],
    pub track_v: u8,
    pub manifold_w: u8,
}

impl<'a> AudioSpectralSheetView<'a> {
    #[inline]
    pub fn from_tensor_preview(t: &Tensor10D, bins: &'a [f32]) -> Self {
        Self {
            alpha: t.alpha,
            mu: t.mu,
            bins,
            position: [t.x, t.y, t.z],
            track_v: t.v.clamp(0.0, 255.0) as u8,
            manifold_w: t.w.clamp(0.0, 255.0) as u8,
        }
    }
}

/// Map 10D tensor channels to 64-bin spectral preview (stack only, no heap).
#[inline]
pub fn preview_bins_from_tensor(t: &Tensor10D) -> [f32; SPECTRAL_PREVIEW_BINS] {
    let src = [t.q, t.v, t.w, t.x, t.y, t.z, t.t, t.alpha, t.mu, t.sigma];
    let mut out = [0.0_f32; SPECTRAL_PREVIEW_BINS];
    for i in 0..SPECTRAL_PREVIEW_BINS {
        let f = (i as f32 / SPECTRAL_PREVIEW_BINS as f32) * 10.0;
        let idx = f.floor() as usize;
        let next = (idx + 1).min(9);
        let frac = f - idx as f32;
        out[i] = src[idx] * (1.0 - frac) + src[next] * frac;
    }
    out
}

/// Frame column from mmap sidecar at `frame_index` (cold playback).
#[inline]
pub fn sidecar_frame_view<'a>(
    bytes: &'a [u8],
    frame_index: u32,
) -> Option<&'a [f32]> {
    let header = parse_sidecar_header(bytes)?;
    let header_bytes = std::mem::size_of::<AudioSpectralSidecarHeader>();
    let payload = &bytes[header_bytes..];
    if frame_index >= header.frame_count {
        return None;
    }
    let bins = header.bin_count as usize;
    let offset = frame_index as usize * bins;
    let need = offset.saturating_add(bins).saturating_mul(4);
    if payload.len() < need {
        return None;
    }
    let floats: &[f32] = bytemuck::try_cast_slice(&payload[offset * 4..offset * 4 + bins * 4]).ok()?;
    Some(floats)
}

/// Copy one sidecar frame into a fixed preview bin array (hot U3 path, zero-heap).
#[inline]
pub fn copy_sidecar_frame_to_preview_bins(
    bytes: &[u8],
    frame_index: u32,
    out: &mut [f32; SPECTRAL_PREVIEW_BINS],
) -> bool {
    let Some(frame) = sidecar_frame_view(bytes, frame_index) else {
        return false;
    };
    let n = frame.len().min(SPECTRAL_PREVIEW_BINS);
    out[..n].copy_from_slice(&frame[..n]);
    for slot in out.iter_mut().skip(n) {
        *slot = 0.0;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_header_size_and_magic() {
        assert_eq!(std::mem::size_of::<AudioSpectralSidecarHeader>(), 20);
        let h = AudioSpectralSidecarHeader::empty();
        assert_eq!(h.magic, SPECTRAL_SIDECAR_MAGIC);
    }

    #[test]
    fn preview_bins_len_fixed() {
        let t = Tensor10D::new(0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0);
        let bins = preview_bins_from_tensor(&t);
        assert_eq!(bins.len(), SPECTRAL_PREVIEW_BINS);
        assert!(bins[0] > 0.0);
        assert!(bins[63] > 0.0);
    }

    #[test]
    fn parse_header_rejects_short_slice() {
        assert!(parse_sidecar_header(&[0u8; 8]).is_none());
    }

    #[test]
    fn parse_header_accepts_valid() {
        let h = AudioSpectralSidecarHeader {
            magic: SPECTRAL_SIDECAR_MAGIC,
            version: 1,
            _pad: 0,
            bin_count: 64,
            frame_count: 100,
            sample_rate: 48_000,
        };
        let bytes = bytemuck::bytes_of(&h);
        let parsed = parse_sidecar_header(bytes).expect("valid header");
        assert_eq!(parsed.bin_count, 64);
        assert_eq!(parsed.payload_bytes(), 64 * 100 * 4);
    }
}