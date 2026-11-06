//! `.q4au` v3 64-byte Spatial Acoustics container header, Higher-Order Ambisonics (HOA),
//! SOFA HRTF FIR table block, and v1/v2 -> v3 migration helper.

use bytemuck::{Pod, Zeroable};

use super::audio_spectral_sheet::{
    parse_sidecar_header, parse_v2_subheader, AudioSpectralSidecarHeader,
    SPECTRAL_SIDECAR_MAGIC,
};

/// Format version for Q4AU-v3 spatial acoustic sheets.
pub const Q4AU_VERSION_V3: u16 = 3;

/// Exact byte size of the v3 cache-line aligned header.
pub const Q4AU_V3_HEADER_BYTES: usize = 64;

/// Maximum supported ambisonic order (3rd order).
pub const Q4AU_HOA_MAX_ORDER: u8 = 3;

/// Maximum channels for 3rd order ambisonics: (3 + 1)^2 = 16.
pub const Q4AU_HOA_MAX_CHANNELS: usize = 16;

/// Transform kind tags stored in `Q4AuHeaderV3.transform_kind`.
pub const TRANSFORM_KIND_STFT: u16 = 0;
pub const TRANSFORM_KIND_CQT: u16 = 1;
pub const TRANSFORM_KIND_HOA_STFT: u16 = 2;

/// Channel normalization conventions stored in `Q4AuHeaderV3.channel_normalization`.
pub const NORMALIZATION_SN3D_ACN: u8 = 0;
pub const NORMALIZATION_N3D_ACN: u8 = 1;

/// Default number of FIR filter taps per ear for embedded SOFA HRIR entries.
pub const SOFA_DEFAULT_FIR_TAPS: usize = 64;

/// 64-byte cache-line-aligned spatial acoustic container header.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Q4AuHeaderV3 {
    /// Format magic "Q4AU" (`SPECTRAL_SIDECAR_MAGIC` = 0x5134_4155).
    pub magic: u32,
    /// Format version (`Q4AU_VERSION_V3` = 3).
    pub version: u16,
    /// Transform kind (0 = STFT, 1 = CQT, 2 = HOA_STFT).
    pub transform_kind: u16,
    /// Frequency bins per frame.
    pub bin_count: u32,
    /// Number of frames in the raster.
    pub frame_count: u32,
    /// Source sample rate (e.g. 48_000).
    pub sample_rate: u32,
    /// Ambisonic order L in 0..=3 ((L+1)^2 channels).
    pub ambisonic_order: u8,
    /// Channel normalization (0 = SN3D_ACN, 1 = N3D_ACN).
    pub channel_normalization: u8,
    /// Number of FIR taps per ear in SOFA table (e.g. 64).
    pub sofa_fir_taps: u16,
    /// Number of discrete directions in the SOFA table block.
    pub sofa_direction_count: u32,
    /// Absolute byte offset from start of file to SOFA HRIR table block (0 if none).
    pub sofa_block_offset: u32,
    /// Number of mel frequency bands (0 if none).
    pub mel_count: u16,
    /// Number of MFCC coefficients (0 if none).
    pub mfcc_count: u16,
    /// RT60 reverberation decay times in seconds for low / mid / high octave bands.
    pub rt60_bands_sec: [f32; 3],
    /// DID hash of linked acoustic portal boundary for diffraction/transmission.
    pub portal_coupling_did_hash: u64,
    /// Portal transmission scalar gain (0.0 .. 1.0).
    pub portal_transmission_gain: f32,
    /// Whole-header CRC-32C computed over the first 60 bytes.
    pub header_crc32c: u32,
}

impl Q4AuHeaderV3 {
    /// Empty header initialized with magic, version 3, and default 48kHz.
    #[inline]
    pub const fn empty() -> Self {
        Self {
            magic: SPECTRAL_SIDECAR_MAGIC,
            version: Q4AU_VERSION_V3,
            transform_kind: TRANSFORM_KIND_STFT,
            bin_count: 0,
            frame_count: 0,
            sample_rate: 48_000,
            ambisonic_order: 0,
            channel_normalization: NORMALIZATION_SN3D_ACN,
            sofa_fir_taps: 0,
            sofa_direction_count: 0,
            sofa_block_offset: 0,
            mel_count: 0,
            mfcc_count: 0,
            rt60_bands_sec: [0.0; 3],
            portal_coupling_did_hash: 0,
            portal_transmission_gain: 1.0,
            header_crc32c: 0,
        }
    }

    /// Compute CRC-32C over the first 60 bytes of this header.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes = bytemuck::bytes_of(self);
        crate::container_10d::crc32c::crc32c(&bytes[..60])
    }

    /// Update `header_crc32c` with the computed CRC-32C of this header.
    #[inline]
    pub fn update_crc(&mut self) {
        self.header_crc32c = self.compute_crc32c();
    }

    /// True when magic, version, order, geometry and CRC-32C are valid.
    #[inline]
    pub fn validate(&self) -> bool {
        self.magic == SPECTRAL_SIDECAR_MAGIC
            && self.version == Q4AU_VERSION_V3
            && self.ambisonic_order <= Q4AU_HOA_MAX_ORDER
            && self.bin_count > 0
            && self.frame_count > 0
            && self.header_crc32c == self.compute_crc32c()
    }

    /// Channel count corresponding to `ambisonic_order`: `(order + 1)^2`.
    #[inline]
    pub fn hoa_channel_count(&self) -> usize {
        ((self.ambisonic_order as usize) + 1).pow(2)
    }

    /// Byte size of the HOA spectral raster payload.
    #[inline]
    pub fn payload_bytes(&self) -> usize {
        (self.bin_count as usize)
            .saturating_mul(self.frame_count as usize)
            .saturating_mul(self.hoa_channel_count())
            .saturating_mul(4)
    }

    /// Byte size of the embedded SOFA HRIR table.
    #[inline]
    pub fn sofa_bytes(&self) -> usize {
        (self.sofa_direction_count as usize).saturating_mul(std::mem::size_of::<SofaHrirEntry>())
    }
}

/// Compact SOFA HRIR measurement entry for one spherical direction.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SofaHrirEntry {
    /// Azimuth angle in degrees (-180.0 ..= 180.0 or 0.0 ..= 360.0).
    pub azimuth_deg: f32,
    /// Elevation angle in degrees (-90.0 ..= 90.0).
    pub elevation_deg: f32,
    /// Interaural Time Difference (ITD) in fractional samples.
    pub itd_samples: f32,
    /// Left ear finite impulse response taps.
    pub left_fir: [f32; SOFA_DEFAULT_FIR_TAPS],
    /// Right ear finite impulse response taps.
    pub right_fir: [f32; SOFA_DEFAULT_FIR_TAPS],
}

/// Zero-heap ACN / SN3D real spherical harmonics encoder for orders 0..=3.
///
/// Computes basis gains into caller-supplied `out_gains` (up to 16 channels) and returns
/// the number of channels written `((order + 1)^2)`. Remaining channels are zeroed.
#[inline]
pub fn encode_hoa_acn_sn3d(
    order: u8,
    azimuth_rad: f32,
    elevation_rad: f32,
    out_gains: &mut [f32; Q4AU_HOA_MAX_CHANNELS],
) -> usize {
    let cos_el = elevation_rad.cos();
    let sin_el = elevation_rad.sin();
    let cos_az = azimuth_rad.cos();
    let sin_az = azimuth_rad.sin();

    let cos_2az = (2.0 * azimuth_rad).cos();
    let sin_2az = (2.0 * azimuth_rad).sin();
    let cos_3az = (3.0 * azimuth_rad).cos();
    let sin_3az = (3.0 * azimuth_rad).sin();

    let cos_el2 = cos_el * cos_el;
    let cos_el3 = cos_el2 * cos_el;
    let sin_el2 = sin_el * sin_el;

    // Order 0 (ACN 0: W)
    out_gains[0] = 1.0;

    // Order 1 (ACN 1..=3: Y, Z, X)
    out_gains[1] = sin_az * cos_el;
    out_gains[2] = sin_el;
    out_gains[3] = cos_az * cos_el;

    // Order 2 (ACN 4..=8: V, T, R, S, U)
    const SQRT_3_OVER_2: f32 = 0.866_025_4;
    const SQRT_3: f32 = 1.732_050_8;
    out_gains[4] = SQRT_3_OVER_2 * sin_2az * cos_el2;
    out_gains[5] = SQRT_3 * sin_az * sin_el * cos_el;
    out_gains[6] = 0.5 * (3.0 * sin_el2 - 1.0);
    out_gains[7] = SQRT_3 * cos_az * sin_el * cos_el;
    out_gains[8] = SQRT_3_OVER_2 * cos_2az * cos_el2;

    // Order 3 (ACN 9..=15: Q, O, M, K, L, N, P)
    const SQRT_5_OVER_8: f32 = 0.790_569_4;
    const SQRT_15_OVER_2: f32 = 1.936_491_7;
    const SQRT_3_OVER_8: f32 = 0.612_372_4;
    out_gains[9] = SQRT_5_OVER_8 * sin_3az * cos_el3;
    out_gains[10] = SQRT_15_OVER_2 * sin_2az * sin_el * cos_el2;
    out_gains[11] = SQRT_3_OVER_8 * sin_az * cos_el * (5.0 * sin_el2 - 1.0);
    out_gains[12] = 0.5 * sin_el * (5.0 * sin_el2 - 3.0);
    out_gains[13] = SQRT_3_OVER_8 * cos_az * cos_el * (5.0 * sin_el2 - 1.0);
    out_gains[14] = SQRT_15_OVER_2 * cos_2az * sin_el * cos_el2;
    out_gains[15] = SQRT_5_OVER_8 * cos_3az * cos_el3;

    let effective_order = order.min(Q4AU_HOA_MAX_ORDER);
    let count = ((effective_order as usize) + 1).pow(2);
    for slot in out_gains.iter_mut().skip(count) {
        *slot = 0.0;
    }
    count
}

/// Nearest-direction lookup in a discrete SOFA HRIR table using unit-vector dot product.
#[inline]
pub fn sample_sofa_hrir_zero_heap<'a>(
    table: &'a [SofaHrirEntry],
    azimuth_deg: f32,
    elevation_deg: f32,
) -> Option<&'a SofaHrirEntry> {
    if table.is_empty() {
        return None;
    }
    let az_rad = azimuth_deg.to_radians();
    let el_rad = elevation_deg.to_radians();
    let tx = el_rad.cos() * az_rad.cos();
    let ty = el_rad.cos() * az_rad.sin();
    let tz = el_rad.sin();

    let mut best_entry = &table[0];
    let mut max_dot = -f32::INFINITY;

    for entry in table {
        let e_az = entry.azimuth_deg.to_radians();
        let e_el = entry.elevation_deg.to_radians();
        let ex = e_el.cos() * e_az.cos();
        let ey = e_el.cos() * e_az.sin();
        let ez = e_el.sin();
        let dot = tx * ex + ty * ey + tz * ez;
        if dot > max_dot {
            max_dot = dot;
            best_entry = entry;
        }
    }
    Some(best_entry)
}

/// Parse v3 header from raw slice.
#[inline]
pub fn parse_v3_header(bytes: &[u8]) -> Option<Q4AuHeaderV3> {
    if bytes.len() < Q4AU_V3_HEADER_BYTES {
        return None;
    }
    let header = bytemuck::pod_read_unaligned::<Q4AuHeaderV3>(&bytes[..Q4AU_V3_HEADER_BYTES]);
    header.validate().then_some(header)
}

/// Zero-heap frame slice view for v3 sidecars.
#[inline]
pub fn v3_frame_view<'a>(bytes: &'a [u8], frame_index: u32) -> Option<&'a [f32]> {
    let header = parse_v3_header(bytes)?;
    if frame_index >= header.frame_count {
        return None;
    }
    let channels = header.hoa_channel_count();
    let frame_floats = (header.bin_count as usize).saturating_mul(channels);
    let offset_bytes = Q4AU_V3_HEADER_BYTES
        + (frame_index as usize).saturating_mul(frame_floats).saturating_mul(4);
    let len_bytes = frame_floats.saturating_mul(4);
    if bytes.len() < offset_bytes.saturating_add(len_bytes) {
        return None;
    }
    bytemuck::try_cast_slice(&bytes[offset_bytes..offset_bytes + len_bytes]).ok()
}

/// Borrow the embedded SOFA HRIR table slice if present in the v3 container.
#[inline]
pub fn v3_sofa_table<'a>(bytes: &'a [u8]) -> Option<&'a [SofaHrirEntry]> {
    let header = parse_v3_header(bytes)?;
    if header.sofa_direction_count == 0 || (header.sofa_block_offset as usize) < Q4AU_V3_HEADER_BYTES {
        return None;
    }
    let start = header.sofa_block_offset as usize;
    let end = start.checked_add(header.sofa_bytes())?;
    if bytes.len() < end {
        return None;
    }
    bytemuck::try_cast_slice(&bytes[start..end]).ok()
}

/// Migrate existing Q4AU-v1 or Q4AU-v2 audio sidecar into Q4AU-v3 format.
pub fn upgrade_q4au_v1_or_v2_to_v3(
    legacy_bytes: &[u8],
    ambisonic_order: u8,
    rt60_bands_sec: [f32; 3],
) -> Option<(Q4AuHeaderV3, Vec<u8>)> {
    let legacy_header = parse_sidecar_header(legacy_bytes)?;
    let plane0_bytes = legacy_header.payload_bytes();
    let header_size = std::mem::size_of::<AudioSpectralSidecarHeader>();
    if legacy_bytes.len() < header_size.checked_add(plane0_bytes)? {
        return None;
    }
    let plane0 = &legacy_bytes[header_size..header_size + plane0_bytes];

    let mut mel_count = 0u16;
    let mut mfcc_count = 0u16;
    let mut payload = Vec::with_capacity(plane0_bytes);
    payload.extend_from_slice(plane0);

    if legacy_header.version == AudioSpectralSidecarHeader::VERSION_V2 {
        if let Some(v2_sub) = parse_v2_subheader(legacy_bytes) {
            mel_count = v2_sub.n_mel as u16;
            mfcc_count = v2_sub.n_mfcc as u16;
            let sub_size = std::mem::size_of::<super::audio_spectral_sheet::SpectralV2SubHeader>();
            let sub_off = header_size + plane0_bytes;
            let mel_bytes = (legacy_header.frame_count as usize)
                .saturating_mul(v2_sub.n_mel as usize)
                .saturating_mul(4);
            let mfcc_bytes = (legacy_header.frame_count as usize)
                .saturating_mul(v2_sub.n_mfcc as usize)
                .saturating_mul(4);
            let mel_start = sub_off + sub_size;
            let mfcc_start = mel_start + mel_bytes;
            let total_v2_len = mfcc_start + mfcc_bytes;
            if legacy_bytes.len() >= total_v2_len {
                payload.reserve(mel_bytes + mfcc_bytes);
                payload.extend_from_slice(&legacy_bytes[mel_start..mel_start + mel_bytes]);
                payload.extend_from_slice(&legacy_bytes[mfcc_start..mfcc_start + mfcc_bytes]);
            }
        }
    }

    let mut v3_header = Q4AuHeaderV3 {
        magic: SPECTRAL_SIDECAR_MAGIC,
        version: Q4AU_VERSION_V3,
        transform_kind: legacy_header._pad,
        bin_count: legacy_header.bin_count,
        frame_count: legacy_header.frame_count,
        sample_rate: legacy_header.sample_rate,
        ambisonic_order: ambisonic_order.min(Q4AU_HOA_MAX_ORDER),
        channel_normalization: NORMALIZATION_SN3D_ACN,
        sofa_fir_taps: 0,
        sofa_direction_count: 0,
        sofa_block_offset: 0,
        mel_count,
        mfcc_count,
        rt60_bands_sec,
        portal_coupling_did_hash: 0,
        portal_transmission_gain: 1.0,
        header_crc32c: 0,
    };
    v3_header.update_crc();

    Some((v3_header, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v3_header_size_and_alignment_are_exact() {
        assert_eq!(std::mem::size_of::<Q4AuHeaderV3>(), 64);
        assert_eq!(std::mem::align_of::<Q4AuHeaderV3>(), 64);
    }

    #[test]
    fn v3_header_crc_and_validation() {
        let mut hdr = Q4AuHeaderV3::empty();
        hdr.bin_count = 64;
        hdr.frame_count = 100;
        hdr.update_crc();
        assert!(hdr.validate());
        assert_eq!(hdr.hoa_channel_count(), 1);

        hdr.bin_count = 0;
        assert!(!hdr.validate());
    }

    #[test]
    fn hoa_acn_sn3d_encoding() {
        let mut gains = [0.0f32; Q4AU_HOA_MAX_CHANNELS];
        let channels = encode_hoa_acn_sn3d(3, 0.0, 0.0, &mut gains);
        assert_eq!(channels, 16);
        assert_eq!(gains[0], 1.0); // W
        assert_eq!(gains[1], 0.0); // Y
        assert_eq!(gains[2], 0.0); // Z
        assert_eq!(gains[3], 1.0); // X

        // Polar elevation = 90 deg -> zonal harmonics evaluate to 1.0
        let _ = encode_hoa_acn_sn3d(3, 0.0, std::f32::consts::FRAC_PI_2, &mut gains);
        assert_eq!(gains[0], 1.0); // l=0, m=0
        assert_eq!(gains[2], 1.0); // l=1, m=0
        assert_eq!(gains[6], 1.0); // l=2, m=0
        assert_eq!(gains[12], 1.0); // l=3, m=0
    }

    #[test]
    fn sofa_hrir_nearest_direction_sampling() {
        let mut front = SofaHrirEntry {
            azimuth_deg: 0.0,
            elevation_deg: 0.0,
            itd_samples: 0.0,
            left_fir: [0.0; SOFA_DEFAULT_FIR_TAPS],
            right_fir: [0.0; SOFA_DEFAULT_FIR_TAPS],
        };
        front.left_fir[0] = 1.0;

        let mut left = SofaHrirEntry {
            azimuth_deg: 90.0,
            elevation_deg: 0.0,
            itd_samples: 12.0,
            left_fir: [0.0; SOFA_DEFAULT_FIR_TAPS],
            right_fir: [0.0; SOFA_DEFAULT_FIR_TAPS],
        };
        left.left_fir[0] = 2.0;

        let table = [front, left];
        let sample1 = sample_sofa_hrir_zero_heap(&table, 5.0, 0.0).unwrap();
        assert_eq!(sample1.azimuth_deg, 0.0);
        assert_eq!(sample1.left_fir[0], 1.0);

        let sample2 = sample_sofa_hrir_zero_heap(&table, 85.0, 0.0).unwrap();
        assert_eq!(sample2.azimuth_deg, 90.0);
        assert_eq!(sample2.left_fir[0], 2.0);
    }

    #[test]
    fn upgrade_v1_to_v3() {
        let mut legacy = Vec::new();
        let hdr = AudioSpectralSidecarHeader {
            magic: SPECTRAL_SIDECAR_MAGIC,
            version: AudioSpectralSidecarHeader::VERSION,
            _pad: TRANSFORM_KIND_STFT,
            bin_count: 8,
            frame_count: 4,
            sample_rate: 44_100,
        };
        legacy.extend_from_slice(bytemuck::bytes_of(&hdr));
        legacy.resize(legacy.len() + 8 * 4 * 4, 0x11);

        let (v3_hdr, payload) =
            upgrade_q4au_v1_or_v2_to_v3(&legacy, 1, [0.5, 0.4, 0.3]).expect("valid upgrade");
        assert!(v3_hdr.validate());
        assert_eq!(v3_hdr.version, Q4AU_VERSION_V3);
        assert_eq!(v3_hdr.ambisonic_order, 1);
        assert_eq!(v3_hdr.hoa_channel_count(), 4);
        assert_eq!(v3_hdr.sample_rate, 44_100);
        assert_eq!(payload.len(), 8 * 4 * 4);
    }
}
