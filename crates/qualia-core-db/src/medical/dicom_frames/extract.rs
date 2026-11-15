use super::quantitative::{self, QuantitativeStatus};
use crate::dicom::DicomMetadata;
use crate::medical::{
    dicom_dataset::{DatasetError, ElementKind, ItemId, Part10Dataset, WorkBudget},
    dicom_geometry::{GeometryError, PatientImagePlane},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    Dataset(DatasetError),
    FrameLimit,
    MissingFrames,
    SharedPerFrameConflict(u32),
    TopLevelConflict(u32),
    Geometry(GeometryError),
    IdentityMismatch,
    NonImage,
    Pixel(crate::medical::dicom_codecs::PixelError),
    PaddingInterpretationUnavailable,
}
impl From<DatasetError> for FrameError {
    fn from(e: DatasetError) -> Self {
        Self::Dataset(e)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct FrameLimits {
    pub max_frames: usize,
    pub max_work: usize,
    pub max_frame_bytes: usize,
}
impl Default for FrameLimits {
    fn default() -> Self {
        Self {
            max_frames: 4096,
            max_work: 16 * 1024 * 1024,
            max_frame_bytes: 4 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DatasetIdentity<'a> {
    pub source_blake3: [u8; 32],
    pub study_instance_uid: Option<&'a str>,
    pub series_instance_uid: Option<&'a str>,
    pub sop_class_uid: Option<&'a str>,
    pub sop_instance_uid: Option<&'a str>,
    pub frame_of_reference_uid: Option<&'a str>,
}
pub struct FrameRecord<'a> {
    frame_number: u32,
    identity: DatasetIdentity<'a>,
    metadata: DicomMetadata,
    quantitative: QuantitativeStatus<'a>,
}
impl<'a> FrameRecord<'a> {
    pub fn frame_number(&self) -> u32 {
        self.frame_number
    }
    pub fn identity(&self) -> DatasetIdentity<'a> {
        self.identity
    }
    pub fn metadata(&self) -> &DicomMetadata {
        &self.metadata
    }
    pub fn quantitative(&self) -> QuantitativeStatus<'a> {
        self.quantitative
    }
    /// Apply only the explicitly selected mapping to the matching source/frame.
    /// Pixel-padding semantics are not silently treated as measured intensities.
    pub fn quantitative_sample(
        &self,
        pixels: &crate::medical::dicom_codecs::NativePixelView<'_>,
        row: u16,
        column: u16,
        real_world: bool,
    ) -> Result<Option<f64>, FrameError> {
        if pixels.source_blake3() != self.identity.source_blake3 {
            return Err(FrameError::IdentityMismatch);
        }
        if pixels.padding_declared() {
            return Err(FrameError::PaddingInterpretationUnavailable);
        }
        let mapping = if real_world {
            self.quantitative.real_world
        } else {
            self.quantitative.modality
        };
        let Some(mapping) = mapping else {
            return Ok(None);
        };
        if mapping.source_blake3() != self.identity.source_blake3
            || mapping.frame_number() != self.frame_number
        {
            return Err(FrameError::IdentityMismatch);
        }
        let stored = pixels
            .sample(
                self.frame_number
                    .checked_sub(1)
                    .ok_or(FrameError::IdentityMismatch)?,
                row,
                column,
            )
            .map_err(FrameError::Pixel)?;
        Ok(Some(mapping.apply(stored.as_f64())?))
    }
    pub fn plane(&self) -> Result<PatientImagePlane<'_>, GeometryError> {
        PatientImagePlane::from_metadata(&self.metadata)
    }
}
pub struct FrameSet<'a> {
    pub identity: DatasetIdentity<'a>,
    pub frames: Vec<FrameRecord<'a>>,
    pub enhanced: bool,
}
pub(super) fn single_item(
    ds: &Part10Dataset<'_>,
    scope: Option<ItemId>,
    tag: u32,
    w: &mut WorkBudget,
) -> Result<Option<ItemId>, FrameError> {
    let Some(id) = ds.find(scope, tag, w)? else {
        return Ok(None);
    };
    if ds.element(id)?.kind != ElementKind::Sequence {
        return Err(DatasetError::Invalid(tag).into());
    }
    let mut items = ds.sequence_items(id, w)?;
    let first = items.next().ok_or(DatasetError::Invalid(tag))?;
    if items.next().is_some() {
        return Err(DatasetError::Invalid(tag).into());
    }
    Ok(Some(first))
}
pub(super) fn macro_item(
    ds: &Part10Dataset<'_>,
    shared: Option<ItemId>,
    per: Option<ItemId>,
    tag: u32,
    w: &mut WorkBudget,
) -> Result<Option<ItemId>, FrameError> {
    let a = if let Some(i) = shared {
        single_item(ds, Some(i), tag, w)?
    } else {
        None
    };
    let b = if let Some(i) = per {
        single_item(ds, Some(i), tag, w)?
    } else {
        None
    };
    if a.is_some() && b.is_some() {
        return Err(FrameError::SharedPerFrameConflict(tag));
    }
    Ok(a.or(b))
}
fn geometry(
    ds: &Part10Dataset<'_>,
    meta: &mut DicomMetadata,
    scope: Option<ItemId>,
    tag: u32,
    w: &mut WorkBudget,
) -> Result<(), FrameError> {
    if let Some(id) = ds.find(scope, tag, w)? {
        let e = ds.element(id)?;
        if e.kind != ElementKind::Value {
            return Err(DatasetError::Invalid(tag).into());
        }
        crate::medical::dicom_geometry::apply_geometry_value(meta, tag, ds.value(id)?, e.vr);
    }
    Ok(())
}
impl<'a> FrameSet<'a> {
    pub fn extract(ds: &Part10Dataset<'a>, limits: FrameLimits) -> Result<Self, FrameError> {
        let mut w = WorkBudget::new(limits.max_work);
        let identity = DatasetIdentity {
            source_blake3: ds.source_blake3(),
            study_instance_uid: ds.uid(None, 0x0020000d, &mut w)?,
            series_instance_uid: ds.uid(None, 0x0020000e, &mut w)?,
            sop_class_uid: ds.uid(None, 0x00080016, &mut w)?,
            sop_instance_uid: ds.uid(None, 0x00080018, &mut w)?,
            frame_of_reference_uid: ds.uid(None, 0x00200052, &mut w)?,
        };
        if ds.media_sop_class_uid().is_some() && ds.media_sop_class_uid() != identity.sop_class_uid
            || ds.media_sop_instance_uid().is_some()
                && ds.media_sop_instance_uid() != identity.sop_instance_uid
        {
            return Err(FrameError::IdentityMismatch);
        }
        let shared = single_item(ds, None, 0x52009229, &mut w)?;
        let per_seq = ds.find(None, 0x52009230, &mut w)?;
        let enhanced = shared.is_some() || per_seq.is_some();
        let mut pixels = 0;
        for tag in [0x7fe00010, 0x7fe00008, 0x7fe00009] {
            pixels += usize::from(ds.find(None, tag, &mut w)?.is_some());
        }
        if pixels == 0 {
            return Ok(Self {
                identity,
                frames: Vec::new(),
                enhanced,
            });
        }
        if pixels > 1 {
            return Err(DatasetError::Invalid(0x7fe00010).into());
        }
        let count = usize::try_from(ds.frame_count(&mut w)?).map_err(|_| FrameError::FrameLimit)?;
        let frame_bytes = count
            .checked_mul(
                std::mem::size_of::<FrameRecord<'a>>() + std::mem::size_of::<ItemId>() + 128,
            )
            .ok_or(FrameError::FrameLimit)?;
        if frame_bytes > limits.max_frame_bytes {
            return Err(FrameError::FrameLimit);
        }
        if count > limits.max_frames {
            return Err(FrameError::FrameLimit);
        }
        if enhanced && ds.find(None, 0x00280008, &mut w)?.is_none() {
            return Err(FrameError::MissingFrames);
        }
        let mut per_items = Vec::new();
        per_items
            .try_reserve_exact(count)
            .map_err(|_| FrameError::FrameLimit)?;
        if let Some(id) = per_seq {
            if ds.element(id)?.kind != ElementKind::Sequence {
                return Err(DatasetError::Invalid(0x52009230).into());
            }
            for item in ds.sequence_items(id, &mut w)? {
                if per_items.len() >= count {
                    return Err(FrameError::MissingFrames);
                }
                per_items.push(item);
            }
            if per_items.len() != count {
                return Err(FrameError::MissingFrames);
            }
        }
        let mut frames = Vec::new();
        frames
            .try_reserve_exact(count)
            .map_err(|_| FrameError::FrameLimit)?;
        for frame in 0..count {
            let per = per_items.get(frame).copied();
            let mut metadata = DicomMetadata::default();
            for tag in [0x00102210, 0x00200052] {
                geometry(ds, &mut metadata, None, tag, &mut w)?;
            }
            if enhanced {
                for (macro_tag, value_tag) in [
                    (0x00209113, 0x00200032),
                    (0x00209116, 0x00200037),
                    (0x00289110, 0x00280030),
                ] {
                    if ds.find(None, value_tag, &mut w)?.is_some() {
                        return Err(FrameError::TopLevelConflict(value_tag));
                    }
                    if let Some(item) = macro_item(ds, shared, per, macro_tag, &mut w)? {
                        geometry(ds, &mut metadata, Some(item), value_tag, &mut w)?;
                    }
                }
            } else if count == 1 {
                for tag in [0x00200032, 0x00200037, 0x00280030] {
                    geometry(ds, &mut metadata, None, tag, &mut w)?;
                }
            }
            let quantitative = quantitative::extract(
                ds,
                shared,
                per,
                enhanced,
                u32::try_from(frame + 1).map_err(|_| FrameError::FrameLimit)?,
                &mut w,
            )?;
            frames.push(FrameRecord {
                frame_number: u32::try_from(frame + 1).map_err(|_| FrameError::FrameLimit)?,
                identity,
                metadata,
                quantitative,
            });
        }
        Ok(Self {
            identity,
            frames,
            enhanced,
        })
    }
}
