use super::{PaddingError, SampleValidity};
use crate::medical::dicom_frames::{FrameRecord, ValueUnits};
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuantitativeSample<'a> {
    NoMapping,
    MappingUnavailable(&'static str),
    Padding,
    NonFinite,
    PaddingIndeterminate,
    Value {
        value: f64,
        units: ValueUnits<'a>,
        source_blake3: [u8; 32],
        frame_number: u32,
    },
}
pub(crate) fn map_sample<'a>(
    record: &FrameRecord<'a>,
    source: [u8; 32],
    frame_index: u32,
    stored: Option<f64>,
    validity: SampleValidity,
    real_world: bool,
) -> Result<QuantitativeSample<'a>, PaddingError> {
    if record.identity().source_blake3 != source {
        return Err(PaddingError::SourceMismatch);
    }
    if frame_index.checked_add(1) != Some(record.frame_number()) {
        return Err(PaddingError::FrameMismatch);
    }
    match validity {
        SampleValidity::Padding => return Ok(QuantitativeSample::Padding),
        SampleValidity::NonFinite => return Ok(QuantitativeSample::NonFinite),
        SampleValidity::PaddingIndeterminate => {
            return Ok(QuantitativeSample::PaddingIndeterminate)
        }
        SampleValidity::Valid => {}
    }
    let Some(stored) = stored else {
        return Ok(QuantitativeSample::NonFinite);
    };
    if !stored.is_finite() {
        return Ok(QuantitativeSample::NonFinite);
    }
    let status = record.quantitative();
    if real_world {
        if let Some(reason) = status.unsupported_real_world {
            return Ok(QuantitativeSample::MappingUnavailable(reason));
        }
    }
    let mapping = if real_world {
        status.real_world
    } else {
        status.modality
    };
    let Some(mapping) = mapping else {
        return Ok(QuantitativeSample::NoMapping);
    };
    if mapping.source_blake3() != source {
        return Err(PaddingError::SourceMismatch);
    }
    if mapping.frame_number() != record.frame_number() {
        return Err(PaddingError::FrameMismatch);
    }
    let value = mapping.apply(stored)?;
    Ok(QuantitativeSample::Value {
        value,
        units: mapping.units(),
        source_blake3: source,
        frame_number: record.frame_number(),
    })
}
