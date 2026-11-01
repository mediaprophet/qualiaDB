//! Caller-bounded contracts for preserving physical measurement meaning when
//! projecting selected coordinates into QualiaDB's semantic Tensor10D space.
//!
//! This module does not acquire devices, convert units, infer coordinates,
//! establish calibration, verify provenance, or claim scientific truth. A
//! partial projection remains partial until a caller supplies every required
//! axis and a declared profile/domain/convention.

use core::num::NonZeroU64;

pub mod association;
pub mod association_table;
pub mod authorization;
pub mod container;
pub mod discovery;
pub mod fusion;
mod fusion_time;
pub mod grounding;
#[cfg(not(target_arch = "wasm32"))]
pub mod local_custody;
pub mod nfc_evidence;
pub mod projection_map;
pub mod quin_index;
pub mod radio_evidence;
pub mod radio_receipt;
pub mod record;
pub mod revocation;
pub mod sidecar_attestation;
pub mod signed_evidence;
pub mod signed_scope;
pub mod uncertainty;

use crate::container_10d::{
    field_section::{FieldDescriptor, FieldRepresentationKind, FieldSupportKind, FieldTypeKind},
    manifold_identity_v2::TypedFieldRecord,
};
use crate::tensor::Tensor10D;

/// Tensor10D's existing semantic axis order.
pub const PROJECTION_AXIS_ORDER: [&str; 10] = crate::container_10d::axis_role::AXIS_ORDER;

/// An opaque registry value, with missing/unknown represented explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticId {
    Unknown,
    Known(NonZeroU64),
}

impl SemanticId {
    pub const fn new(value: u64) -> Result<Self, ProjectionError> {
        match NonZeroU64::new(value) {
            Some(value) => Ok(Self::Known(value)),
            None => Err(ProjectionError::InvalidIdentifier),
        }
    }

    pub const fn from_optional(value: u64) -> Self {
        match NonZeroU64::new(value) {
            Some(value) => Self::Known(value),
            None => Self::Unknown,
        }
    }

    pub const fn value(self) -> Option<u64> {
        match self {
            Self::Unknown => None,
            Self::Known(value) => Some(value.get()),
        }
    }
}

/// Broad measurement modality. A registry-defined modality must use a
/// nonzero identifier; `Unknown` stays unknown and is never guessed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementModality {
    Unknown,
    Electromagnetic,
    Optical,
    Acoustic,
    Inertial,
    Thermal,
    Pressure,
    Registered(NonZeroU64),
}

impl MeasurementModality {
    pub const fn registered(id: u64) -> Result<Self, ProjectionError> {
        match NonZeroU64::new(id) {
            Some(id) => Ok(Self::Registered(id)),
            None => Err(ProjectionError::InvalidIdentifier),
        }
    }
}

/// Physical-field metadata paired by the caller's checked container resolver.
/// FIEL `unit_hash` and MID2 `unit_id` are both preserved independently; this
/// type does not infer a conversion or claim that those registries agree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalFieldSemantics {
    field_id: u64,
    typed_field_kind: u16,
    field_name_hash: u64,
    legacy_unit_hash: u64,
    typed_unit_id: SemanticId,
    field_type: FieldTypeKind,
    support: FieldSupportKind,
    representation: FieldRepresentationKind,
    modality: MeasurementModality,
    physical_quantity: SemanticId,
}

impl PhysicalFieldSemantics {
    /// Validate known FIEL descriptor tags while retaining the MID2 kind and
    /// unit identifier verbatim. `field` and `typed` must refer to the same
    /// selected field as established by the caller's envelope resolver.
    pub fn from_existing(
        field: &FieldDescriptor,
        typed: &TypedFieldRecord,
        modality: MeasurementModality,
        physical_quantity: SemanticId,
    ) -> Result<Self, ProjectionError> {
        if field.name_hash == 0
            || typed.field_id == 0
            || field.reserved != 0
            || field.reserved2 != 0
        {
            return Err(ProjectionError::InvalidFieldDescriptor);
        }
        if typed.flags != 0 {
            return Err(ProjectionError::InvalidTypedFieldRecord);
        }
        let field_type = FieldTypeKind::from_u8(field.type_kind)
            .ok_or(ProjectionError::UnsupportedFieldDescriptor)?;
        let support = FieldSupportKind::from_u8(field.support)
            .ok_or(ProjectionError::UnsupportedFieldDescriptor)?;
        let representation = FieldRepresentationKind::from_u8(field.representation)
            .ok_or(ProjectionError::UnsupportedFieldDescriptor)?;

        Ok(Self {
            field_id: typed.field_id,
            typed_field_kind: typed.kind,
            field_name_hash: field.name_hash,
            legacy_unit_hash: field.unit_hash,
            typed_unit_id: SemanticId::from_optional(typed.unit_id),
            field_type,
            support,
            representation,
            modality,
            physical_quantity,
        })
    }

    pub const fn field_id(self) -> u64 {
        self.field_id
    }

    /// MID2 kind is retained raw. Consumers must interpret it through the
    /// versioned typed-field registry rather than infer physical meaning here.
    pub const fn typed_field_kind(self) -> u16 {
        self.typed_field_kind
    }

    pub const fn field_name_hash(self) -> u64 {
        self.field_name_hash
    }

    pub const fn legacy_unit_hash(self) -> u64 {
        self.legacy_unit_hash
    }

    pub const fn typed_unit_id(self) -> SemanticId {
        self.typed_unit_id
    }

    pub const fn field_type(self) -> FieldTypeKind {
        self.field_type
    }

    pub const fn support(self) -> FieldSupportKind {
        self.support
    }

    pub const fn representation(self) -> FieldRepresentationKind {
        self.representation
    }

    pub const fn modality(self) -> MeasurementModality {
        self.modality
    }

    pub const fn physical_quantity(self) -> SemanticId {
        self.physical_quantity
    }
}

/// One explicit coordinate claim. `Unknown` carries no numeric default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisClaim {
    Unknown,
    Known(f32),
}

impl AxisClaim {
    pub fn known(value: f32) -> Result<Self, ProjectionError> {
        if !value.is_finite() {
            return Err(ProjectionError::InvalidCoordinate);
        }
        Ok(Self::Known(value))
    }
}

/// An interpretation declaration attached to a projection. IDs may remain
/// unknown; materialization requires them all to be known.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticProjection {
    physical: PhysicalFieldSemantics,
    profile_id: SemanticId,
    domain_id: SemanticId,
    coordinate_convention_id: SemanticId,
    axes: [AxisClaim; 10],
}

impl SemanticProjection {
    pub fn new(
        physical: PhysicalFieldSemantics,
        profile_id: SemanticId,
        domain_id: SemanticId,
        coordinate_convention_id: SemanticId,
        axes: [AxisClaim; 10],
    ) -> Result<Self, ProjectionError> {
        for axis in axes {
            if let AxisClaim::Known(value) = axis {
                if !value.is_finite() {
                    return Err(ProjectionError::InvalidCoordinate);
                }
            }
        }
        Ok(Self {
            physical,
            profile_id,
            domain_id,
            coordinate_convention_id,
            axes,
        })
    }

    pub const fn physical(&self) -> &PhysicalFieldSemantics {
        &self.physical
    }

    pub const fn profile_id(&self) -> SemanticId {
        self.profile_id
    }

    pub const fn domain_id(&self) -> SemanticId {
        self.domain_id
    }

    pub const fn coordinate_convention_id(&self) -> SemanticId {
        self.coordinate_convention_id
    }

    pub const fn axes(&self) -> &[AxisClaim; 10] {
        &self.axes
    }

    /// Materialize a Tensor10D only when every axis and interpretation ID is
    /// explicitly supplied. No q=0, alpha=1, hash-to-coordinate, or other
    /// defaults are applied.
    pub fn materialize(&self) -> Result<ProjectedTensor10D, ProjectionError> {
        require_known(self.profile_id, ProjectionError::UnknownProfile)?;
        require_known(self.domain_id, ProjectionError::UnknownDomain)?;
        require_known(
            self.coordinate_convention_id,
            ProjectionError::UnknownCoordinateConvention,
        )?;
        let mut values = [0.0f32; 10];
        for (index, axis) in self.axes.iter().enumerate() {
            values[index] = match axis {
                AxisClaim::Known(value) if value.is_finite() => *value,
                AxisClaim::Known(_) => return Err(ProjectionError::InvalidCoordinate),
                AxisClaim::Unknown => return Err(ProjectionError::UnknownAxis { index }),
            };
        }
        Ok(ProjectedTensor10D {
            tensor: Tensor10D::new(
                values[0], values[1], values[2], values[3], values[4], values[5], values[6],
                values[7], values[8], values[9],
            ),
            physical: self.physical,
            profile_id: self.profile_id,
            domain_id: self.domain_id,
            coordinate_convention_id: self.coordinate_convention_id,
        })
    }
}

/// A fully declared Tensor10D projection together with its physical meaning
/// and interpretation registry IDs. Tensor coordinates alone are not a
/// physical observation or evidence of measurement accuracy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedTensor10D {
    tensor: Tensor10D,
    physical: PhysicalFieldSemantics,
    profile_id: SemanticId,
    domain_id: SemanticId,
    coordinate_convention_id: SemanticId,
}

impl ProjectedTensor10D {
    pub const fn tensor(&self) -> &Tensor10D {
        &self.tensor
    }

    pub const fn physical(&self) -> &PhysicalFieldSemantics {
        &self.physical
    }

    pub const fn profile_id(&self) -> SemanticId {
        self.profile_id
    }

    pub const fn domain_id(&self) -> SemanticId {
        self.domain_id
    }

    pub const fn coordinate_convention_id(&self) -> SemanticId {
        self.coordinate_convention_id
    }
}

fn require_known(id: SemanticId, unknown_error: ProjectionError) -> Result<u64, ProjectionError> {
    match id.value() {
        Some(value) => Ok(value),
        None => Err(unknown_error),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidIdentifier,
    InvalidFieldDescriptor,
    InvalidTypedFieldRecord,
    UnsupportedFieldDescriptor,
    InvalidCoordinate,
    UnknownAxis { index: usize },
    UnknownProfile,
    UnknownDomain,
    UnknownCoordinateConvention,
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
