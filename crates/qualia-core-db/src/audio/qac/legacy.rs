//! Whole-object admission of legacy versions 1, 2 and 3; no allocation.
use super::samples::F32LeSlice;
use crate::audio::audio_spectral_sheet_v3::SofaHrirEntry;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QacReadError {
    Limit,
    Truncated,
    Magic,
    Version,
    Geometry,
    Transform,
    Crc,
    Metadata,
    Extent,
    Nonfinite,
    Padding,
    Trailing,
}
/// Budgets checked before payload scanning.
#[derive(Clone, Copy, Debug)]
pub struct QacReadLimits {
    pub max_bytes: u64,
    pub max_frames: u32,
    pub max_bins: u32,
}
impl Default for QacReadLimits {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            max_frames: 16_777_216,
            max_bins: 1_048_576,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegacyQacInfo {
    pub version: u16,
    pub transform_kind: u16,
    pub bins: u32,
    pub frames: u32,
    pub sample_rate: u32,
    pub channels: u32,
    pub mel_count: u32,
    pub mfcc_count: u32,
    pub hrir_directions: u32,
}
/// Complete finite legacy object. V3 checks header CRC only: legacy formats have
/// no payload digest. This view does not recover missing phase/PCM or transform
/// parameters. Arbitrary nonfinite originals need separate opaque preservation.
#[derive(Clone, Copy, Debug)]
pub struct LegacyQacView<'a> {
    bytes: &'a [u8],
    info: LegacyQacInfo,
    raster: &'a [u8],
    mel: &'a [u8],
    mfcc: &'a [u8],
    hrir: &'a [u8],
}
fn span(bytes: &[u8], start: usize, len: usize) -> Result<&[u8], QacReadError> {
    bytes
        .get(start..start.checked_add(len).ok_or(QacReadError::Extent)?)
        .ok_or(QacReadError::Truncated)
}
fn u16_at(bytes: &[u8], at: usize) -> Result<u16, QacReadError> {
    let b = span(bytes, at, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}
fn u32_at(bytes: &[u8], at: usize) -> Result<u32, QacReadError> {
    let b = span(bytes, at, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn f32_at(bytes: &[u8], at: usize) -> Result<f32, QacReadError> {
    Ok(f32::from_bits(u32_at(bytes, at)?))
}
fn scalar_bytes(a: u32, b: u32, c: u32) -> Result<usize, QacReadError> {
    usize::try_from(a)
        .ok()
        .and_then(|n| n.checked_mul(usize::try_from(b).ok()?))
        .and_then(|n| n.checked_mul(usize::try_from(c).ok()?))
        .and_then(|n| n.checked_mul(4))
        .ok_or(QacReadError::Extent)
}
fn take<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], QacReadError> {
    let out = span(bytes, *cursor, len)?;
    *cursor = cursor.checked_add(len).ok_or(QacReadError::Extent)?;
    Ok(out)
}
fn finite(bytes: &[u8]) -> Result<(), QacReadError> {
    if bytes.len() % 4 != 0 {
        return Err(QacReadError::Extent);
    }
    if F32LeSlice(bytes).iter().all(f32::is_finite) {
        Ok(())
    } else {
        Err(QacReadError::Nonfinite)
    }
}
impl<'a> LegacyQacView<'a> {
    pub fn parse(bytes: &'a [u8], limits: QacReadLimits) -> Result<Self, QacReadError> {
        if u64::try_from(bytes.len()).map_err(|_| QacReadError::Limit)? > limits.max_bytes {
            return Err(QacReadError::Limit);
        }
        if span(bytes, 0, 4)? != [0x55, 0x41, 0x34, 0x51] {
            return Err(QacReadError::Magic);
        }
        let version = u16_at(bytes, 4)?;
        if !(1..=3).contains(&version) {
            return Err(QacReadError::Version);
        }
        let kind = u16_at(bytes, 6)?;
        if kind > if version == 3 { 2 } else { 1 } {
            return Err(QacReadError::Transform);
        }
        let bins = u32_at(bytes, 8)?;
        let frames = u32_at(bytes, 12)?;
        let rate = u32_at(bytes, 16)?;
        if bins == 0 || frames == 0 || rate == 0 {
            return Err(QacReadError::Geometry);
        }
        if bins > limits.max_bins || frames > limits.max_frames {
            return Err(QacReadError::Limit);
        }
        let mut channels = 1;
        let mut mel_count = 0;
        let mut mfcc_count = 0;
        let mut directions = 0;
        let mut hrir_offset = 0;
        let mut cursor = 20;
        if version == 3 {
            let header = span(bytes, 0, 64)?;
            if crate::container_10d::crc32c::crc32c(&header[..60]) != u32_at(header, 60)? {
                return Err(QacReadError::Crc);
            }
            if header[20] > 3 || header[21] > 1 {
                return Err(QacReadError::Metadata);
            }
            channels = (u32::from(header[20]) + 1).pow(2);
            for at in [36, 40, 44] {
                let value = f32_at(header, at)?;
                if !value.is_finite() || value < 0.0 {
                    return Err(QacReadError::Metadata);
                }
            }
            let gain = f32_at(header, 56)?;
            if !gain.is_finite() || !(0.0..=1.0).contains(&gain) {
                return Err(QacReadError::Metadata);
            }
            mel_count = u32::from(u16_at(header, 32)?);
            mfcc_count = u32::from(u16_at(header, 34)?);
            directions = u32_at(header, 24)?;
            hrir_offset = usize::try_from(u32_at(header, 28)?).map_err(|_| QacReadError::Extent)?;
            let taps = u16_at(header, 22)?;
            if (directions == 0 && (taps != 0 || hrir_offset != 0))
                || (directions != 0 && taps != 64)
            {
                return Err(QacReadError::Metadata);
            }
            cursor = 64;
        }
        let raster = take(bytes, &mut cursor, scalar_bytes(bins, frames, channels)?)?;
        if version == 2 {
            let sub = take(bytes, &mut cursor, 12)?;
            if &sub[..4] != b"Q4M2" {
                return Err(QacReadError::Magic);
            }
            mel_count = u32_at(sub, 4)?;
            mfcc_count = u32_at(sub, 8)?;
            if mel_count == 0 || mfcc_count == 0 {
                return Err(QacReadError::Geometry);
            }
        }
        let mel = take(bytes, &mut cursor, scalar_bytes(mel_count, frames, 1)?)?;
        let mfcc = take(bytes, &mut cursor, scalar_bytes(mfcc_count, frames, 1)?)?;
        let hrir = if directions == 0 {
            &bytes[0..0]
        } else {
            let gap = hrir_offset
                .checked_sub(cursor)
                .ok_or(QacReadError::Extent)?;
            if take(bytes, &mut cursor, gap)?.iter().any(|b| *b != 0) {
                return Err(QacReadError::Padding);
            }
            let len = usize::try_from(directions)
                .ok()
                .and_then(|n| n.checked_mul(524))
                .ok_or(QacReadError::Extent)?;
            take(bytes, &mut cursor, len)?
        };
        if cursor != bytes.len() {
            return Err(QacReadError::Trailing);
        }
        for plane in [raster, mel, mfcc, hrir] {
            finite(plane)?;
        }
        Ok(Self {
            bytes,
            info: LegacyQacInfo {
                version,
                transform_kind: kind,
                bins,
                frames,
                sample_rate: rate,
                channels,
                mel_count,
                mfcc_count,
                hrir_directions: directions,
            },
            raster,
            mel,
            mfcc,
            hrir,
        })
    }
    pub fn info(self) -> LegacyQacInfo {
        self.info
    }
    pub fn as_bytes(self) -> &'a [u8] {
        self.bytes
    }
    fn frame(self, plane: &'a [u8], index: u32, width: usize) -> Option<F32LeSlice<'a>> {
        if index >= self.info.frames || width == 0 {
            return None;
        }
        let len = width.checked_mul(4)?;
        let start = usize::try_from(index).ok()?.checked_mul(len)?;
        Some(F32LeSlice(plane.get(start..start.checked_add(len)?)?))
    }
    pub fn spectral_frame(self, index: u32) -> Option<F32LeSlice<'a>> {
        let width = usize::try_from(self.info.bins)
            .ok()?
            .checked_mul(usize::try_from(self.info.channels).ok()?)?;
        self.frame(self.raster, index, width)
    }
    pub fn mel_frame(self, index: u32) -> Option<F32LeSlice<'a>> {
        self.frame(self.mel, index, usize::try_from(self.info.mel_count).ok()?)
    }
    pub fn mfcc_frame(self, index: u32) -> Option<F32LeSlice<'a>> {
        self.frame(
            self.mfcc,
            index,
            usize::try_from(self.info.mfcc_count).ok()?,
        )
    }
    /// Decode one fixed-64-tap-per-ear HRIR onto the stack, without alignment assumptions.
    pub fn hrir(self, index: u32) -> Option<SofaHrirEntry> {
        let start = usize::try_from(index).ok()?.checked_mul(524)?;
        let values = F32LeSlice(self.hrir.get(start..start.checked_add(524)?)?);
        let mut out = SofaHrirEntry {
            azimuth_deg: values.get(0)?,
            elevation_deg: values.get(1)?,
            itd_samples: values.get(2)?,
            left_fir: [0.0; 64],
            right_fir: [0.0; 64],
        };
        for i in 0..64 {
            out.left_fir[i] = values.get(3 + i)?;
            out.right_fir[i] = values.get(67 + i)?;
        }
        Some(out)
    }
}
