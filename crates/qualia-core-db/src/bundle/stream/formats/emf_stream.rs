use super::{
    AuthenticatedRange, EmfDirectoryView, EmfV3DirectoryView, FormatError, FormatLimits,
    PayloadInterval,
};
use crate::q42::emf_container::{
    chunk_directory::{EmfChunkDirectoryEntry, CHUNK_DIRECTORY_ENTRY_BYTES},
    extension::EmfExtensionLimits,
    header::{
        EmfContainerHeader, EMF_CONTAINER_VERSION, EMF_EXTENDED_VERSION, EMF_LEGACY_VERSION,
    },
};
use crate::q42::emf_profiles::ProfileRegistry;
use crate::q42::field_envelope::{self, Family, Limits, StructuralView};
use crate::q42::field_semantics::DomainRegistry;
/// Streaming EMF admit. Envelope v4 via `StructuralView`. Brick v2/v3 via
/// split `AuthenticatedRange`s into `EmfDirectoryView` / `EmfV3DirectoryView`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmfProfile {
    EnvelopeV4,
    DirectoryV2,
    InterpretationV3,
    OpaqueArchival,
}
enum EmfInner<'a> {
    EnvelopeV4(StructuralView<'a>),
    DirectoryV2(EmfDirectoryView<'a>),
    InterpretationV3(EmfV3DirectoryView<'a>),
}
pub struct EmfStreamView<'a> {
    range: AuthenticatedRange<'a>,
    inner: EmfInner<'a>,
    profile: EmfProfile,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmfAdmitError {
    Format(FormatError),
    Family,
    Version,
    Extent,
    Crc,
    IndexDigest,
    PayloadDigest,
    Limit,
    Unsupported,
    Envelope(field_envelope::Error),
}
impl From<FormatError> for EmfAdmitError {
    fn from(e: FormatError) -> Self {
        match e {
            FormatError::Unsupported => Self::Unsupported,
            FormatError::Limit => Self::Limit,
            FormatError::Bounds => Self::Extent,
            other => Self::Format(other),
        }
    }
}
impl From<field_envelope::Error> for EmfAdmitError {
    fn from(e: field_envelope::Error) -> Self {
        match e {
            field_envelope::Error::Family => Self::Family,
            field_envelope::Error::Version => Self::Version,
            field_envelope::Error::Extent => Self::Extent,
            field_envelope::Error::Crc => Self::Crc,
            field_envelope::Error::IndexDigest => Self::IndexDigest,
            field_envelope::Error::PayloadDigest => Self::PayloadDigest,
            field_envelope::Error::Limit => Self::Limit,
            other => Self::Envelope(other),
        }
    }
}
const EMF_STREAM_SCRATCH_ROWS: usize = 4096;
fn unknown_critical(view: &StructuralView<'_>) -> bool {
    let registry = ProfileRegistry;
    for block in view.blocks() {
        for field in block.fields() {
            if field.flags() & 1 != 0 && !(1..=11).contains(&field.type_tag()) {
                return true;
            }
        }
        let known = (block.revision() == 1 && matches!(block.schema_type(), 1..=5))
            || registry.rules(block.schema_type(), block.revision()).is_some();
        if !known && (block.flags() & 1 != 0 || block.fields().any(|f| f.flags() & 1 != 0)) {
            return true;
        }
    }
    false
}
fn format_limits(limits: Limits) -> Result<FormatLimits, EmfAdmitError> {
    Ok(FormatLimits {
        max_entry_bytes: limits.file_bytes,
        max_rows: usize::try_from(limits.entries)
            .map_err(|_| EmfAdmitError::Limit)?
            .min(EMF_STREAM_SCRATCH_ROWS),
        max_metadata_bytes: usize::try_from(limits.metadata_bytes)
            .map_err(|_| EmfAdmitError::Limit)?,
        max_chunk_bytes: usize::try_from(limits.chunk_bytes).map_err(|_| EmfAdmitError::Limit)?,
    })
}
fn extension_limits(limits: Limits) -> Result<EmfExtensionLimits, EmfAdmitError> {
    let conservative = EmfExtensionLimits::conservative();
    Ok(EmfExtensionLimits {
        max_file_bytes: usize::try_from(limits.file_bytes).map_err(|_| EmfAdmitError::Limit)?,
        max_metadata_bytes: conservative.max_metadata_bytes,
        max_audio_bytes: conservative.max_audio_bytes,
        max_tracks: conservative.max_tracks,
    })
}
fn split_header_directory(
    range: AuthenticatedRange<'_>,
) -> Result<(AuthenticatedRange<'_>, AuthenticatedRange<'_>), EmfAdmitError> {
    let header = range.subrange(0, 64)?;
    let h: EmfContainerHeader = bytemuck::pod_read_unaligned(header.bytes());
    let count = usize::try_from(h.chunk_count).map_err(|_| EmfAdmitError::Limit)?;
    let bytes = count
        .checked_mul(CHUNK_DIRECTORY_ENTRY_BYTES)
        .ok_or(EmfAdmitError::Extent)?;
    let directory = range.subrange(h.directory_offset, bytes)?;
    Ok((header, directory))
}
fn split_v3_tail(range: AuthenticatedRange<'_>) -> Result<AuthenticatedRange<'_>, EmfAdmitError> {
    let bytes = range.bytes();
    let footer_at = bytes.len().checked_sub(64).ok_or(EmfAdmitError::Extent)?;
    let mut field_end_raw = [0u8; 8];
    field_end_raw.copy_from_slice(&bytes[footer_at + 8..footer_at + 16]);
    let field_end = u64::from_le_bytes(field_end_raw);
    let start = usize::try_from(field_end).map_err(|_| EmfAdmitError::Extent)?;
    let length = bytes.len().checked_sub(start).ok_or(EmfAdmitError::Extent)?;
    Ok(range.subrange(field_end, length)?)
}
fn chunk_payload<'a>(
    range: AuthenticatedRange<'a>,
    header: &EmfContainerHeader,
    index: usize,
) -> Result<AuthenticatedRange<'a>, EmfAdmitError> {
    let row_rel = u64::try_from(index)
        .ok()
        .and_then(|i| i.checked_mul(CHUNK_DIRECTORY_ENTRY_BYTES as u64))
        .ok_or(EmfAdmitError::Extent)?;
    let row_off = header
        .directory_offset
        .checked_add(row_rel)
        .ok_or(EmfAdmitError::Extent)?;
    let row = range.subrange(row_off, CHUNK_DIRECTORY_ENTRY_BYTES)?;
    let entry: EmfChunkDirectoryEntry = bytemuck::pod_read_unaligned(row.bytes());
    let length = usize::try_from(entry.byte_length).map_err(|_| EmfAdmitError::Limit)?;
    Ok(range.subrange(entry.byte_offset, length)?)
}
impl<'a> EmfStreamView<'a> {
    /// Dispatch on magic/version bytes, not filename. Whole authenticated entry only.
    /// Partial range is Extent, never a shorter valid field. No silent v2 fallback.
    pub fn admit(range: AuthenticatedRange<'a>, limits: Limits) -> Result<Self, EmfAdmitError> {
        if range.offset() != 0 || range.bytes().len() as u64 != range.selected().logical_length() {
            return Err(EmfAdmitError::Extent);
        }
        let bytes = range.bytes();
        let header = bytes.get(..10).ok_or(EmfAdmitError::Extent)?;
        if &header[..8] != Family::EmfV4Candidate.magic() {
            return Err(EmfAdmitError::Family);
        }
        let version = u16::from_le_bytes([header[8], header[9]]);
        match version {
            v if v == Family::EmfV4Candidate.version() => Self::admit_v4(range, limits),
            EMF_CONTAINER_VERSION => Self::admit_v2(range, limits),
            EMF_EXTENDED_VERSION => Self::admit_v3(range, limits),
            EMF_LEGACY_VERSION => Err(EmfAdmitError::Unsupported),
            _ => Err(EmfAdmitError::Version),
        }
    }
    fn admit_v4(range: AuthenticatedRange<'a>, limits: Limits) -> Result<Self, EmfAdmitError> {
        let structure = StructuralView::open(range.bytes(), Family::EmfV4Candidate, limits)?;
        let profile = if unknown_critical(&structure) {
            EmfProfile::OpaqueArchival
        } else {
            EmfProfile::EnvelopeV4
        };
        Ok(Self {
            range,
            inner: EmfInner::EnvelopeV4(structure),
            profile,
        })
    }
    fn admit_v2(range: AuthenticatedRange<'a>, limits: Limits) -> Result<Self, EmfAdmitError> {
        let (header, directory) = split_header_directory(range)?;
        let mut scratch = [PayloadInterval::default(); EMF_STREAM_SCRATCH_ROWS];
        let view = EmfDirectoryView::admit(header, directory, format_limits(limits)?, &mut scratch)?;
        Ok(Self {
            range,
            inner: EmfInner::DirectoryV2(view),
            profile: EmfProfile::DirectoryV2,
        })
    }
    fn admit_v3(range: AuthenticatedRange<'a>, limits: Limits) -> Result<Self, EmfAdmitError> {
        let (header, directory) = split_header_directory(range)?;
        let tail = split_v3_tail(range)?;
        let mut scratch = [PayloadInterval::default(); EMF_STREAM_SCRATCH_ROWS];
        let view = EmfV3DirectoryView::admit(
            header,
            directory,
            tail,
            format_limits(limits)?,
            extension_limits(limits)?,
            &mut scratch,
        )?;
        Ok(Self {
            range,
            inner: EmfInner::InterpretationV3(view),
            profile: EmfProfile::InterpretationV3,
        })
    }
    pub const fn profile(&self) -> EmfProfile {
        self.profile
    }
    pub const fn ready_for_plan(&self) -> bool {
        !matches!(self.profile, EmfProfile::OpaqueArchival)
    }
    pub const fn range(&self) -> AuthenticatedRange<'a> {
        self.range
    }
    /// Envelope v4 structural view. Brick v2/v3 have none.
    pub fn structure(&self) -> Option<&StructuralView<'a>> {
        match &self.inner {
            EmfInner::EnvelopeV4(structure) => Some(structure),
            EmfInner::DirectoryV2(_) | EmfInner::InterpretationV3(_) => None,
        }
    }
    /// Zero-based selected chunk. Not a whole-volume receipt.
    pub fn select_region(&self, chunk_or_entry_index: usize) -> Result<&'a [u8], EmfAdmitError> {
        match &self.inner {
            EmfInner::EnvelopeV4(structure) => {
                let index = u32::try_from(chunk_or_entry_index)
                    .ok()
                    .and_then(|i| i.checked_add(1))
                    .ok_or(EmfAdmitError::Envelope(field_envelope::Error::EntryIndex))?;
                Ok(structure.verify_chunk(index)?)
            }
            EmfInner::DirectoryV2(view) => {
                let payload = chunk_payload(self.range, view.header(), chunk_or_entry_index)?;
                Ok(view.selected_chunk(chunk_or_entry_index, payload)?.records())
            }
            EmfInner::InterpretationV3(view) => {
                let payload = chunk_payload(self.range, view.header(), chunk_or_entry_index)?;
                Ok(view.selected_chunk(chunk_or_entry_index, payload)?.records())
            }
        }
    }
    /// Declared `(start_tick, time_count)` when present. Never invented.
    pub fn declared_interval(&self, chunk_or_entry_index: usize) -> Option<(i64, u64)> {
        match &self.inner {
            EmfInner::EnvelopeV4(structure) => {
                let index = u32::try_from(chunk_or_entry_index)
                    .ok()
                    .and_then(|i| i.checked_add(1))?;
                let entry = structure.entry(index).ok()?;
                if entry.flags() & 2 == 0 {
                    None
                } else {
                    Some((entry.start_tick(), entry.time_count()))
                }
            }
            EmfInner::DirectoryV2(_) => None,
            EmfInner::InterpretationV3(view) => {
                let payload = chunk_payload(self.range, view.header(), chunk_or_entry_index).ok()?;
                let chunk = view.selected_chunk(chunk_or_entry_index, payload).ok()?;
                let timebase = &view.extension().metadata().field.timebase;
                let start = timebase.slice_tick(chunk.entry().time_slice_index).ok()?;
                Some((start, timebase.slice_step_ticks))
            }
        }
    }
}
/// Planner `present` id. Successful structural admit of a mandatory Emf entry.
pub const fn emf_present_id(entry_id: u32, view: &EmfStreamView<'_>) -> Option<u32> {
    if view.ready_for_plan() {
        Some(entry_id)
    } else {
        None
    }
}
#[cfg(test)]
#[path = "emf_stream_tests.rs"]
mod emf_stream_tests;
