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
    /// Q4AU-v2: plane-0 (unchanged v1 layout) plus appended mel + MFCC planes.
    /// See [`bake_spectral_v2_from_samples`] and the [`SpectralV2SubHeader`] layout.
    pub const VERSION_V2: u16 = 2;

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
            && (self.version == Self::VERSION || self.version == Self::VERSION_V2)
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
pub fn sidecar_frame_view<'a>(bytes: &'a [u8], frame_index: u32) -> Option<&'a [f32]> {
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
    let floats: &[f32] =
        bytemuck::try_cast_slice(&payload[offset * 4..offset * 4 + bins * 4]).ok()?;
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

// ----------------------------------------------------------------------------
// Q4AU-v2 sidecar planes: mel + MFCC appended after the unchanged v1 plane-0.
// ----------------------------------------------------------------------------

/// Magic for the v2 sub-header that precedes the mel/MFCC planes ("Q4M2").
pub const SPECTRAL_V2_SUBHEADER_MAGIC: u32 = 0x324D_3451;

/// Fixed 12-byte sub-header introducing the appended v2 planes. Sits immediately
/// after plane-0 (`header + frame_count × bin_count` f32); followed by the mel
/// plane (`frame_count × n_mel` f32) then the MFCC plane (`frame_count × n_mfcc`
/// f32). All f32, row-major by frame.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SpectralV2SubHeader {
    pub magic: u32,
    pub n_mel: u32,
    pub n_mfcc: u32,
}

impl SpectralV2SubHeader {
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.magic == SPECTRAL_V2_SUBHEADER_MAGIC && self.n_mel > 0 && self.n_mfcc > 0
    }
}

/// Byte offset of the v2 sub-header (end of plane-0). `None` unless the header is
/// a valid v2 header and plane-0 fits within `bytes`.
#[inline]
fn v2_subheader_offset(bytes: &[u8]) -> Option<(AudioSpectralSidecarHeader, usize)> {
    let header = parse_sidecar_header(bytes)?;
    if header.version != AudioSpectralSidecarHeader::VERSION_V2 {
        return None;
    }
    let header_bytes = std::mem::size_of::<AudioSpectralSidecarHeader>();
    let plane0_end = header_bytes.checked_add(header.payload_bytes())?;
    if bytes.len() < plane0_end.checked_add(std::mem::size_of::<SpectralV2SubHeader>())? {
        return None;
    }
    Some((header, plane0_end))
}

/// Parse the v2 sub-header from a v2 sidecar (validates magic and counts).
#[inline]
pub fn parse_v2_subheader(bytes: &[u8]) -> Option<SpectralV2SubHeader> {
    let (_, off) = v2_subheader_offset(bytes)?;
    let sub = bytemuck::pod_read_unaligned::<SpectralV2SubHeader>(
        &bytes[off..off + std::mem::size_of::<SpectralV2SubHeader>()],
    );
    sub.is_valid().then_some(sub)
}

/// Mel band vector for `frame_index` from a v2 sidecar (`n_mel` f32), or `None`
/// if the file is not a valid v2 sidecar or the index/planes are out of range.
#[inline]
pub fn sidecar_mel_frame_view(bytes: &[u8], frame_index: u32) -> Option<&[f32]> {
    let (header, sub_off) = v2_subheader_offset(bytes)?;
    let sub_size = std::mem::size_of::<SpectralV2SubHeader>();
    let sub =
        bytemuck::pod_read_unaligned::<SpectralV2SubHeader>(&bytes[sub_off..sub_off + sub_size]);
    if !sub.is_valid() || frame_index >= header.frame_count {
        return None;
    }
    let n_mel = sub.n_mel as usize;
    let mel_base = sub_off + sub_size;
    let off = mel_base + frame_index as usize * n_mel * 4;
    let end = off.checked_add(n_mel * 4)?;
    if bytes.len() < end {
        return None;
    }
    bytemuck::try_cast_slice(&bytes[off..end]).ok()
}

/// MFCC coefficient vector for `frame_index` from a v2 sidecar (`n_mfcc` f32), or
/// `None` if the file is not a valid v2 sidecar or the index/planes are out of range.
#[inline]
pub fn sidecar_mfcc_frame_view(bytes: &[u8], frame_index: u32) -> Option<&[f32]> {
    let (header, sub_off) = v2_subheader_offset(bytes)?;
    let sub_size = std::mem::size_of::<SpectralV2SubHeader>();
    let sub =
        bytemuck::pod_read_unaligned::<SpectralV2SubHeader>(&bytes[sub_off..sub_off + sub_size]);
    if !sub.is_valid() || frame_index >= header.frame_count {
        return None;
    }
    let n_mel = sub.n_mel as usize;
    let n_mfcc = sub.n_mfcc as usize;
    let mel_bytes = header.frame_count as usize * n_mel * 4;
    let mfcc_base = sub_off + sub_size + mel_bytes;
    let off = mfcc_base + frame_index as usize * n_mfcc * 4;
    let end = off.checked_add(n_mfcc * 4)?;
    if bytes.len() < end {
        return None;
    }
    bytemuck::try_cast_slice(&bytes[off..end]).ok()
}

/// Total byte size of a Q4AU-v2 sidecar with the given geometry.
#[inline]
pub fn v2_sidecar_size(frame_count: usize, bin_count: usize, n_mel: usize, n_mfcc: usize) -> usize {
    std::mem::size_of::<AudioSpectralSidecarHeader>()
        + frame_count * bin_count * 4
        + std::mem::size_of::<SpectralV2SubHeader>()
        + frame_count * n_mel * 4
        + frame_count * n_mfcc * 4
}

/// Bake a Q4AU-**v2** sidecar from REAL audio `samples`.
///
/// Layout: the unchanged v1 header (`version = 2`, `bin_count = frame_size/2 + 1`,
/// `_pad = SIDECAR_KIND_STFT`) + plane-0 (the one-sided STFT magnitude spectrum,
/// `frame_count × bin_count` f32, from [`forward_stft`] + [`stft_magnitudes`]);
/// then a [`SpectralV2SubHeader`]; then a mel plane (`frame_count × n_mel`) and an
/// MFCC plane (`frame_count × n_mfcc`) computed per frame from the power spectrum
/// (`|X|²`) via a triangular mel bank built once with
/// [`build_mel_bank`](qualia_audio::features::mel::build_mel_bank)
/// (`0 … sample_rate/2` Hz).
///
/// A v1 reader reads plane-0 correctly and stops at [`payload_bytes`] — the
/// appended planes are invisible to it.
///
/// Returns the number of bytes written into `out`. Native only.
#[cfg(not(target_arch = "wasm32"))]
pub fn bake_spectral_v2_from_samples(
    samples: &[f32],
    frame_size: usize,
    hop: usize,
    sample_rate: u32,
    n_mel: usize,
    n_mfcc: usize,
    out: &mut [u8],
) -> Result<usize, crate::audio::stft_bake::StftBakeError> {
    use crate::audio::stft::{forward_stft, stft_magnitudes};
    use crate::audio::stft_bake::StftBakeError;
    use qualia_audio::features::mel::{build_mel_bank, mel_bands, mfcc};

    if n_mel == 0 || n_mfcc == 0 || n_mfcc > n_mel {
        return Err(StftBakeError::InvalidFrameCount);
    }

    let spec = forward_stft(samples, frame_size, hop)?;
    let mags = stft_magnitudes(&spec);
    let frame_count = mags.len();
    if frame_count == 0 || frame_count > 4096 {
        return Err(StftBakeError::InvalidFrameCount);
    }
    let bin_count = frame_size / 2 + 1;

    let need = v2_sidecar_size(frame_count, bin_count, n_mel, n_mfcc);
    if out.len() < need {
        return Err(StftBakeError::OutputTooSmall);
    }

    // Triangular mel bank over the one-sided spectrum, built once (cold path).
    let mut bank = vec![0.0f32; n_mel * bin_count];
    build_mel_bank(
        bin_count,
        n_mel,
        sample_rate as f32,
        0.0,
        sample_rate as f32 * 0.5,
        &mut bank,
    )
    .map_err(|_| StftBakeError::InvalidFrameCount)?;

    let header = AudioSpectralSidecarHeader {
        magic: SPECTRAL_SIDECAR_MAGIC,
        version: AudioSpectralSidecarHeader::VERSION_V2,
        _pad: SIDECAR_KIND_STFT,
        bin_count: bin_count as u32,
        frame_count: frame_count as u32,
        sample_rate,
    };

    let header_bytes = std::mem::size_of::<AudioSpectralSidecarHeader>();
    out[..header_bytes].copy_from_slice(bytemuck::bytes_of(&header));

    // Plane-0: one-sided magnitude spectrum, frame-major.
    let mut off = header_bytes;
    for frame_mags in &mags {
        // Each frame carries exactly bin_count one-sided bins.
        if frame_mags.len() != bin_count {
            return Err(StftBakeError::InvalidFrameCount);
        }
        out[off..off + bin_count * 4].copy_from_slice(bytemuck::cast_slice(frame_mags));
        off += bin_count * 4;
    }

    // v2 sub-header.
    let sub = SpectralV2SubHeader {
        magic: SPECTRAL_V2_SUBHEADER_MAGIC,
        n_mel: n_mel as u32,
        n_mfcc: n_mfcc as u32,
    };
    out[off..off + std::mem::size_of::<SpectralV2SubHeader>()]
        .copy_from_slice(bytemuck::bytes_of(&sub));
    off += std::mem::size_of::<SpectralV2SubHeader>();

    // Mel plane, then MFCC plane. Power = |X|².
    let mel_base = off;
    let mfcc_base = mel_base + frame_count * n_mel * 4;
    let mut power = vec![0.0f32; bin_count];
    let mut mel_out = vec![0.0f32; n_mel];
    let mut mfcc_out = vec![0.0f32; n_mfcc];
    let mut scratch = vec![0.0f32; 2 * n_mel];
    for (f, frame_mags) in mags.iter().enumerate() {
        for (p, &m) in power.iter_mut().zip(frame_mags.iter()) {
            *p = m * m;
        }
        mel_bands(&power, &bank, n_mel, &mut mel_out)
            .map_err(|_| StftBakeError::InvalidFrameCount)?;
        mfcc(&power, &bank, n_mel, n_mfcc, &mut mfcc_out, &mut scratch)
            .map_err(|_| StftBakeError::InvalidFrameCount)?;

        let mel_off = mel_base + f * n_mel * 4;
        out[mel_off..mel_off + n_mel * 4].copy_from_slice(bytemuck::cast_slice(&mel_out));
        let mfcc_off = mfcc_base + f * n_mfcc * 4;
        out[mfcc_off..mfcc_off + n_mfcc * 4].copy_from_slice(bytemuck::cast_slice(&mfcc_out));
    }

    Ok(need)
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

    #[test]
    fn v2_subheader_size_is_12() {
        assert_eq!(std::mem::size_of::<SpectralV2SubHeader>(), 12);
    }

    #[test]
    fn v1_header_still_valid_after_widening() {
        // A pure v1 header (version == 1) must still parse after is_valid widened.
        let h = AudioSpectralSidecarHeader {
            magic: SPECTRAL_SIDECAR_MAGIC,
            version: AudioSpectralSidecarHeader::VERSION,
            _pad: SIDECAR_KIND_STFT,
            bin_count: 64,
            frame_count: 4,
            sample_rate: 48_000,
        };
        let mut buf = vec![0u8; std::mem::size_of::<AudioSpectralSidecarHeader>() + 64 * 4 * 4];
        buf[..std::mem::size_of::<AudioSpectralSidecarHeader>()]
            .copy_from_slice(bytemuck::bytes_of(&h));
        // Put a marker in frame 2 so the frame view is observably correct.
        let po = std::mem::size_of::<AudioSpectralSidecarHeader>();
        let f2 = po + 2 * 64 * 4;
        buf[f2..f2 + 4].copy_from_slice(&7.5f32.to_le_bytes());
        let parsed = parse_sidecar_header(&buf).expect("v1 still valid");
        assert_eq!(parsed.version, 1);
        let frame = sidecar_frame_view(&buf, 2).expect("frame 2");
        assert_eq!(frame.len(), 64);
        assert_eq!(frame[0], 7.5);
        // A v1 file has no v2 planes.
        assert!(parse_v2_subheader(&buf).is_none());
        assert!(sidecar_mel_frame_view(&buf, 0).is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn v2_bake_round_trips_and_tone_energy_is_concentrated() {
        use core::f32::consts::TAU;
        const FRAME: usize = 256;
        const HOP: usize = 128;
        const N_MEL: usize = 26;
        const N_MFCC: usize = 13;
        let sr = 16_000u32;
        // A single tone at ~2 kHz over several frames.
        let freq = 2000.0f32;
        let samples: Vec<f32> = (0..FRAME * 6)
            .map(|i| (TAU * freq * i as f32 / sr as f32).sin())
            .collect();

        let bin_count = FRAME / 2 + 1;
        // Frame count = (len - FRAME)/HOP + 1.
        let frame_count = (samples.len() - FRAME) / HOP + 1;
        let mut buf = vec![0u8; v2_sidecar_size(frame_count, bin_count, N_MEL, N_MFCC)];
        let n = bake_spectral_v2_from_samples(&samples, FRAME, HOP, sr, N_MEL, N_MFCC, &mut buf)
            .expect("v2 bake");
        assert_eq!(n, buf.len());

        let header = parse_sidecar_header(&buf).expect("v2 header parses");
        assert_eq!(header.version, AudioSpectralSidecarHeader::VERSION_V2);
        assert_eq!(header.bin_count as usize, bin_count);
        assert_eq!(header.frame_count as usize, frame_count);

        let sub = parse_v2_subheader(&buf).expect("v2 subheader parses");
        assert_eq!(sub.n_mel as usize, N_MEL);
        assert_eq!(sub.n_mfcc as usize, N_MFCC);

        // Frame views return the right widths.
        let mel = sidecar_mel_frame_view(&buf, 1).expect("mel frame 1");
        assert_eq!(mel.len(), N_MEL);
        let mfcc_v = sidecar_mfcc_frame_view(&buf, 1).expect("mfcc frame 1");
        assert_eq!(mfcc_v.len(), N_MFCC);
        // Out-of-range frame → None.
        assert!(sidecar_mel_frame_view(&buf, header.frame_count).is_none());

        // Known tone: mel energy concentrated — the peak mel band carries a large
        // share of the total, and its index maps to a sane (mid-band) region.
        let total: f32 = mel.iter().sum();
        assert!(total > 0.0, "mel energy present for a tone");
        let (peak_idx, &peak_val) = mel
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        assert!(
            peak_val > 0.25 * total,
            "tone not concentrated: peak {peak_val} vs total {total}"
        );
        // ~2 kHz on a 0..8 kHz mel axis sits in the interior, not at an edge.
        assert!(
            peak_idx > 0 && peak_idx < N_MEL - 1,
            "peak mel band {peak_idx} at edge"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn v1_reader_reads_plane0_of_v2_file() {
        use core::f32::consts::TAU;
        const FRAME: usize = 256;
        const HOP: usize = 128;
        let sr = 16_000u32;
        let samples: Vec<f32> = (0..FRAME * 4)
            .map(|i| (TAU * 1000.0 * i as f32 / sr as f32).sin())
            .collect();
        let bin_count = FRAME / 2 + 1;
        let frame_count = (samples.len() - FRAME) / HOP + 1;
        let mut buf = vec![0u8; v2_sidecar_size(frame_count, bin_count, 20, 10)];
        bake_spectral_v2_from_samples(&samples, FRAME, HOP, sr, 20, 10, &mut buf).expect("bake");

        // A v1-style reader (parse_sidecar_header + sidecar_frame_view, bounded by
        // payload_bytes) reads plane-0 correctly and never crosses into v2 planes.
        let header = parse_sidecar_header(&buf).expect("header");
        assert_eq!(header.bin_count as usize, bin_count);
        let frame0 = sidecar_frame_view(&buf, 0).expect("plane-0 frame 0");
        assert_eq!(frame0.len(), bin_count);
        assert!(
            frame0.iter().any(|&v| v > 0.0),
            "plane-0 has real STFT energy"
        );
        // The last plane-0 frame is still within payload_bytes().
        let last = sidecar_frame_view(&buf, header.frame_count - 1).expect("last plane-0 frame");
        assert_eq!(last.len(), bin_count);
    }
}
