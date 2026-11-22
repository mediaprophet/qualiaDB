//! Validated borrowed field selection; codec readers remain separate.
use super::super::{
    ArchetypeRefError, Container10dHeader, FIELD_KIND_ARCHETYPE_REF, FIELD_KIND_INSTANCE_OVERRIDES,
    FIELD_KIND_MORPH_TARGETS, FIELD_KIND_SURFACE_FRAMES, HeaderParseError, InstanceOverrideHeader,
    ManifoldContainerError, ManifoldIdentityV2, SectionDescriptor, SectionTableError, SectionType,
    SsmInstanceCoeff, StableEntityId, TypedFieldRecord, decode_archetype_ref_record,
    decode_container_manifold_identity, decode_instance_overrides_into, parse_section_table,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedPayloadError {
    Header(HeaderParseError),
    Sections(SectionTableError),
    Manifest(ManifoldContainerError),
    MissingPayload { field_id: u64 },
    AmbiguousKind { kind: u16 },
    InvalidSemanticBinding { field_id: u64 },
    OverridePairCount { references: usize, overrides: usize },
    OverrideCodec { field_id: u64, error: ArchetypeRefError },
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

/// Semantic anchor for an override selected in canonical MID2 field order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceOverrideIdentity {
    Leaf {
        archetype_entity_id: StableEntityId,
        taxon_or_class_iri_hash: u64,
        subresource_key_hash: u64,
    },
    Standalone(StableEntityId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundInstanceOverride {
    pub identity: InstanceOverrideIdentity,
    pub header: InstanceOverrideHeader,
    pub coefficient_count: usize,
}

/// Validated envelope view. Record storage belongs to the caller. Creating and
/// selecting this view performs no heap allocation; codec decoding is separate.
pub struct TypedPayloadResolver<'a, 'f> {
    data: &'a [u8],
    sections: &'a [SectionDescriptor],
    fields: &'f [TypedFieldRecord],
    identity: Option<ManifoldIdentityV2>,
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
        let (identity, count) = if has_manifest {
            let (identity, count) = decode_container_manifold_identity(
                data,
                &header,
                supported_extension_kinds,
                field_scratch,
                out_fields,
            )
            .map_err(TypedPayloadError::Manifest)?;
            (Some(identity), count)
        } else {
            (None, 0)
        };
        Ok(Self {
            data,
            sections,
            fields: &out_fields[..count],
            identity,
        })
    }

    pub fn has_manifest(&self) -> bool {
        self.identity.is_some()
    }
    pub fn identity(&self) -> Option<&ManifoldIdentityV2> {
        self.identity.as_ref()
    }
    pub fn fields(&self) -> &[TypedFieldRecord] {
        self.fields
    }

    pub fn section_payload(&self, kind: SectionType) -> Option<&'a [u8]> {
        let section = self.sections.iter().find(|section| section.typ() == Some(kind))?;
        let start = section.byte_offset as usize;
        let end = start.checked_add(section.byte_length as usize)?;
        self.data.get(start..end)
    }

    /// Visit payloads of one kind in ascending MID2 field ID order.
    pub fn payloads_of_kind(
        &self,
        kind: u16,
    ) -> impl Iterator<Item = Result<TypedPayload<'a>, TypedPayloadError>> + '_ {
        self.fields
            .iter()
            .filter(move |record| record.kind == kind)
            .map(|record| self.payload(*record))
    }

    /// Validate the cardinality of the ordered AREF and IOVR lists.
    pub fn instance_override_count(&self) -> Result<usize, TypedPayloadError> {
        let references = self.payloads_of_kind(FIELD_KIND_ARCHETYPE_REF).count();
        let overrides = self.payloads_of_kind(FIELD_KIND_INSTANCE_OVERRIDES).count();
        if overrides == references || (overrides == 1 && references == 0 && self.has_manifest()) {
            Ok(overrides)
        } else {
            Err(TypedPayloadError::OverridePairCount { references, overrides })
        }
    }

    /// Validate authored overrides before admitting a renderable asset. A lone
    /// legacy archetype reference can carry no override state.
    pub fn validate_instance_overrides(&self) -> Result<(), TypedPayloadError> {
        if self.payloads_of_kind(FIELD_KIND_INSTANCE_OVERRIDES).next().is_none()
            && self.payloads_of_kind(FIELD_KIND_ARCHETYPE_REF).count() <= 1
        {
            return Ok(());
        }
        let count = self.instance_override_count()?;
        let mut coefficients = [SsmInstanceCoeff::default(); super::super::MAX_INSTANCE_SSM_COEFFS];
        for slot in 0..count {
            self.instance_override_into(slot, &mut coefficients)?;
        }
        Ok(())
    }

    /// Decode the kth IOVR and resolve it through the kth AREF or the standalone MID2 identity.
    /// The caller owns coefficient scratch storage.
    pub fn instance_override_into(
        &self,
        index: usize,
        coefficients: &mut [SsmInstanceCoeff],
    ) -> Result<Option<BoundInstanceOverride>, TypedPayloadError> {
        if index >= self.instance_override_count()? {
            return Ok(None);
        }
        let Some(override_record) = self.payloads_of_kind(FIELD_KIND_INSTANCE_OVERRIDES).nth(index)
        else {
            return Ok(None);
        };
        let iovr = override_record?;
        let identity = if let Some(aref) = self.payloads_of_kind(FIELD_KIND_ARCHETYPE_REF).nth(index) {
            let aref = aref?;
            let record = decode_archetype_ref_record(aref.bytes).map_err(|error| {
                TypedPayloadError::OverrideCodec { field_id: aref.record.field_id, error }
            })?;
            if record.taxon_or_class_iri_hash == 0 || record.subresource_key_hash == 0 {
                return Err(TypedPayloadError::InvalidSemanticBinding { field_id: aref.record.field_id });
            }
            InstanceOverrideIdentity::Leaf {
                archetype_entity_id: StableEntityId(record.archetype_entity_id),
                taxon_or_class_iri_hash: record.taxon_or_class_iri_hash,
                subresource_key_hash: record.subresource_key_hash,
            }
        } else {
            let identity = self.identity().ok_or(TypedPayloadError::InvalidSemanticBinding {
                field_id: iovr.record.field_id,
            })?;
            InstanceOverrideIdentity::Standalone(identity.entity_id)
        };
        let (header, coefficient_count) = decode_instance_overrides_into(iovr.bytes, coefficients)
            .map_err(|error| TypedPayloadError::OverrideCodec {
                field_id: iovr.record.field_id, error,
            })?;
        Ok(Some(BoundInstanceOverride { identity, header, coefficient_count }))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::{
        encode_archetype_ref_record, encode_instance_overrides, ArchetypeDigestAlg,
        ArchetypeTargetKind, ARCHETYPE_REF_RECORD_SIZE, INSTANCE_OVERRIDE_HEADER_SIZE,
    };

    #[test]
    fn pairs_each_override_by_ordinal_even_when_payloads_are_interleaved() {
        let mut bytes = Vec::new();
        let mut records = Vec::new();
        for slot in 0..2 {
            let mut aref = [0; ARCHETYPE_REF_RECORD_SIZE];
            let reference = super::super::super::ArchetypeRefRecord::new(
                ArchetypeDigestAlg::Sha256,
                ArchetypeTargetKind::HmcBundleEntry,
                0,
                [slot as u8 + 1; 32],
                [slot as u8 + 1; 16],
                11,
                slot + 21,
            );
            encode_archetype_ref_record(&reference, &mut aref).unwrap();
            let mut iovr = [0; INSTANCE_OVERRIDE_HEADER_SIZE];
            encode_instance_overrides(
                0.0, slot as f32 + 1.0, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                &[], &mut iovr,
            ).unwrap();
            for (kind, payload) in [
                (FIELD_KIND_ARCHETYPE_REF, aref.as_slice()),
                (FIELD_KIND_INSTANCE_OVERRIDES, iovr.as_slice()),
            ] {
                records.push(TypedFieldRecord {
                    field_id: records.len() as u64 + 1,
                    kind,
                    payload_section_id: SectionType::FieldSidecar as u32,
                    payload_offset: bytes.len() as u64,
                    payload_length: payload.len() as u64,
                    ..TypedFieldRecord::default()
                });
                bytes.extend_from_slice(payload);
            }
        }
        let sections = [SectionDescriptor {
            section_type: SectionType::FieldSidecar as u8,
            alignment_tier: 0,
            reserved16: 0,
            byte_offset: 0,
            byte_length: bytes.len() as u32,
            stride: 0,
            element_count: 0,
            crc32c: 0,
        }];
        let view = TypedPayloadResolver {
            data: &bytes, sections: &sections, fields: &records, identity: None,
        };
        assert_eq!(view.instance_override_count().unwrap(), 2);
        let bound = view.instance_override_into(1, &mut []).unwrap().unwrap();
        assert_eq!(bound.header.uniform_scale, 2.0);
        assert_eq!(bound.identity, InstanceOverrideIdentity::Leaf {
            archetype_entity_id: StableEntityId([2; 16]),
            taxon_or_class_iri_hash: 11,
            subresource_key_hash: 22,
        });
        let missing_override = TypedPayloadResolver { fields: &records[..3], ..view };
        assert!(matches!(missing_override.instance_override_count(),
            Err(TypedPayloadError::OverridePairCount { references: 2, overrides: 1 })
        ));
        assert_eq!(core::mem::size_of::<InstanceOverrideHeader>(), 48);
    }
}
