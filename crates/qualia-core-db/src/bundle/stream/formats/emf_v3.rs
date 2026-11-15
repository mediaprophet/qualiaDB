//! Typed EMF v3 range view with complete interpretation/PCM closure.
use super::{
    AuthenticatedRange, EmfChunkView, EmfDirectoryView, FormatError as E, FormatLimits,
    PayloadInterval,
};
use crate::q42::emf_container::{
    extension::{
        parse_extension_tail, validate_directory_bytes, EmfExtensionLimits, EmfExtensionView,
    },
    header::EMF_EXTENDED_VERSION,
};
/// Only selected bricks are verified. This is not a complete field-volume receipt.
pub struct EmfV3DirectoryView<'a> {
    directory: EmfDirectoryView<'a>,
    extension: EmfExtensionView<'a>,
}
impl<'a> EmfV3DirectoryView<'a> {
    pub fn admit(
        header: AuthenticatedRange<'a>,
        directory: AuthenticatedRange<'a>,
        tail: AuthenticatedRange<'a>,
        limits: FormatLimits,
        extension_limits: EmfExtensionLimits,
        scratch: &mut [PayloadInterval],
    ) -> Result<Self, E> {
        if !header.same_entry(&tail) || !directory.same_entry(&tail) {
            return Err(E::Identity);
        }
        let total = usize::try_from(header.selected().logical_length()).map_err(|_| E::Limit)?;
        let field_end = usize::try_from(tail.offset()).map_err(|_| E::Bounds)?;
        if field_end.checked_add(tail.bytes().len()) != Some(total) {
            return Err(E::Bounds);
        }
        // Metadata admission is cold. PCM stays in original authenticated bytes;
        // its separate caller-selected byte ceiling remains mandatory.
        let directory_bytes = directory.bytes().len().checked_add(64).ok_or(E::Bounds)?;
        if directory_bytes
            .checked_add(extension_limits.max_metadata_bytes)
            .ok_or(E::Bounds)?
            > limits.max_metadata_bytes
        {
            return Err(E::Limit);
        }
        let base = EmfDirectoryView::admit_version(
            header,
            directory,
            limits,
            scratch,
            EMF_EXTENDED_VERSION,
        )?;
        let extension = parse_extension_tail(tail.bytes(), field_end, total, extension_limits)
            .map_err(|_| E::Integrity)?;
        validate_directory_bytes(base.header(), directory.bytes(), &extension)
            .map_err(|_| E::Geometry)?;
        Ok(Self {
            directory: base,
            extension,
        })
    }
    pub fn header(&self) -> &crate::q42::emf_container::header::EmfContainerHeader {
        self.directory.header()
    }
    pub fn extension(&self) -> &EmfExtensionView<'a> {
        &self.extension
    }
    pub fn selected_chunk(
        &self,
        index: usize,
        payload: AuthenticatedRange<'a>,
    ) -> Result<EmfChunkView<'a>, E> {
        self.directory.selected_chunk(index, payload)
    }
}
