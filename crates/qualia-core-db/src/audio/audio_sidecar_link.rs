//! Cold-path audio sidecar linker — bake, hash, mmap-ready bytes, optional native file write.
//!
//! Links baked tensors to `q42:hasSpectralSheet` NQuin objects at ingest.

use crate::audio::audio_spectral_sheet::{
    copy_sidecar_frame_to_preview_bins, SPECTRAL_PREVIEW_BINS,
};
use crate::audio::cqt_bake::bake_cqt_sidecar_from_preview;
use crate::audio::stft_bake::{bake_stft_sidecar_from_preview, StftBakeError};
use crate::tensor::bake_pipeline::{audio_sidecar_relpath, PRED_HAS_SPECTRAL_SHEET};
use crate::tensor::Tensor10D;
use crate::NQuin;
use crate::audio::audio_spectral_sheet::preview_bins_from_tensor;

/// FNV-1a over preview bins — stable sidecar filename hash.
#[inline]
pub fn sidecar_content_hash(preview: &[f32; SPECTRAL_PREVIEW_BINS]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in preview {
        let bits = b.to_bits() as u64;
        h ^= bits;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidecarBakeKind {
    Stft,
    Cqt,
}

/// Bake sidecar bytes into `out`; returns (content_hash, bytes_written).
pub fn bake_audio_sidecar_into(
    preview: &[f32; SPECTRAL_PREVIEW_BINS],
    frame_count: u32,
    kind: SidecarBakeKind,
    out: &mut [u8],
) -> Result<(u64, usize), StftBakeError> {
    let hash = sidecar_content_hash(preview);
    let n = match kind {
        SidecarBakeKind::Stft => bake_stft_sidecar_from_preview(preview, frame_count, 48_000, out)?,
        SidecarBakeKind::Cqt => bake_cqt_sidecar_from_preview(preview, frame_count, 48_000, out)?,
    };
    Ok((hash, n))
}

/// Emit linker NQuin: subject → `q42:hasSpectralSheet` → sheet index (lower 60 bits of hash).
#[inline]
pub fn compile_spectral_sheet_quin(subject_hash: u64, sheet_index: u32) -> NQuin {
    let mut q = NQuin::default();
    q.subject = subject_hash;
    q.predicate = PRED_HAS_SPECTRAL_SHEET;
    q.object = (sheet_index as u64) & 0x0FFF_FFFF_FFFF_FFFF;
    q
}

/// Write relative path `spectral/audio/{hash:016x}.bin` into caller buffer; returns length.
#[inline]
pub fn format_sidecar_relpath(content_hash: u64, out: &mut [u8]) -> usize {
    audio_sidecar_relpath(content_hash, out)
}

/// Bake from tensor preview + link quin for cold ingest pipelines.
pub fn link_tensor_audio_sidecar(
    t: &Tensor10D,
    subject_hash: u64,
    frame_count: u32,
    kind: SidecarBakeKind,
    out_bytes: &mut [u8],
) -> Result<(NQuin, u64, usize), StftBakeError> {
    let preview = preview_bins_from_tensor(t);
    let (hash, n) = bake_audio_sidecar_into(&preview, frame_count, kind, out_bytes)?;
    let index = (hash & 0xffff_ffff) as u32;
    let quin = compile_spectral_sheet_quin(subject_hash, index);
    Ok((quin, hash, n))
}

/// Hot path: overlay mmap/CQT/STFT column into uniform preview bins.
#[inline]
pub fn enrich_preview_from_sidecar(
    sidecar_bytes: &[u8],
    frame_index: u32,
    preview: &mut [f32; SPECTRAL_PREVIEW_BINS],
) -> bool {
    copy_sidecar_frame_to_preview_bins(sidecar_bytes, frame_index, preview)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_sidecar_file(
    storage_root: &std::path::Path,
    content_hash: u64,
    bytes: &[u8],
) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    let mut rel = [0u8; 64];
    let n = format_sidecar_relpath(content_hash, &mut rel);
    let rel_str = std::str::from_utf8(&rel[..n]).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let path = storage_root.join(rel_str);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(&path)?;
    f.write_all(bytes)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_stable_for_same_preview() {
        let p = [0.5_f32; SPECTRAL_PREVIEW_BINS];
        assert_eq!(sidecar_content_hash(&p), sidecar_content_hash(&p));
    }

    #[test]
    fn link_emits_sheet_quin() {
        let t = Tensor10D::new(0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.55);
        let mut buf = [0u8; 20 + 64 * 16 * 4];
        let (q, _, n) = link_tensor_audio_sidecar(
            &t,
            0xabc,
            16,
            SidecarBakeKind::Cqt,
            &mut buf,
        )
        .unwrap();
        assert!(n > 20);
        assert_eq!(q.subject, 0xabc);
        assert_eq!(q.predicate, PRED_HAS_SPECTRAL_SHEET);
    }

    #[test]
    fn enrich_preview_from_baked_sidecar() {
        let preview = [0.7_f32; SPECTRAL_PREVIEW_BINS];
        let mut buf = [0u8; 20 + 64 * 4 * 4];
        bake_stft_sidecar_from_preview(&preview, 4, 48_000, &mut buf).unwrap();
        let mut out = [0.0_f32; SPECTRAL_PREVIEW_BINS];
        assert!(enrich_preview_from_sidecar(&buf, 2, &mut out));
        assert!(out[0] > 0.0);
    }
}