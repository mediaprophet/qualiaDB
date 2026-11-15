use super::{FloatBits, FloatWidth};
use crate::medical::{
    dicom_codecs::padding::{
        self, FloatPadding, PaddingError, PaddingMatch, QuantitativeSample, SampleValidity,
    },
    dicom_dataset::{DatasetError, ElementKind, Part10Dataset, TransferSyntax, WorkBudget},
    dicom_frames::FrameRecord,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatSample {
    pub bits: FloatBits,
    pub validity: SampleValidity,
}
/// Borrowed OF/OD sample bits, never an aligned float cast or image normalization.
/// Only designated DICOM Float/DoubleFloat Pixel Data tags enter this image view.
pub struct FloatPixelView<'a> {
    data: &'a [u8],
    source: [u8; 32],
    width: FloatWidth,
    rows: u16,
    columns: u16,
    frames: u32,
    padding: FloatPadding,
}
impl<'a> FloatPixelView<'a> {
    pub fn from_dataset(
        ds: &Part10Dataset<'a>,
        max_pixel_bytes: usize,
        w: &mut WorkBudget,
    ) -> Result<Self, PaddingError> {
        if ds.transfer_syntax() == TransferSyntax::EncapsulatedExplicitLittleEndian {
            return Err(PaddingError::Unsupported(
                "float OF/OD requires a selected native little-endian transfer syntax",
            ));
        }
        let mut selected = None;
        for (tag, width) in [(0x7fe00008, FloatWidth::F32), (0x7fe00009, FloatWidth::F64)] {
            if let Some(id) = ds.find(None, tag, w)? {
                if selected.is_some() {
                    return Err(PaddingError::Invalid(tag));
                }
                selected = Some((id, width));
            }
        }
        let (id, width) = selected.ok_or(PaddingError::Missing(0x7fe00008))?;
        for tag in [
            0x7fe00010, 0x00280101, 0x00280102, 0x00280103, 0x00280006, 0x00287fe0,
        ] {
            if ds.find(None, tag, w)?.is_some() {
                return Err(PaddingError::Invalid(tag));
            }
        }
        let required =
            |tag, w: &mut WorkBudget| ds.unsigned_short(tag, w)?.ok_or(PaddingError::Missing(tag));
        let rows = required(0x00280010, w)?;
        let columns = required(0x00280011, w)?;
        if rows == 0 || columns == 0 {
            return Err(PaddingError::Invalid(0x00280010));
        }
        if required(0x00280002, w)? != 1 {
            return Err(PaddingError::Invalid(0x00280002));
        }
        if ds.text(None, 0x00280004, *b"CS", w)? != Some("MONOCHROME2") {
            return Err(PaddingError::Invalid(0x00280004));
        }
        if required(0x00280100, w)? as usize != width.bytes() * 8 {
            return Err(PaddingError::Invalid(0x00280100));
        }
        let frames = ds.frame_count(w)?;
        let need = (rows as usize)
            .checked_mul(columns as usize)
            .and_then(|n| n.checked_mul(frames as usize))
            .and_then(|n| n.checked_mul(width.bytes()))
            .ok_or(DatasetError::Overflow)?;
        if need > max_pixel_bytes {
            return Err(PaddingError::Pixel(
                crate::medical::dicom_codecs::PixelError::ByteLimit,
            ));
        }
        let element = ds.element(id)?;
        let vr = if width == FloatWidth::F32 {
            *b"OF"
        } else {
            *b"OD"
        };
        if element.kind != ElementKind::Value || element.vr.is_some_and(|v| v != vr) {
            return Err(PaddingError::Invalid(width.pixel_tag()));
        }
        let data = ds.value(id)?;
        if data.len() != need {
            return Err(PaddingError::Invalid(width.pixel_tag()));
        }
        let padding = FloatPadding::from_dataset(ds, width, w)?;
        Ok(Self {
            data,
            source: ds.source_blake3(),
            width,
            rows,
            columns,
            frames,
            padding,
        })
    }
    pub fn source_blake3(&self) -> [u8; 32] {
        self.source
    }
    pub fn width(&self) -> FloatWidth {
        self.width
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
    pub fn stored_sample_bytes(&self) -> &'a [u8] {
        self.data
    }
    pub fn padding(&self) -> FloatPadding {
        self.padding
    }
    pub fn sample_bits(
        &self,
        frame: u32,
        row: u16,
        column: u16,
    ) -> Result<FloatBits, PaddingError> {
        if frame >= self.frames || row >= self.rows || column >= self.columns {
            return Err(PaddingError::Pixel(
                crate::medical::dicom_codecs::PixelError::Bounds,
            ));
        }
        let row_index = (frame as usize)
            .checked_mul(self.rows as usize)
            .and_then(|n| n.checked_add(row as usize))
            .ok_or(DatasetError::Overflow)?;
        let pixel = row_index
            .checked_mul(self.columns as usize)
            .and_then(|n| n.checked_add(column as usize))
            .ok_or(DatasetError::Overflow)?;
        let start = pixel
            .checked_mul(self.width.bytes())
            .ok_or(DatasetError::Overflow)?;
        let end = start
            .checked_add(self.width.bytes())
            .ok_or(DatasetError::Overflow)?;
        let b = self
            .data
            .get(start..end)
            .ok_or(PaddingError::Invalid(self.width.pixel_tag()))?;
        Ok(match self.width {
            FloatWidth::F32 => FloatBits::F32(u32::from_le_bytes(
                b.try_into()
                    .map_err(|_| PaddingError::Invalid(self.width.pixel_tag()))?,
            )),
            FloatWidth::F64 => FloatBits::F64(u64::from_le_bytes(
                b.try_into()
                    .map_err(|_| PaddingError::Invalid(self.width.pixel_tag()))?,
            )),
        })
    }
    pub fn sample(&self, frame: u32, row: u16, column: u16) -> Result<FloatSample, PaddingError> {
        let bits = self.sample_bits(frame, row, column)?;
        let validity = match self.padding.classify(bits)? {
            PaddingMatch::Padding => SampleValidity::Padding,
            PaddingMatch::Unordered => SampleValidity::PaddingIndeterminate,
            PaddingMatch::NotPadding => {
                if bits.is_finite() {
                    SampleValidity::Valid
                } else {
                    SampleValidity::NonFinite
                }
            }
        };
        Ok(FloatSample { bits, validity })
    }
    pub fn quantitative_sample<'b>(
        &self,
        record: &FrameRecord<'b>,
        row: u16,
        column: u16,
        real_world: bool,
    ) -> Result<QuantitativeSample<'b>, PaddingError> {
        if record.identity().source_blake3 != self.source {
            return Err(PaddingError::SourceMismatch);
        }
        let frame = record
            .frame_number()
            .checked_sub(1)
            .ok_or(PaddingError::FrameMismatch)?;
        let sample = self.sample(frame, row, column)?;
        padding::map_sample(
            record,
            self.source,
            frame,
            sample.bits.finite_f64(),
            sample.validity,
            real_world,
        )
    }
}
