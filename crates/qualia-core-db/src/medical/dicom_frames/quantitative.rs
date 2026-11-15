use super::extract::{macro_item, single_item, FrameError};
use crate::medical::dicom_dataset::{DatasetError, ItemId, Part10Dataset, WorkBudget};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueUnits<'a> {
    Unspecified,
    RescaleType(&'a str),
    Code {
        scheme: &'a str,
        value: &'a str,
        meaning: Option<&'a str>,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct LinearTransform<'a> {
    slope: f64,
    intercept: f64,
    units: ValueUnits<'a>,
    source_blake3: [u8; 32],
    frame_number: u32,
    stored_range: Option<[f64; 2]>,
}
impl<'a> LinearTransform<'a> {
    pub fn slope(&self) -> f64 {
        self.slope
    }
    pub fn intercept(&self) -> f64 {
        self.intercept
    }
    pub fn units(&self) -> ValueUnits<'a> {
        self.units
    }
    pub fn source_blake3(&self) -> [u8; 32] {
        self.source_blake3
    }
    pub fn frame_number(&self) -> u32 {
        self.frame_number
    }
    pub fn stored_range(&self) -> Option<[f64; 2]> {
        self.stored_range
    }
    pub fn apply(&self, stored: f64) -> Result<f64, DatasetError> {
        if !stored.is_finite()
            || self
                .stored_range
                .is_some_and(|r| stored < r[0] || stored > r[1])
        {
            return Err(DatasetError::Invalid(0x00409096));
        }
        let value = self.slope * stored + self.intercept;
        if !value.is_finite() {
            return Err(DatasetError::Invalid(0x00409096));
        }
        Ok(value)
    }
}
/// The modality and real-world mappings are independent alternatives, never composed.
#[derive(Debug, Clone, Copy, Default)]
pub struct QuantitativeStatus<'a> {
    pub modality: Option<LinearTransform<'a>>,
    pub real_world: Option<LinearTransform<'a>>,
    pub unsupported_real_world: Option<&'static str>,
}
fn required<'a>(v: Option<&'a str>, tag: u32) -> Result<&'a str, FrameError> {
    v.ok_or(DatasetError::Missing(tag).into())
}
fn ds_number(
    ds: &Part10Dataset<'_>,
    scope: Option<ItemId>,
    tag: u32,
    w: &mut WorkBudget,
) -> Result<Option<f64>, FrameError> {
    ds.text(scope, tag, *b"DS", w)?
        .map(|s| {
            if s.len() > 16
                || s.contains('\\')
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'))
            {
                return Err(DatasetError::Invalid(tag).into());
            }
            let n = s
                .parse::<f64>()
                .map_err(|_| FrameError::Dataset(DatasetError::Invalid(tag)))?;
            if !n.is_finite() {
                return Err(DatasetError::Invalid(tag).into());
            }
            Ok(n)
        })
        .transpose()
}
fn fd(
    ds: &Part10Dataset<'_>,
    item: ItemId,
    tag: u32,
    w: &mut WorkBudget,
) -> Result<Option<f64>, FrameError> {
    ds.expected_value(Some(item), tag, *b"FD", w)?
        .map(|b| {
            let arr: [u8; 8] = b.try_into().map_err(|_| DatasetError::Invalid(tag))?;
            let n = f64::from_le_bytes(arr);
            if !n.is_finite() {
                return Err(DatasetError::Invalid(tag).into());
            }
            Ok(n)
        })
        .transpose()
}
pub(super) fn extract<'a>(
    ds: &Part10Dataset<'a>,
    shared: Option<ItemId>,
    per: Option<ItemId>,
    enhanced: bool,
    frame_number: u32,
    w: &mut WorkBudget,
) -> Result<QuantitativeStatus<'a>, FrameError> {
    let mut result = QuantitativeStatus::default();
    let transform = macro_item(ds, shared, per, 0x00289145, w)?;
    if enhanced {
        for tag in [0x00281052, 0x00281053, 0x00281054] {
            if ds.find(None, tag, w)?.is_some() {
                return Err(FrameError::TopLevelConflict(tag));
            }
        }
    }
    let scope = if enhanced { transform } else { None };
    if !enhanced || transform.is_some() {
        let slope = ds_number(ds, scope, 0x00281053, w)?;
        let intercept = ds_number(ds, scope, 0x00281052, w)?;
        let units = ds.text(scope, 0x00281054, *b"LO", w)?;
        if slope.is_some() || intercept.is_some() || units.is_some() {
            let slope = slope.ok_or(DatasetError::Missing(0x00281053))?;
            if slope == 0.0 {
                return Err(DatasetError::Invalid(0x00281053).into());
            }
            let intercept = intercept.ok_or(DatasetError::Missing(0x00281052))?;
            let units = required(units, 0x00281054)?;
            result.modality = Some(LinearTransform {
                slope,
                intercept,
                units: if units == "US" {
                    ValueUnits::Unspecified
                } else {
                    ValueUnits::RescaleType(units)
                },
                source_blake3: ds.source_blake3(),
                frame_number,
                stored_range: None,
            });
        }
    }
    // RWVM permits multiple items; this initial view refuses to choose one implicitly.
    let top = ds.find(None, 0x00409096, w)?;
    if enhanced && top.is_some() {
        return Err(FrameError::TopLevelConflict(0x00409096));
    }
    let mut rw_scope = top;
    for candidate in [shared, per].into_iter().flatten() {
        if let Some(seq) = ds.find(Some(candidate), 0x00409096, w)? {
            if rw_scope.is_some() {
                return Err(FrameError::SharedPerFrameConflict(0x00409096));
            }
            rw_scope = Some(seq);
        }
    }
    if let Some(seq) = rw_scope {
        let mut items = ds.sequence_items(seq, w)?;
        let first = items.next().ok_or(DatasetError::Invalid(0x00409096))?;
        if items.next().is_some() {
            result.unsupported_real_world =
                Some("multiple RWVM alternatives require explicit selection");
            return Ok(result);
        }
        if ds.find(Some(first), 0x00409212, w)?.is_some() {
            result.unsupported_real_world = Some("nonlinear real-world LUT mapping not selected");
            return Ok(result);
        }
        let slope = fd(ds, first, 0x00409225, w)?.ok_or(DatasetError::Missing(0x00409225))?;
        let intercept = fd(ds, first, 0x00409224, w)?.ok_or(DatasetError::Missing(0x00409224))?;
        let fa = fd(ds, first, 0x00409214, w)?;
        let fb = fd(ds, first, 0x00409213, w)?;
        let range = match (fa, fb) {
            (Some(a), Some(b)) => {
                if ds.find(Some(first), 0x00409216, w)?.is_some()
                    || ds.find(Some(first), 0x00409211, w)?.is_some()
                {
                    return Err(DatasetError::Invalid(0x00409214).into());
                }
                [a, b]
            }
            (None, None) => {
                let signed = ds
                    .unsigned_short(0x00280103, w)?
                    .ok_or(DatasetError::Missing(0x00280103))?;
                if signed > 1 {
                    return Err(DatasetError::Invalid(0x00280103).into());
                }
                let mut range = [0.0; 2];
                for (slot, tag) in [(0, 0x00409216), (1, 0x00409211)] {
                    let b = ds
                        .expected_value(
                            Some(first),
                            tag,
                            if signed == 1 { *b"SS" } else { *b"US" },
                            w,
                        )?
                        .ok_or(DatasetError::Missing(tag))?;
                    let a: [u8; 2] = b.try_into().map_err(|_| DatasetError::Invalid(tag))?;
                    range[slot] = if signed == 1 {
                        i16::from_le_bytes(a) as f64
                    } else {
                        u16::from_le_bytes(a) as f64
                    };
                }
                range
            }
            _ => return Err(DatasetError::Invalid(0x00409214).into()),
        };
        if range[0] > range[1] {
            return Err(DatasetError::Invalid(0x00409216).into());
        }
        let unit = single_item(ds, Some(first), 0x004008ea, w)?
            .ok_or(DatasetError::Missing(0x004008ea))?;
        let scheme = required(ds.text(Some(unit), 0x00080102, *b"SH", w)?, 0x00080102)?;
        let value = required(ds.text(Some(unit), 0x00080100, *b"SH", w)?, 0x00080100)?;
        let meaning = ds.text(Some(unit), 0x00080104, *b"LO", w)?;
        result.real_world = Some(LinearTransform {
            slope,
            intercept,
            units: ValueUnits::Code {
                scheme,
                value,
                meaning,
            },
            source_blake3: ds.source_blake3(),
            frame_number,
            stored_range: Some(range),
        });
    }
    Ok(result)
}
