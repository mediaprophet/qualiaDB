//! Ambient Fields — spatial-physical field declarations and samplers.
//!
//! Represents physical quantities defined over manifold space (pressure, temperature,
//! EM / IOR medium, acoustic impedance, chemical species) without consuming 10D tensor axes.
//!
//! Reference: `docs/plans/vibe-design/20260819_fields-materials-and-creator-physics.md` §2.1.

use crate::ast::{FieldRepresentation, FieldSupport};

/// Physical quantity kind of an ambient field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldQuantity {
    Pressure,
    Temperature,
    GravityPotential,
    OpticalMedium,
    AcousticImpedance,
    ChemicalSpecies(String),
    Custom(String),
}

impl FieldQuantity {
    pub fn as_iri(&self) -> &str {
        match self {
            Self::Pressure => "http://qudt.org/vocab/quantitykind/Pressure",
            Self::Temperature => "http://qudt.org/vocab/quantitykind/ThermodynamicTemperature",
            Self::GravityPotential => "http://qudt.org/vocab/quantitykind/GravitationalPotential",
            Self::OpticalMedium => "https://qualiadb.org/schema/field/OpticalRefractiveIndex",
            Self::AcousticImpedance => "http://qudt.org/vocab/quantitykind/AcousticImpedance",
            Self::ChemicalSpecies(s) => s.as_str(),
            Self::Custom(c) => c.as_str(),
        }
    }
}

/// Standardized unit of measurement for field samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldUnit {
    KiloPascal,
    Pascal,
    Kelvin,
    Celsius,
    MetresPerSecondSquared,
    JoulesPerKilogram,
    Dimensionless,
    PascalSecondPerMetre,
    MolePerCubicMetre,
    Custom(String),
}

impl FieldUnit {
    pub fn as_iri(&self) -> &str {
        match self {
            Self::KiloPascal => "http://qudt.org/vocab/unit/KiloPA",
            Self::Pascal => "http://qudt.org/vocab/unit/PA",
            Self::Kelvin => "http://qudt.org/vocab/unit/K",
            Self::Celsius => "http://qudt.org/vocab/unit/DEG_C",
            Self::MetresPerSecondSquared => "http://qudt.org/vocab/unit/M-PER-SEC2",
            Self::JoulesPerKilogram => "http://qudt.org/vocab/unit/J-PER-KILO",
            Self::Dimensionless => "http://qudt.org/vocab/unit/UNITLESS",
            Self::PascalSecondPerMetre => "http://qudt.org/vocab/unit/PA-SEC-PER-M",
            Self::MolePerCubicMetre => "http://qudt.org/vocab/unit/MOL-PER-M3",
            Self::Custom(s) => s.as_str(),
        }
    }
}

/// Analytic field model parameterization.
#[derive(Debug, Clone, PartialEq)]
pub enum AnalyticFieldProfile {
    /// Uniform scalar value throughout space.
    Uniform(f64),
    /// Barometric exponential gradient along Z: P(z) = P0 * exp(-z / scale_height).
    BarometricGradient { p0: f64, scale_height: f64 },
    /// Radial inverse-square point source (e.g. thermal / acoustic source at origin).
    RadialSource {
        origin: [f64; 3],
        intensity: f64,
        falloff: f64,
    },
    /// Linear directional gradient: V(p) = base + dot(p, direction) * slope.
    LinearGradient {
        base: f64,
        direction: [f64; 3],
        slope: f64,
    },
}

/// Ambient Field Declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDeclaration {
    /// Unique Field IRI (e.g. `did:q42:field:ambient_pressure`).
    pub id: String,
    /// Physical quantity kind.
    pub quantity: FieldQuantity,
    /// Standardized unit.
    pub unit: FieldUnit,
    /// Spatial distribution support.
    pub support: FieldSupport,
    /// Underlying representation.
    pub representation: FieldRepresentation,
    /// Analytical profile when representation is Analytic.
    pub profile: AnalyticFieldProfile,
}

impl FieldDeclaration {
    /// Create standard atmospheric sea-level pressure field (101.325 kPa at z=0, scale height 8500m).
    pub fn standard_atmosphere() -> Self {
        Self {
            id: "did:q42:field:standard_atmosphere".to_string(),
            quantity: FieldQuantity::Pressure,
            unit: FieldUnit::KiloPascal,
            support: FieldSupport::Region,
            representation: FieldRepresentation::Analytic,
            profile: AnalyticFieldProfile::BarometricGradient {
                p0: 101.325,
                scale_height: 8500.0,
            },
        }
    }

    /// Create room-temperature thermal field (293.15 K uniform).
    pub fn ambient_room_temperature() -> Self {
        Self {
            id: "did:q42:field:room_temperature".to_string(),
            quantity: FieldQuantity::Temperature,
            unit: FieldUnit::Kelvin,
            support: FieldSupport::Region,
            representation: FieldRepresentation::Analytic,
            profile: AnalyticFieldProfile::Uniform(293.15),
        }
    }

    /// Create a water solvent presence field (H2O concentration mol/m³).
    pub fn water_solvent(concentration: f64) -> Self {
        Self {
            id: "did:q42:field:aqueous_medium".to_string(),
            quantity: FieldQuantity::ChemicalSpecies("did:q42:species:H2O".to_string()),
            unit: FieldUnit::MolePerCubicMetre,
            support: FieldSupport::Region,
            representation: FieldRepresentation::Analytic,
            profile: AnalyticFieldProfile::Uniform(concentration),
        }
    }

    /// Sample the field value at a 3D position [x, y, z].
    #[inline]
    pub fn sample_at(&self, position: &[f64]) -> f64 {
        let x = position.first().copied().unwrap_or(0.0);
        let y = position.get(1).copied().unwrap_or(0.0);
        let z = position.get(2).copied().unwrap_or(0.0);

        match &self.profile {
            AnalyticFieldProfile::Uniform(v) => *v,
            AnalyticFieldProfile::BarometricGradient { p0, scale_height } => {
                if *scale_height > 0.0 {
                    p0 * (-z / scale_height).exp()
                } else {
                    *p0
                }
            }
            AnalyticFieldProfile::RadialSource {
                origin,
                intensity,
                falloff,
            } => {
                let dx = x - origin[0];
                let dy = y - origin[1];
                let dz = z - origin[2];
                let r_sq = dx * dx + dy * dy + dz * dz;
                intensity / (1.0 + falloff * r_sq)
            }
            AnalyticFieldProfile::LinearGradient {
                base,
                direction,
                slope,
            } => {
                let proj = x * direction[0] + y * direction[1] + z * direction[2];
                base + proj * slope
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barometric_pressure_sampling() {
        let atm = FieldDeclaration::standard_atmosphere();
        let sea_level = atm.sample_at(&[0.0, 0.0, 0.0]);
        assert!((sea_level - 101.325).abs() < 1e-4);

        let summit = atm.sample_at(&[0.0, 0.0, 8500.0]);
        // At 1 scale height, P = P0 * e^(-1) ≈ 101.325 * 0.367879 = 37.275 kPa
        assert!((summit - (101.325 * (-1.0f64).exp())).abs() < 1e-3);
    }

    #[test]
    fn test_radial_source_sampling() {
        let thermal = FieldDeclaration {
            id: "did:q42:field:heat_source".to_string(),
            quantity: FieldQuantity::Temperature,
            unit: FieldUnit::Kelvin,
            support: FieldSupport::Point,
            representation: FieldRepresentation::Analytic,
            profile: AnalyticFieldProfile::RadialSource {
                origin: [0.0, 0.0, 0.0],
                intensity: 1000.0,
                falloff: 0.1,
            },
        };

        let at_origin = thermal.sample_at(&[0.0, 0.0, 0.0]);
        assert_eq!(at_origin, 1000.0);

        let at_dist = thermal.sample_at(&[3.0, 0.0, 0.0]); // r^2 = 9
        assert!((at_dist - (1000.0 / (1.0 + 0.9))).abs() < 1e-3);
    }
}
