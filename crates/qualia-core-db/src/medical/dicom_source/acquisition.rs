//! Retain once, then inspect the same immutable bytes with explicit admission.
use super::storage::{preserve, read_source};
use super::{DicomSourceArtifact, DicomSourceBudget, DicomSourceError};
use crate::medical::{
    dicom_dataset::{DatasetError, ParseLimits, Part10Dataset},
    dicom_frames::{FrameError, FrameLimits, FrameSet},
};
use std::path::Path;

/// Cold-path acquisition owner. The source and byte identity cannot be replaced
/// through this API after publication. Unsupported interpretation does not discard
/// the original. This is not an IOD-conformance or clinical-validity assertion.
pub struct RetainedDicom {
    source: DicomSourceArtifact,
    bytes: Vec<u8>,
}
impl RetainedDicom {
    /// Read one bounded snapshot, publish it without replacement and keep that
    /// exact snapshot for analysis. No second source-path read can change identity.
    pub fn retain(
        storage_root: &Path,
        source_path: &Path,
        budget: DicomSourceBudget,
    ) -> Result<Self, DicomSourceError> {
        let bytes = read_source(source_path, budget)?;
        if !crate::dicom::is_dicom_part10(&bytes) {
            return Err(DicomSourceError::NotPart10);
        }
        let source = preserve(storage_root, &bytes, budget)?;
        Ok(Self { source, bytes })
    }
    pub fn source(&self) -> &DicomSourceArtifact {
        &self.source
    }
    pub fn original_bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Interpret only under the supplied parser/frame budgets. Parse failure
    /// leaves the retained object usable for archival or another qualified adapter.
    /// Frame extraction is a separate result: geometry/mapping failure must not
    /// pretend the dataset or its original source disappeared.
    pub fn inspect(
        &self,
        parse_limits: ParseLimits,
        frame_limits: FrameLimits,
    ) -> Result<AcquisitionView<'_>, DatasetError> {
        let dataset = Part10Dataset::parse(&self.bytes, parse_limits)?;
        let frames = FrameSet::extract(&dataset, frame_limits);
        Ok(AcquisitionView { dataset, frames })
    }
}
/// Both views borrow the retained source snapshot, not a reconstructed pixel file.
pub struct AcquisitionView<'a> {
    dataset: Part10Dataset<'a>,
    frames: Result<FrameSet<'a>, FrameError>,
}
impl<'a> AcquisitionView<'a> {
    pub fn dataset(&self) -> &Part10Dataset<'a> {
        &self.dataset
    }
    pub fn frames(&self) -> Result<&FrameSet<'a>, &FrameError> {
        self.frames.as_ref()
    }
}
