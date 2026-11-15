use crate::medical::dicom_dataset::{
    DatasetError, ElementKind, Part10Dataset, TransferSyntax, WorkBudget,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelError {
    Dataset(DatasetError),
    Missing(u32),
    Unsupported(&'static str),
    Invalid(u32),
    Bounds,
    Overflow,
    ByteLimit,
}
impl From<DatasetError> for PixelError {
    fn from(e: DatasetError) -> Self {
        Self::Dataset(e)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoredSample {
    Unsigned(u32),
    Signed(i32),
}
impl StoredSample {
    pub fn as_f64(self) -> f64 {
        match self {
            Self::Unsigned(n) => n as f64,
            Self::Signed(n) => n as f64,
        }
    }
}
/// Borrowed native values with sign extension from BitsStored. Unused bits/source
/// bytes remain intact. MONOCHROME1 is not inverted; no display LUT is applied.
pub struct NativePixelView<'a> {
    data: &'a [u8],
    source_blake3: [u8; 32],
    rows: u16,
    columns: u16,
    frames: u32,
    bits_allocated: u16,
    bits_stored: u16,
    signed: bool,
    photometric: &'a str,
    samples_per_frame: usize,
    padding_declared: bool,
}
impl<'a> NativePixelView<'a> {
    pub fn from_dataset(
        ds: &Part10Dataset<'a>,
        max_pixel_bytes: usize,
        w: &mut WorkBudget,
    ) -> Result<Self, PixelError> {
        if ds.transfer_syntax() == TransferSyntax::EncapsulatedExplicitLittleEndian {
            return Err(PixelError::Unsupported(
                "encapsulated bytes require a qualified codec adapter",
            ));
        }
        let info = super::metadata::IntegerImageInfo::from_dataset(ds, max_pixel_bytes, w)?;
        let need = info.bytes;
        let id = ds
            .find(None, 0x7fe00010, w)?
            .ok_or(PixelError::Missing(0x7fe00010))?;
        let e = ds.element(id)?;
        if e.kind != ElementKind::Value || e.vr.is_some_and(|vr| !matches!(&vr, b"OB" | b"OW")) {
            return Err(PixelError::Invalid(0x7fe00010));
        }
        let all = ds.value(id)?;
        let padded = need.checked_add(need % 2).ok_or(PixelError::Overflow)?;
        if all.len() != padded
            || all
                .get(need..)
                .is_some_and(|padding| padding.iter().any(|b| *b != 0))
        {
            return Err(PixelError::Invalid(0x7fe00010));
        }
        Self::from_admitted_layout(info, &all[..need])
    }
    pub(super) fn from_admitted_layout(
        info: super::metadata::IntegerImageInfo<'a>,
        data: &'a [u8],
    ) -> Result<Self, PixelError> {
        if data.len() != info.bytes {
            return Err(PixelError::Invalid(0x7fe00010));
        }
        Ok(Self {
            data,
            source_blake3: info.source_blake3,
            rows: info.rows,
            columns: info.columns,
            frames: info.frames,
            bits_allocated: info.bits_allocated,
            bits_stored: info.bits_stored,
            signed: info.signed,
            photometric: info.photometric,
            samples_per_frame: info.samples_per_frame,
            padding_declared: info.padding_declared,
        })
    }
    pub fn source_blake3(&self) -> [u8; 32] {
        self.source_blake3
    }
    pub fn rows(&self) -> u16 {
        self.rows
    }
    pub fn columns(&self) -> u16 {
        self.columns
    }
    pub fn frames(&self) -> u32 {
        self.frames
    }
    pub fn photometric(&self) -> &'a str {
        self.photometric
    }
    /// Raw stored-sample access does not classify these values as measurements.
    pub fn padding_declared(&self) -> bool {
        self.padding_declared
    }
    /// Stored sample layout: borrowed native bytes or losslessly decoded bytes.
    /// The encoded original always remains in Part10Dataset::original_bytes.
    pub fn stored_sample_bytes(&self) -> &'a [u8] {
        self.data
    }
    pub fn sample(
        &self,
        frame_index: u32,
        row: u16,
        column: u16,
    ) -> Result<StoredSample, PixelError> {
        if frame_index >= self.frames || row >= self.rows || column >= self.columns {
            return Err(PixelError::Bounds);
        }
        let row_offset = (row as usize)
            .checked_mul(self.columns as usize)
            .ok_or(PixelError::Overflow)?;
        let index = (frame_index as usize)
            .checked_mul(self.samples_per_frame)
            .and_then(|v| v.checked_add(row_offset))
            .and_then(|v| v.checked_add(column as usize))
            .ok_or(PixelError::Overflow)?;
        let width = self.bits_allocated as usize / 8;
        let start = index.checked_mul(width).ok_or(PixelError::Overflow)?;
        let b = self
            .data
            .get(start..start.checked_add(width).ok_or(PixelError::Overflow)?)
            .ok_or(PixelError::Bounds)?;
        let word = match width {
            1 => b[0] as u32,
            2 => u16::from_le_bytes([b[0], b[1]]) as u32,
            4 => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            _ => return Err(PixelError::Invalid(0x00280100)),
        };
        let shift = 32 - self.bits_stored;
        let masked = if self.bits_stored == 32 {
            word
        } else {
            word & ((1u32 << self.bits_stored) - 1)
        };
        Ok(if self.signed {
            StoredSample::Signed(((masked << shift) as i32) >> shift)
        } else {
            StoredSample::Unsigned(masked)
        })
    }
}
