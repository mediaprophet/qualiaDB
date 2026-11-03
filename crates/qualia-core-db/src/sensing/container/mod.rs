//! Sensing indexes in existing `.10d` MID2 extension fields.
//!
//! No section discriminant or global field kind is allocated. Hosts explicitly
//! register the extension kind in their MID2 allow-list and authenticate the
//! container, container identifier and FIEL/MID2 semantic pairing. CRC and
//! structural binding alone establish neither permission nor source trust.
mod assemble;
mod pack;
mod registration;
mod view;
pub use assemble::{encode_sensing_container, SensingAssemblyError};
pub use pack::{pack_sensing_index_field, PackedSensingIndex};
pub use registration::{
    ProfileResolverError, RegistrationError, SensingIndexProfile, MAX_SENSING_PROFILE_EXTENSIONS,
    NATIVE_SENSING_INDEX_KIND_V1, SENSING_INDEX_SCHEMA_V1, SENSING_INDEX_SCHEMA_V1_HASH,
};
pub use view::{ResolvedSensingMeasurement, SensingIndexView};

use super::record::RecordError;
use crate::container_10d::manifold_identity_v2::BUILTIN_FIELD_KINDS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SensingContainerError {
    InvalidExtensionKind,
    InvalidContainerId,
    InvalidFieldId,
    MissingIndex,
    InvalidIndex,
    Capacity,
    Range,
    DuplicateRecordId,
    ContainerMismatch,
    IndexPayloadOverlap,
    Resolver,
    Record(RecordError),
}

fn validate_kind(kind: u16) -> Result<(), SensingContainerError> {
    if kind == 0 || BUILTIN_FIELD_KINDS.contains(&kind) {
        return Err(SensingContainerError::InvalidExtensionKind);
    }
    Ok(())
}

fn validate_container(id: &[u8; 16]) -> Result<(), SensingContainerError> {
    if id.iter().all(|byte| *byte == 0) {
        return Err(SensingContainerError::InvalidContainerId);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
