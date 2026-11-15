use super::{map_sample, PaddingError, QuantitativeSample, SampleValidity};
use crate::medical::{
    dicom_codecs::{metadata::IntegerImageInfo, NativePixelView, StoredSample},
    dicom_dataset::{Part10Dataset, WorkBudget},
    dicom_frames::FrameRecord,
};
#[derive(Clone, Copy, Debug)]
pub struct IntegerPadding {
    source: [u8; 32],
    endpoints: Option<[i64; 2]>,
}
impl IntegerPadding {
    pub fn source_blake3(&self) -> [u8; 32] {
        self.source
    }
    pub fn endpoints(&self) -> Option<[i64; 2]> {
        self.endpoints
    }
    pub fn is_padding(&self, value: StoredSample) -> bool {
        let value = match value {
            StoredSample::Unsigned(n) => n as i64,
            StoredSample::Signed(n) => n as i64,
        };
        self.endpoints
            .is_some_and(|r| value >= r[0].min(r[1]) && value <= r[0].max(r[1]))
    }
    fn from_dataset(
        ds: &Part10Dataset<'_>,
        max_bytes: usize,
        w: &mut WorkBudget,
    ) -> Result<Self, PaddingError> {
        let info = IntegerImageInfo::from_dataset(ds, max_bytes, w)?;
        for tag in [0x00280122, 0x00280123, 0x00280124, 0x00280125] {
            if ds.find(None, tag, w)?.is_some() {
                return Err(PaddingError::Invalid(tag));
            }
        }
        let read = |tag, w: &mut WorkBudget| -> Result<Option<i64>, PaddingError> {
            ds.expected_value(None, tag, if info.signed { *b"SS" } else { *b"US" }, w)?
                .map(|b| {
                    let a: [u8; 2] = b.try_into().map_err(|_| PaddingError::Invalid(tag))?;
                    Ok(if info.signed {
                        i16::from_le_bytes(a) as i64
                    } else {
                        u16::from_le_bytes(a) as i64
                    })
                })
                .transpose()
        };
        let a = read(0x00280120, w)?;
        let b = read(0x00280121, w)?;
        let endpoints = match (a, b) {
            (None, None) => None,
            (Some(a), None) => Some([a, a]),
            (Some(a), Some(b)) => Some([a, b]),
            (None, Some(_)) => return Err(PaddingError::Missing(0x00280120)),
        };
        if let Some([a, b]) = endpoints {
            let (low, high) = if info.signed {
                let n = 1i64
                    .checked_shl((info.bits_stored - 1) as u32)
                    .ok_or(PaddingError::Invalid(0x00280101))?;
                (-n, n - 1)
            } else {
                (
                    0,
                    1i64.checked_shl(info.bits_stored as u32)
                        .ok_or(PaddingError::Invalid(0x00280101))?
                        - 1,
                )
            };
            if a < low || a > high || b < low || b > high {
                return Err(PaddingError::Invalid(0x00280120));
            }
            if (info.photometric == "MONOCHROME2" && a > b)
                || (info.photometric == "MONOCHROME1" && a < b)
            {
                return Err(PaddingError::Invalid(0x00280121));
            }
        }
        Ok(Self {
            source: ds.source_blake3(),
            endpoints,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegerSample {
    pub stored: StoredSample,
    pub validity: SampleValidity,
}
/// Source-matched integer/native-or-RLE view with stored-domain padding checked
/// BEFORE modality/real-world scaling. Existing stable codecs are unchanged.
pub struct IntegerPaddingView<'v, 'a> {
    pixels: &'v NativePixelView<'a>,
    padding: IntegerPadding,
}
impl<'v, 'a> IntegerPaddingView<'v, 'a> {
    pub fn bind(
        ds: &Part10Dataset<'_>,
        pixels: &'v NativePixelView<'a>,
        w: &mut WorkBudget,
    ) -> Result<Self, PaddingError> {
        if ds.source_blake3() != pixels.source_blake3() {
            return Err(PaddingError::SourceMismatch);
        }
        let padding = IntegerPadding::from_dataset(ds, pixels.stored_sample_bytes().len(), w)?;
        Ok(Self { pixels, padding })
    }
    pub fn padding(&self) -> IntegerPadding {
        self.padding
    }
    pub fn sample(&self, frame: u32, row: u16, column: u16) -> Result<IntegerSample, PaddingError> {
        let stored = self.pixels.sample(frame, row, column)?;
        let validity = if self.padding.is_padding(stored) {
            SampleValidity::Padding
        } else {
            SampleValidity::Valid
        };
        Ok(IntegerSample { stored, validity })
    }
    pub fn quantitative_sample<'b>(
        &self,
        record: &FrameRecord<'b>,
        row: u16,
        column: u16,
        real_world: bool,
    ) -> Result<QuantitativeSample<'b>, PaddingError> {
        if record.identity().source_blake3 != self.padding.source {
            return Err(PaddingError::SourceMismatch);
        }
        let frame = record
            .frame_number()
            .checked_sub(1)
            .ok_or(PaddingError::FrameMismatch)?;
        let sample = self.sample(frame, row, column)?;
        map_sample(
            record,
            self.padding.source,
            frame,
            Some(sample.stored.as_f64()),
            sample.validity,
            real_world,
        )
    }
}
