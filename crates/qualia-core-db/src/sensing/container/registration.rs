//! Opt-in sensing-owned MID2 profile; no universal numeric registry allocation.
use super::super::record::MeasurementRecord;
use super::{
    pack_sensing_index_field, validate_kind, PackedSensingIndex, SensingContainerError,
    SensingIndexView,
};
use crate::container_10d::{
    typed_payloads::{TypedPayloadError, TypedPayloadResolver},
    TypedFieldRecord,
};

const _: () = assert!(
    super::super::record::MEASUREMENT_INDEX_VERSION == 1
        && super::super::record::MEASUREMENT_INDEX_RECORD_SIZE == 264
);

/// Local profile choice only. Hosts must refuse a conflicting assignment or
/// use another explicitly registered extension kind through `new`.
pub const NATIVE_SENSING_INDEX_KIND_V1: u16 = 0x0400;
pub const SENSING_INDEX_SCHEMA_V1: &str = "urn:qualia:sensing:measurement-index:v1";
pub const SENSING_INDEX_SCHEMA_V1_HASH: u64 = crate::q_hash(SENSING_INDEX_SCHEMA_V1);
pub const MAX_SENSING_PROFILE_EXTENSIONS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SensingIndexProfile {
    kind: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    InvalidKind,
    ConflictingKind,
    Capacity,
}

impl SensingIndexProfile {
    pub const fn native_v1() -> Self {
        Self {
            kind: NATIVE_SENSING_INDEX_KIND_V1,
        }
    }
    pub fn new(kind: u16) -> Result<Self, RegistrationError> {
        validate_kind(kind).map_err(|_| RegistrationError::InvalidKind)?;
        Ok(Self { kind })
    }
    pub const fn kind(self) -> u16 {
        self.kind
    }

    /// `other` contains only other extensions. All refusals precede writes;
    /// repeated kinds, built-ins and competing assignments are rejected.
    pub fn write_supported_kinds(
        self,
        other: &[u16],
        output: &mut [u16],
    ) -> Result<usize, RegistrationError> {
        if other.len() >= MAX_SENSING_PROFILE_EXTENSIONS || output.len() < other.len() + 1 {
            return Err(RegistrationError::Capacity);
        }
        validate_kind(self.kind).map_err(|_| RegistrationError::InvalidKind)?;
        for (i, kind) in other.iter().enumerate() {
            validate_kind(*kind).map_err(|_| RegistrationError::InvalidKind)?;
            if *kind == self.kind || other[..i].contains(kind) {
                return Err(RegistrationError::ConflictingKind);
            }
        }
        let count = other.len() + 1;
        output[0] = self.kind;
        output[1..count].copy_from_slice(other);
        output[..count].sort_unstable();
        Ok(count)
    }

    pub fn pack(
        self,
        container_id: [u8; 16],
        index_field_id: u64,
        raw_field_prefix: &[u8],
        records: &[MeasurementRecord],
        output: &mut [u8],
    ) -> Result<PackedSensingIndex, SensingContainerError> {
        pack_sensing_index_field(
            container_id,
            index_field_id,
            self.kind,
            raw_field_prefix,
            records,
            output,
        )
    }
    pub fn open<'r, 'data, 'fields>(
        self,
        resolver: &'r TypedPayloadResolver<'data, 'fields>,
        index_field_id: u64,
        container_id: [u8; 16],
    ) -> Result<SensingIndexView<'r, 'data, 'fields>, SensingContainerError> {
        SensingIndexView::open(resolver, index_field_id, self.kind, container_id)
    }

    /// The temporary allow-list is not retained by the checked resolver.
    pub fn resolve<'data, 'fields>(
        self,
        data: &'data [u8],
        other_extensions: &[u16],
        field_scratch: &mut [TypedFieldRecord],
        fields: &'fields mut [TypedFieldRecord],
    ) -> Result<TypedPayloadResolver<'data, 'fields>, ProfileResolverError> {
        let mut extensions = [0u16; MAX_SENSING_PROFILE_EXTENSIONS];
        let count = self
            .write_supported_kinds(other_extensions, &mut extensions)
            .map_err(ProfileResolverError::Registration)?;
        TypedPayloadResolver::new(data, &extensions[..count], field_scratch, fields)
            .map_err(ProfileResolverError::Payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileResolverError {
    Registration(RegistrationError),
    Payload(TypedPayloadError),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_registration_is_canonical_and_transactional() {
        let profile = SensingIndexProfile::native_v1();
        let mut out = [0xffff; 4];
        assert_eq!(
            profile.write_supported_kinds(&[0x8101, 0x0500], &mut out),
            Ok(3)
        );
        assert_eq!(out, [NATIVE_SENSING_INDEX_KIND_V1, 0x0500, 0x8101, 0xffff]);
        let before = out;
        assert_eq!(
            profile.write_supported_kinds(&[NATIVE_SENSING_INDEX_KIND_V1], &mut out),
            Err(RegistrationError::ConflictingKind)
        );
        assert_eq!(
            profile.write_supported_kinds(&[0x8101, 0x8101], &mut out),
            Err(RegistrationError::ConflictingKind)
        );
        assert_eq!(
            profile.write_supported_kinds(&[1], &mut out),
            Err(RegistrationError::InvalidKind)
        );
        assert_eq!(out, before);
    }
    #[test]
    fn native_kind_is_opt_in_and_remappable() {
        assert!(
            !crate::container_10d::manifold_identity_v2::BUILTIN_FIELD_KINDS
                .contains(&NATIVE_SENSING_INDEX_KIND_V1)
        );
        assert!(SensingIndexProfile::new(0).is_err());
        assert!(SensingIndexProfile::new(0x0100).is_err());
        assert_eq!(SensingIndexProfile::new(0x8101).unwrap().kind(), 0x8101);
    }
}
