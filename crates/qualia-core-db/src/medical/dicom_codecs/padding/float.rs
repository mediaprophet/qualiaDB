use super::{PaddingError, PaddingMatch};
use crate::medical::{
    dicom_codecs::float_pixels::{FloatBits, FloatWidth},
    dicom_dataset::{Part10Dataset, WorkBudget},
};
#[derive(Clone, Copy, Debug)]
pub struct FloatPadding {
    source: [u8; 32],
    width: FloatWidth,
    endpoints: Option<[FloatBits; 2]>,
}
impl FloatPadding {
    pub fn from_dataset(
        ds: &Part10Dataset<'_>,
        width: FloatWidth,
        w: &mut WorkBudget,
    ) -> Result<Self, PaddingError> {
        if ds.find(None, width.pixel_tag(), w)?.is_none() {
            return Err(PaddingError::Missing(width.pixel_tag()));
        }
        for tag in [
            0x7fe00010,
            if width == FloatWidth::F32 {
                0x7fe00009
            } else {
                0x7fe00008
            },
        ] {
            if ds.find(None, tag, w)?.is_some() {
                return Err(PaddingError::Invalid(tag));
            }
        }
        let (value, limit, vr, other) = match width {
            FloatWidth::F32 => (0x00280122, 0x00280124, *b"FL", [0x00280123, 0x00280125]),
            FloatWidth::F64 => (0x00280123, 0x00280125, *b"FD", [0x00280122, 0x00280124]),
        };
        for tag in [0x00280120, 0x00280121, other[0], other[1]] {
            if ds.find(None, tag, w)?.is_some() {
                return Err(PaddingError::Invalid(tag));
            }
        }
        let read = |tag, w: &mut WorkBudget| -> Result<Option<FloatBits>, PaddingError> {
            ds.expected_value(None, tag, vr, w)?
                .map(|b| match width {
                    FloatWidth::F32 => Ok(FloatBits::F32(u32::from_le_bytes(
                        b.try_into().map_err(|_| PaddingError::Invalid(tag))?,
                    ))),
                    FloatWidth::F64 => Ok(FloatBits::F64(u64::from_le_bytes(
                        b.try_into().map_err(|_| PaddingError::Invalid(tag))?,
                    ))),
                })
                .transpose()
        };
        let a = read(value, w)?;
        let b = read(limit, w)?;
        let endpoints = match (a, b) {
            (None, None) => None,
            (Some(a), Some(b)) => Some([a, b]),
            (None, Some(_)) => return Err(PaddingError::Missing(value)),
            (Some(_), None) => return Err(PaddingError::Missing(limit)),
        };
        Ok(Self {
            source: ds.source_blake3(),
            width,
            endpoints,
        })
    }
    pub fn source_blake3(&self) -> [u8; 32] {
        self.source
    }
    pub fn endpoints(&self) -> Option<[FloatBits; 2]> {
        self.endpoints
    }
    /// Ordered endpoints use inclusive numeric comparison (including infinities
    /// and numerically equal signed zeros). NaN endpoints remain Unordered:
    /// DICOM permits these bits but does not define NaN range ordering/payload
    /// equality. No finite measurement is guessed from that declaration.
    pub fn classify(&self, value: FloatBits) -> Result<PaddingMatch, PaddingError> {
        if value.width() != self.width {
            return Err(PaddingError::WidthMismatch);
        }
        let Some([a, b]) = self.endpoints else {
            return Ok(PaddingMatch::NotPadding);
        };
        let (Some(a), Some(b)) = (a.ordered_f64(), b.ordered_f64()) else {
            return Ok(PaddingMatch::Unordered);
        };
        let Some(value) = value.ordered_f64() else {
            return Ok(PaddingMatch::NotPadding);
        };
        let (low, high) = if a <= b { (a, b) } else { (b, a) };
        Ok(if value >= low && value <= high {
            PaddingMatch::Padding
        } else {
            PaddingMatch::NotPadding
        })
    }
}
