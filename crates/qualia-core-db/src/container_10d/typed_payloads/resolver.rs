//! Validated borrowed field selection; codec readers remain separate.
use super::super::{
    Container10dHeader, FIELD_KIND_MORPH_TARGETS, FIELD_KIND_SURFACE_FRAMES, HeaderParseError,
    ManifoldContainerError, SectionDescriptor, SectionTableError, SectionType, TypedFieldRecord,
    decode_container_manifold_identity, parse_section_table,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedPayloadError {
    Header(HeaderParseError),
    Sections(SectionTableError),
    Manifest(ManifoldContainerError),
    MissingPayload { field_id: u64 },
    AmbiguousKind { kind: u16 },
    InvalidSemanticBinding { field_id: u64 },
    VertexCount { expected: usize, got: usize },
    Frames(super::super::NormalSectionError),
    Morph(super::super::MorphSectionError),
}
impl std::fmt::Display for TypedPayloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".10d typed payload: {self:?}")
    }
}
impl std::error::Error for TypedPayloadError {}

/// A borrowed exact codec payload and its validated declaration. The record's
/// identifiers remain qualified by their manifest; they do not establish authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypedPayload<'a> {
    pub record: TypedFieldRecord,
    pub bytes: &'a [u8],
}

/// Validated envelope view. Record storage belongs to the caller. Creating and
/// selecting this view performs no heap allocation; codec decoding is separate.
pub struct TypedPayloadResolver<'a, 'f> {
    data: &'a [u8],
    sections: &'a [SectionDescriptor],
    fields: &'f [TypedFieldRecord],
    has_manifest: bool,
}
impl<'a, 'f> TypedPayloadResolver<'a, 'f> {
    /// All manifest kinds and ranges validate before out_fields changes. Scratch
    /// may change on refusal. Existing envelope section CRC checks are retained;
    /// whole-file sealing remains the enclosing asset admission owner's policy.
    pub fn new(
        data: &'a [u8],
        supported_extension_kinds: &[u16],
        field_scratch: &mut [TypedFieldRecord],
        out_fields: &'f mut [TypedFieldRecord],
    ) -> Result<Self, TypedPayloadError> {
        let header = Container10dHeader::parse(data).map_err(TypedPayloadError::Header)?;
        let sections = parse_section_table(data, &header).map_err(TypedPayloadError::Sections)?;
        let has_manifest = sections
            .iter()
            .any(|s| s.typ() == Some(SectionType::ManifoldIdentityV2));
        let count = if has_manifest {
            decode_container_manifold_identity(
                data,
                &header,
                supported_extension_kinds,
                field_scratch,
                out_fields,
            )
            .map_err(TypedPayloadError::Manifest)?
            .1
        } else {
            0
        };
        Ok(Self {
            data,
            sections,
            fields: &out_fields[..count],
            has_manifest,
        })
    }

    pub fn has_manifest(&self) -> bool {
        self.has_manifest
    }
    pub fn fields(&self) -> &[TypedFieldRecord] {
        self.fields
    }

    /// Select by stable field declaration identifier, independent of byte order.
    pub fn field(&self, field_id: u64) -> Result<Option<TypedPayload<'a>>, TypedPayloadError> {
        self.fields
            .iter()
            .find(|f| f.field_id == field_id)
            .map(|record| self.payload(*record))
            .transpose()
    }

    /// Unique registered presentation field. A generic Vector is never selected.
    pub fn surface_frames(&self) -> Result<Option<TypedPayload<'a>>, TypedPayloadError> {
        self.unique(FIELD_KIND_SURFACE_FRAMES)
    }
    /// Morph records use a mixed schema: position deltas inherit mesh coordinate
    /// convention, while normal/tangent deltas are dimensionless. Their enclosing
    /// declaration has unit_id=0; scientific field units remain independent.
    pub fn morph_targets(&self) -> Result<Option<TypedPayload<'a>>, TypedPayloadError> {
        self.unique(FIELD_KIND_MORPH_TARGETS)
    }

    fn unique(&self, kind: u16) -> Result<Option<TypedPayload<'a>>, TypedPayloadError> {
        let mut found = self.fields.iter().filter(|f| f.kind == kind);
        let Some(record) = found.next() else {
            return Ok(None);
        };
        if found.next().is_some() {
            return Err(TypedPayloadError::AmbiguousKind { kind });
        }
        if record.payload_section_id != SectionType::FieldSidecar as u32 || record.unit_id != 0 {
            return Err(TypedPayloadError::InvalidSemanticBinding {
                field_id: record.field_id,
            });
        }
        self.payload(*record).map(Some)
    }

    fn payload(&self, record: TypedFieldRecord) -> Result<TypedPayload<'a>, TypedPayloadError> {
        if record.payload_section_id == 0 || record.payload_length == 0 {
            return Err(TypedPayloadError::MissingPayload {
                field_id: record.field_id,
            });
        }
        let section = self
            .sections
            .iter()
            .find(|s| u32::from(s.section_type) == record.payload_section_id)
            .ok_or(TypedPayloadError::MissingPayload {
                field_id: record.field_id,
            })?;
        let start = u64::from(section.byte_offset).checked_add(record.payload_offset);
        let range = start
            .and_then(|start| {
                record
                    .payload_length
                    .checked_add(start)
                    .map(|end| (start, end))
            })
            .and_then(|(start, end)| {
                Some((usize::try_from(start).ok()?, usize::try_from(end).ok()?))
            });
        let bytes = range
            .and_then(|(start, end)| self.data.get(start..end))
            .ok_or(TypedPayloadError::MissingPayload {
                field_id: record.field_id,
            })?;
        Ok(TypedPayload { record, bytes })
    }

    fn legacy_sidecar(&self) -> Option<&'a [u8]> {
        let section = self
            .sections
            .iter()
            .find(|s| s.typ() == Some(SectionType::FieldSidecar))?;
        let start = section.byte_offset as usize;
        let payload = self
            .data
            .get(start..start.checked_add(section.byte_length as usize)?)?;
        Some(payload)
    }

    pub(super) fn legacy_frames(&self) -> Option<&'a [u8]> {
        self.legacy_sidecar()
            .filter(|payload| super::super::is_normal_section(payload))
    }

    pub(super) fn legacy_morph_targets(&self) -> Option<&'a [u8]> {
        self.legacy_sidecar()
            .filter(|payload| payload.starts_with(&super::super::MAGIC_MRP1))
    }
}
