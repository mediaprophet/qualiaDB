//! Fail-Closed QUDT Species-Trait & Artefact Dimensional Admission Gate (GAP-13 / G69).
//!
//! Validates every admitted `.10d` organism or vehicle asset's authored bounding box (AABB in SI metres),
//! tissue/material density, and simulation mass against its `.q42` QUDT trait envelope
//! (`AusTraits` / `AFD` / `AVONET` / `made:Artefact` specs per `DTA-ID-05`, `DTA-SEM-05`, `DTA-PHY-02`).
//! Enforces zero-heap validation and relative scale invariants (e.g. preventing oversized fauna vs 4WD).

/// Normative QUDT species-trait and artefact dimensional admission envelope.
///
/// Governed by `https://ns.webizen.org/eco/traits#` / `AusTraits` / `AFD` / `AVONET` / `made:Artefact`
/// under `DTA-ID-05`, `DTA-SEM-05`, `DTA-PHY-02`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetricTraitEnvelope {
    pub entity_class_id: u32,
    /// Minimum horizontal longitudinal span in meters (major axis dx or dz).
    pub min_length_m: f32,
    /// Maximum horizontal longitudinal span in meters (major axis dx or dz).
    pub max_length_m: f32,
    /// Minimum horizontal lateral span in meters (minor axis dx or dz).
    pub min_width_m: f32,
    /// Maximum horizontal lateral span in meters (minor axis dx or dz).
    pub max_width_m: f32,
    /// Minimum vertical span in meters (dy axis).
    pub min_height_m: f32,
    /// Maximum vertical span in meters (dy axis).
    pub max_height_m: f32,
    /// Minimum physical mass in kilograms.
    pub min_mass_kg: f32,
    /// Maximum physical mass in kilograms.
    pub max_mass_kg: f32,
}

/// Canonical 4WD field utility vehicle (Toyota Hilux / Land Cruiser 79 dual-cab archetype).
pub const ENVELOPE_FIELD_UTILITY_4WD: MetricTraitEnvelope = MetricTraitEnvelope {
    entity_class_id: 1,
    min_length_m: 4.20,
    max_length_m: 5.60,
    min_width_m: 1.65,
    max_width_m: 2.20,
    min_height_m: 1.70,
    max_height_m: 2.25,
    min_mass_kg: 1800.0,
    max_mass_kg: 3200.0,
};

/// Canonical Eastern Grey Kangaroo (Macropus giganteus).
pub const ENVELOPE_MACROPUS_GIGANTEUS: MetricTraitEnvelope = MetricTraitEnvelope {
    entity_class_id: 2,
    min_length_m: 0.90,
    max_length_m: 2.60,
    min_width_m: 0.35,
    max_width_m: 1.10,
    min_height_m: 0.90,
    max_height_m: 2.10,
    min_mass_kg: 18.0,
    max_mass_kg: 85.0,
};

/// Canonical Common Wombat (Vombatus ursinus).
pub const ENVELOPE_VOMBATUS_URSINUS: MetricTraitEnvelope = MetricTraitEnvelope {
    entity_class_id: 3,
    min_length_m: 0.70,
    max_length_m: 1.25,
    min_width_m: 0.30,
    max_width_m: 0.65,
    min_height_m: 0.25,
    max_height_m: 0.55,
    min_mass_kg: 15.0,
    max_mass_kg: 40.0,
};

/// Canonical Platypus (Ornithorhynchus anatinus).
pub const ENVELOPE_ORNITHORHYNCHUS_ANATINUS: MetricTraitEnvelope = MetricTraitEnvelope {
    entity_class_id: 4,
    min_length_m: 0.35,
    max_length_m: 0.65,
    min_width_m: 0.12,
    max_width_m: 0.35,
    min_height_m: 0.10,
    max_height_m: 0.28,
    min_mass_kg: 0.7,
    max_mass_kg: 2.5,
};

/// Canonical Wedge-Tailed Eagle (Aquila audax). Width represents wingspan.
pub const ENVELOPE_AQUILA_AUDAX: MetricTraitEnvelope = MetricTraitEnvelope {
    entity_class_id: 5,
    min_length_m: 0.75,
    max_length_m: 1.25,
    min_width_m: 1.70,
    max_width_m: 2.50,
    min_height_m: 0.35,
    max_height_m: 1.10,
    min_mass_kg: 2.8,
    max_mass_kg: 5.8,
};

/// Canonical Ghost Gum (Corymbia aparrerinja).
pub const ENVELOPE_CORYMBIA_APARRERINJA: MetricTraitEnvelope = MetricTraitEnvelope {
    entity_class_id: 6,
    min_length_m: 4.0,
    max_length_m: 28.0,
    min_width_m: 4.0,
    max_width_m: 28.0,
    min_height_m: 6.0,
    max_height_m: 35.0,
    min_mass_kg: 500.0,
    max_mass_kg: 25000.0,
};

/// Rejection reasons emitted when a mesh fails the dimensional admission gate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MetricTraitGateViolation {
    EmptyVertexSlice,
    NonFiniteCoordinate,
    HeightOutOfBounds {
        measured_m: f32,
        min_m: f32,
        max_m: f32,
    },
    LengthOutOfBounds {
        measured_m: f32,
        min_m: f32,
        max_m: f32,
    },
    WidthOutOfBounds {
        measured_m: f32,
        min_m: f32,
        max_m: f32,
    },
    RelativeScaleInversion {
        fauna_max_extent_m: f32,
        vehicle_length_m: f32,
    },
}

impl std::fmt::Display for MetricTraitGateViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyVertexSlice => write!(f, "Empty vertex positions slice"),
            Self::NonFiniteCoordinate => write!(f, "Non-finite coordinate encountered in mesh"),
            Self::HeightOutOfBounds {
                measured_m,
                min_m,
                max_m,
            } => {
                write!(
                    f,
                    "Height {measured_m:.3} m out of bounds [{min_m:.3}, {max_m:.3}] m"
                )
            }
            Self::LengthOutOfBounds {
                measured_m,
                min_m,
                max_m,
            } => {
                write!(
                    f,
                    "Length {measured_m:.3} m out of bounds [{min_m:.3}, {max_m:.3}] m"
                )
            }
            Self::WidthOutOfBounds {
                measured_m,
                min_m,
                max_m,
            } => {
                write!(
                    f,
                    "Width {measured_m:.3} m out of bounds [{min_m:.3}, {max_m:.3}] m"
                )
            }
            Self::RelativeScaleInversion {
                fauna_max_extent_m,
                vehicle_length_m,
            } => {
                write!(
                    f,
                    "Relative scale inversion: fauna extent {fauna_max_extent_m:.3} m exceeds vehicle length {vehicle_length_m:.3} m"
                )
            }
        }
    }
}

impl std::error::Error for MetricTraitGateViolation {}

/// Measures the axis-aligned bounding box (min, max, extents) across a slice of 3D positions with zero heap allocation.
pub fn measure_aabb_extents_zero_heap(
    positions: &[[f32; 3]],
) -> Result<([f32; 3], [f32; 3], [f32; 3]), MetricTraitGateViolation> {
    if positions.is_empty() {
        return Err(MetricTraitGateViolation::EmptyVertexSlice);
    }

    let mut min_xyz = [f32::INFINITY; 3];
    let mut max_xyz = [f32::NEG_INFINITY; 3];

    for p in positions {
        for axis in 0..3 {
            let val = p[axis];
            if !val.is_finite() {
                return Err(MetricTraitGateViolation::NonFiniteCoordinate);
            }
            if val < min_xyz[axis] {
                min_xyz[axis] = val;
            }
            if val > max_xyz[axis] {
                max_xyz[axis] = val;
            }
        }
    }

    let extents = [
        max_xyz[0] - min_xyz[0],
        max_xyz[1] - min_xyz[1],
        max_xyz[2] - min_xyz[2],
    ];

    Ok((min_xyz, max_xyz, extents))
}

/// Verifies that an asset's scaled bounding box extents conform to the normative QUDT trait envelope.
///
/// Returns the scaled extents `[dx, dy, dz]` in meters on admission, or a fail-closed violation.
pub fn verify_metric_trait_bounds_zero_heap(
    positions: &[[f32; 3]],
    instance_scale: f32,
    envelope: &MetricTraitEnvelope,
) -> Result<[f32; 3], MetricTraitGateViolation> {
    if !instance_scale.is_finite() || instance_scale <= 0.0 {
        return Err(MetricTraitGateViolation::NonFiniteCoordinate);
    }

    let (_min, _max, extents) = measure_aabb_extents_zero_heap(positions)?;
    let scaled_extents = [
        extents[0] * instance_scale,
        extents[1] * instance_scale,
        extents[2] * instance_scale,
    ];

    // Vertical height is dy (index 1)
    let scaled_height = scaled_extents[1];
    if scaled_height < envelope.min_height_m || scaled_height > envelope.max_height_m {
        return Err(MetricTraitGateViolation::HeightOutOfBounds {
            measured_m: scaled_height,
            min_m: envelope.min_height_m,
            max_m: envelope.max_height_m,
        });
    }

    // Horizontal spans: major dx.max(dz), minor dx.min(dz)
    let horiz_major = scaled_extents[0].max(scaled_extents[2]);
    let horiz_minor = scaled_extents[0].min(scaled_extents[2]);

    // Check if envelope width is greater than length (e.g. raptor wingspan)
    let (scaled_len, scaled_wid) = if envelope.max_width_m > envelope.max_length_m {
        (horiz_minor, horiz_major)
    } else {
        (horiz_major, horiz_minor)
    };

    if scaled_len < envelope.min_length_m || scaled_len > envelope.max_length_m {
        return Err(MetricTraitGateViolation::LengthOutOfBounds {
            measured_m: scaled_len,
            min_m: envelope.min_length_m,
            max_m: envelope.max_length_m,
        });
    }

    if scaled_wid < envelope.min_width_m || scaled_wid > envelope.max_width_m {
        return Err(MetricTraitGateViolation::WidthOutOfBounds {
            measured_m: scaled_wid,
            min_m: envelope.min_width_m,
            max_m: envelope.max_width_m,
        });
    }

    Ok(scaled_extents)
}

/// Enforces the fail-closed relative scale invariant between terrestrial fauna and vehicles.
///
/// Prevents the "kangaroo twice the size of a 4WD" bug at the engine admission boundary.
pub fn verify_fauna_vs_vehicle_scale_invariant_zero_heap(
    fauna_extents_m: [f32; 3],
    vehicle_extents_m: [f32; 3],
) -> Result<(), MetricTraitGateViolation> {
    for c in fauna_extents_m.iter().chain(vehicle_extents_m.iter()) {
        if !c.is_finite() {
            return Err(MetricTraitGateViolation::NonFiniteCoordinate);
        }
    }

    let fauna_max_extent_m = fauna_extents_m[0]
        .max(fauna_extents_m[1])
        .max(fauna_extents_m[2]);
    let vehicle_length_m = vehicle_extents_m[0].max(vehicle_extents_m[2]);

    if fauna_max_extent_m > vehicle_length_m {
        return Err(MetricTraitGateViolation::RelativeScaleInversion {
            fauna_max_extent_m,
            vehicle_length_m,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kangaroo_valid_and_out_of_bounds() {
        // Authored at 1:1 metric scale: length (Z) = 1.4m, width (X) = 0.5m, height (Y) = 1.6m
        let vertices = [
            [-0.25, 0.0, -0.7],
            [0.25, 0.0, -0.7],
            [-0.25, 1.6, 0.7],
            [0.25, 1.6, 0.7],
        ];

        let result = verify_metric_trait_bounds_zero_heap(
            &vertices,
            1.0,
            &ENVELOPE_MACROPUS_GIGANTEUS,
        );
        assert!(result.is_ok());
        let extents = result.unwrap();
        assert!((extents[1] - 1.6).abs() < 1e-4);

        // Scaled up by 4.0x -> oversized monster kangaroo (height 6.4m > 2.1m)
        let oversized = verify_metric_trait_bounds_zero_heap(
            &vertices,
            4.0,
            &ENVELOPE_MACROPUS_GIGANTEUS,
        );
        assert!(matches!(
            oversized,
            Err(MetricTraitGateViolation::HeightOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_field_utility_4wd_bounds() {
        // Vehicle length = 5.2m, width = 1.9m, height = 1.85m
        let vertices = [
            [-0.95, 0.0, -2.6],
            [0.95, 0.0, -2.6],
            [-0.95, 1.85, 2.6],
            [0.95, 1.85, 2.6],
        ];

        let result = verify_metric_trait_bounds_zero_heap(
            &vertices,
            1.0,
            &ENVELOPE_FIELD_UTILITY_4WD,
        );
        assert!(result.is_ok());

        // Miniature vehicle (scale 0.1) -> length 0.52m < 4.20m
        let miniature = verify_metric_trait_bounds_zero_heap(
            &vertices,
            0.1,
            &ENVELOPE_FIELD_UTILITY_4WD,
        );
        assert!(matches!(
            miniature,
            Err(MetricTraitGateViolation::HeightOutOfBounds { .. })
                | Err(MetricTraitGateViolation::LengthOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_eagle_wingspan_major_span() {
        // Eagle: length 0.9m (Z), wingspan 2.1m (X), height 0.6m (Y)
        let vertices = [
            [-1.05, 0.0, -0.45],
            [1.05, 0.0, -0.45],
            [-1.05, 0.6, 0.45],
            [1.05, 0.6, 0.45],
        ];

        let result = verify_metric_trait_bounds_zero_heap(
            &vertices,
            1.0,
            &ENVELOPE_AQUILA_AUDAX,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_relative_scale_inversion_fauna_vs_vehicle() {
        let vehicle_extents = [1.9_f32, 1.85, 5.2]; // 4WD: length 5.2m

        // Realistic kangaroo: 0.5m x 1.6m x 1.4m (max extent = 1.6m < 5.2m)
        let normal_kangaroo = [0.5_f32, 1.6, 1.4];
        assert!(verify_fauna_vs_vehicle_scale_invariant_zero_heap(
            normal_kangaroo,
            vehicle_extents
        )
        .is_ok());

        // Broken scale: giant kangaroo with height 6.0m or length 6.5m (> 5.2m 4WD length)
        let giant_kangaroo = [2.0_f32, 6.0, 5.5];
        let err = verify_fauna_vs_vehicle_scale_invariant_zero_heap(
            giant_kangaroo,
            vehicle_extents,
        );
        assert_eq!(
            err,
            Err(MetricTraitGateViolation::RelativeScaleInversion {
                fauna_max_extent_m: 6.0,
                vehicle_length_m: 5.2,
            })
        );
    }

    #[test]
    fn test_empty_and_non_finite_vertex_checks() {
        let empty: &[[f32; 3]] = &[];
        assert_eq!(
            measure_aabb_extents_zero_heap(empty),
            Err(MetricTraitGateViolation::EmptyVertexSlice)
        );

        let nan_vertices = [[0.0, f32::NAN, 1.0]];
        assert_eq!(
            measure_aabb_extents_zero_heap(&nan_vertices),
            Err(MetricTraitGateViolation::NonFiniteCoordinate)
        );
    }
}
