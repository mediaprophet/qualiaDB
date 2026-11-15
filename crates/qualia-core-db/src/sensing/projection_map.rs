//! Explicit scalar-observation mapping into declared Tensor10D axes.
//!
//! Coefficients, offsets, units, and interpretation IDs are caller supplied.
//! This module performs no unit conversion, calibration, scientific inference,
//! or universal mapping from a physical quantity to Tensor10D coordinates.
//! Propagated uncertainty covers the declared input uncertainty and f32
//! conversion only; unknown sensor/calibration errors and intermediate f64
//! rounding are not included, so it is not a complete error bound.

use super::{AxisClaim, PhysicalFieldSemantics, ProjectionError, SemanticId, SemanticProjection};

/// A finite scalar observation and its explicit physical quantity and unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalObservation {
    quantity_id: SemanticId,
    unit_id: SemanticId,
    value: ObservationValue,
    absolute_uncertainty: ObservationUncertainty,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObservationValue {
    Unknown,
    Known(f64),
}

/// Absolute uncertainty in the same unit as the associated observation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObservationUncertainty {
    Unknown,
    Known(f64),
}

impl PhysicalObservation {
    pub fn new(
        quantity_id: SemanticId,
        unit_id: SemanticId,
        value: ObservationValue,
        absolute_uncertainty: ObservationUncertainty,
    ) -> Result<Self, ProjectionMapError> {
        if quantity_id.value().is_none() || unit_id.value().is_none() {
            return Err(ProjectionMapError::UnknownPhysicalSemantics);
        }
        if let ObservationValue::Known(value) = value {
            if !value.is_finite() {
                return Err(ProjectionMapError::InvalidObservation);
            }
        }
        if let ObservationUncertainty::Known(value) = absolute_uncertainty {
            if !value.is_finite() || value < 0.0 {
                return Err(ProjectionMapError::InvalidObservation);
            }
        }
        Ok(Self {
            quantity_id,
            unit_id,
            value,
            absolute_uncertainty,
        })
    }

    pub const fn quantity_id(self) -> SemanticId {
        self.quantity_id
    }
    pub const fn unit_id(self) -> SemanticId {
        self.unit_id
    }
    pub const fn value(self) -> ObservationValue {
        self.value
    }
    pub const fn absolute_uncertainty(self) -> ObservationUncertainty {
        self.absolute_uncertainty
    }
}

/// Caller-declared affine mapping: `axis = observation * coefficient + offset`.
/// The coefficient's implied units and coordinate convention are identified
/// by the containing `ProjectionMap`; this type does not interpret them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisMapping {
    coefficient: f64,
    offset: f64,
}

impl AxisMapping {
    pub fn new(coefficient: f64, offset: f64) -> Result<Self, ProjectionMapError> {
        if !coefficient.is_finite() || !offset.is_finite() {
            return Err(ProjectionMapError::InvalidMapping);
        }
        Ok(Self {
            coefficient,
            offset,
        })
    }

    pub const fn coefficient(self) -> f64 {
        self.coefficient
    }
    pub const fn offset(self) -> f64 {
        self.offset
    }
}

/// Mapping declaration for one physical field into one interpretation profile.
/// `axes[i] == None` means that coordinate is unknown and remains so.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectionMap {
    mapping_id: u64,
    physical: PhysicalFieldSemantics,
    profile_id: SemanticId,
    domain_id: SemanticId,
    coordinate_convention_id: SemanticId,
    quantity_id: SemanticId,
    unit_id: SemanticId,
    axes: [Option<AxisMapping>; 10],
}

impl ProjectionMap {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mapping_id: u64,
        physical: PhysicalFieldSemantics,
        profile_id: SemanticId,
        domain_id: SemanticId,
        coordinate_convention_id: SemanticId,
        quantity_id: SemanticId,
        unit_id: SemanticId,
        axes: [Option<AxisMapping>; 10],
    ) -> Result<Self, ProjectionMapError> {
        if mapping_id == 0 {
            return Err(ProjectionMapError::InvalidIdentifier);
        }
        if profile_id.value().is_none()
            || domain_id.value().is_none()
            || coordinate_convention_id.value().is_none()
            || quantity_id.value().is_none()
            || unit_id.value().is_none()
        {
            return Err(ProjectionMapError::UnknownInterpretation);
        }
        if physical.physical_quantity().value() != quantity_id.value() {
            return Err(ProjectionMapError::QuantityMismatch);
        }
        if physical.typed_unit_id().value() != unit_id.value() {
            return Err(ProjectionMapError::UnitMismatch);
        }
        Ok(Self {
            mapping_id,
            physical,
            profile_id,
            domain_id,
            coordinate_convention_id,
            quantity_id,
            unit_id,
            axes,
        })
    }

    pub const fn mapping_id(self) -> u64 {
        self.mapping_id
    }

    /// Apply this exact mapping declaration to one observation. Unknown input
    /// values and unmapped axes remain `AxisClaim::Unknown`; numeric overflow
    /// rejects the whole result before a projection is returned.
    pub fn project(
        self,
        observation: PhysicalObservation,
    ) -> Result<MappedProjection, ProjectionMapError> {
        if observation.quantity_id != self.quantity_id {
            return Err(ProjectionMapError::QuantityMismatch);
        }
        if observation.unit_id != self.unit_id {
            return Err(ProjectionMapError::UnitMismatch);
        }

        let mut axes = [AxisClaim::Unknown; 10];
        let mut axis_uncertainty = [AxisUncertainty::Unknown; 10];
        if let ObservationValue::Known(value) = observation.value {
            let mut i = 0;
            while i < 10 {
                if let Some(mapping) = self.axes[i] {
                    let mapped = value * mapping.coefficient + mapping.offset;
                    let mapped_f32 = mapped as f32;
                    if !mapped.is_finite() || !mapped_f32.is_finite() {
                        return Err(ProjectionMapError::CoordinateOverflow { axis: i });
                    }
                    axes[i] =
                        AxisClaim::known(mapped_f32).map_err(ProjectionMapError::Projection)?;
                    if let ObservationUncertainty::Known(uncertainty) =
                        observation.absolute_uncertainty
                    {
                        // Include the deterministic f64-to-f32 coordinate
                        // quantization error in the propagated absolute bound.
                        let rounding_error = (mapped - mapped_f32 as f64).abs();
                        let mapped_uncertainty =
                            uncertainty * mapping.coefficient.abs() + rounding_error;
                        let uncertainty_f32 = round_up_f64_to_f32(mapped_uncertainty)
                            .ok_or(ProjectionMapError::UncertaintyOverflow { axis: i })?;
                        axis_uncertainty[i] = AxisUncertainty::Known(uncertainty_f32);
                    }
                }
                i += 1;
            }
        }

        let projection = SemanticProjection::new(
            self.physical,
            self.profile_id,
            self.domain_id,
            self.coordinate_convention_id,
            axes,
        )
        .map_err(ProjectionMapError::Projection)?;
        Ok(MappedProjection {
            mapping_id: self.mapping_id,
            observation,
            projection,
            axis_uncertainty,
        })
    }
}

/// Output keeps physical observation and uncertainty alongside the 10D
/// projection. The tensor is an interpretation, not a replacement for the
/// physical value or evidence record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MappedProjection {
    mapping_id: u64,
    observation: PhysicalObservation,
    projection: SemanticProjection,
    axis_uncertainty: [AxisUncertainty; 10],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AxisUncertainty {
    Unknown,
    /// Rounded outward to f32. This includes scaled input uncertainty and
    /// f64-to-f32 coordinate quantization only. It excludes f64 intermediate
    /// arithmetic error and any sensor, calibration, or model error.
    Known(f32),
}

/// Convert a nonnegative finite f64 bound to f32 without rounding inward.
/// `None` means the value cannot be represented as a finite f32 upper bound.
fn round_up_f64_to_f32(value: f64) -> Option<f32> {
    if !value.is_finite() || value < 0.0 || value > f32::MAX as f64 {
        return None;
    }
    let mut rounded = value as f32;
    if !rounded.is_finite() {
        return None;
    }
    if (rounded as f64) < value {
        let next_bits = rounded.to_bits().checked_add(1)?;
        rounded = f32::from_bits(next_bits);
        if !rounded.is_finite() {
            return None;
        }
    }
    Some(rounded)
}

impl MappedProjection {
    pub const fn mapping_id(self) -> u64 {
        self.mapping_id
    }
    pub const fn observation(self) -> PhysicalObservation {
        self.observation
    }
    pub const fn projection(&self) -> &SemanticProjection {
        &self.projection
    }
    pub const fn axis_uncertainty(&self) -> &[AxisUncertainty; 10] {
        &self.axis_uncertainty
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionMapError {
    InvalidIdentifier,
    UnknownPhysicalSemantics,
    UnknownInterpretation,
    InvalidObservation,
    InvalidMapping,
    QuantityMismatch,
    UnitMismatch,
    CoordinateOverflow { axis: usize },
    UncertaintyOverflow { axis: usize },
    Projection(ProjectionError),
}

#[cfg(test)]
#[path = "projection_map_tests.rs"]
mod tests;
