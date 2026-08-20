//! Celestial body taxonomy and profile types (OCS §2).
//!
//! Reference: OCS Specification v2.2.0 §2.

use crate::value::Value;
use std::collections::BTreeMap;

/// Celestial body classification (OCS §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CelestialBodyClass {
    TerrestrialPlanet,
    GasGiant,
    IceGiant,
    DwarfPlanet,
    NaturalSatellite,
    MinorBodyIrregular,
    Star,
    CompactRelativistic,
    BlackHole,
    Megastructure,
    FictionalWorld,
}

impl CelestialBodyClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TerrestrialPlanet => "TerrestrialPlanet",
            Self::GasGiant => "GasGiant",
            Self::IceGiant => "IceGiant",
            Self::DwarfPlanet => "DwarfPlanet",
            Self::NaturalSatellite => "NaturalSatellite",
            Self::MinorBodyIrregular => "MinorBodyIrregular",
            Self::Star => "Star",
            Self::CompactRelativistic => "CompactRelativistic",
            Self::BlackHole => "BlackHole",
            Self::Megastructure => "Megastructure",
            Self::FictionalWorld => "FictionalWorld",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "TerrestrialPlanet" => Some(Self::TerrestrialPlanet),
            "GasGiant" => Some(Self::GasGiant),
            "IceGiant" => Some(Self::IceGiant),
            "DwarfPlanet" => Some(Self::DwarfPlanet),
            "NaturalSatellite" => Some(Self::NaturalSatellite),
            "MinorBodyIrregular" => Some(Self::MinorBodyIrregular),
            "Star" => Some(Self::Star),
            "CompactRelativistic" => Some(Self::CompactRelativistic),
            "BlackHole" => Some(Self::BlackHole),
            "Megastructure" => Some(Self::Megastructure),
            "FictionalWorld" => Some(Self::FictionalWorld),
            _ => None,
        }
    }

    /// Whether this body type has a solid surface.
    pub fn has_solid_surface(&self) -> bool {
        matches!(
            self,
            Self::TerrestrialPlanet
                | Self::DwarfPlanet
                | Self::NaturalSatellite
                | Self::MinorBodyIrregular
                | Self::Megastructure
                | Self::FictionalWorld
        )
    }

    /// Whether this body type requires relativistic metric handling.
    pub fn is_relativistic(&self) -> bool {
        matches!(self, Self::CompactRelativistic | Self::BlackHole)
    }
}

/// Reference ellipsoid type (OCS §2.2).
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceSurface {
    /// Biaxial ellipsoid (a = b ≠ c) for rotationally flattened bodies.
    Biaxial { a: f64, c: f64 },
    /// Triaxial ellipsoid (a > b > c) for non-axisymmetric bodies.
    Triaxial { a: f64, b: f64, c: f64 },
    /// Equipotential surface (geoid, areoid, 1-bar level).
    Equipotential { name: String },
    /// 3D polygonal mesh for irregular bodies.
    Mesh { name: String },
}

/// Gravitational field model (OCS §2.3).
#[derive(Debug, Clone, PartialEq)]
pub struct GravitationalField {
    /// Standard gravitational parameter μ = GM (m³/s²).
    pub mu: f64,
    /// Reference radius for harmonic expansion (m).
    pub r_ref: f64,
    /// Zonal harmonic coefficients J₂, J₃, ...
    pub jn: Vec<f64>,
    /// Schwarzschild radius r_s = 2GM/c² (m). Zero for non-relativistic bodies.
    pub schwarzschild_r: f64,
    /// Kerr spin parameter a = J/(Mc) (m). Zero for non-spinning bodies.
    pub kerr_spin_a: f64,
}

impl GravitationalField {
    /// Create a simple Newtonian field (no relativistic terms).
    pub fn newtonian(mu: f64, r_ref: f64) -> Self {
        Self {
            mu,
            r_ref,
            jn: Vec::new(),
            schwarzschild_r: 0.0,
            kerr_spin_a: 0.0,
        }
    }

    /// Kerr event horizon radius r₊ (OCS §2.3).
    /// r₊ = r_s/2 + sqrt((r_s/2)² - a²)
    pub fn kerr_horizon_r(&self) -> f64 {
        if self.schwarzschild_r == 0.0 {
            return 0.0;
        }
        let half_rs = self.schwarzschild_r / 2.0;
        let disc = half_rs * half_rs - self.kerr_spin_a * self.kerr_spin_a;
        if disc < 0.0 {
            return 0.0; // No horizon (naked singularity — unphysical)
        }
        half_rs + disc.sqrt()
    }

    /// Kerr ergosphere boundary r_E(θ) at θ = π/2 (equator) (OCS §2.3).
    /// r_E(θ) = r_s/2 + sqrt((r_s/2)² - a²cos²θ)
    pub fn kerr_ergosphere_r(&self, theta: f64) -> f64 {
        if self.schwarzschild_r == 0.0 {
            return 0.0;
        }
        let half_rs = self.schwarzschild_r / 2.0;
        let cos_theta = theta.cos();
        let disc = half_rs * half_rs - self.kerr_spin_a * self.kerr_spin_a * cos_theta * cos_theta;
        if disc < 0.0 {
            return 0.0;
        }
        half_rs + disc.sqrt()
    }

    /// Frame-dragging precession frequency Ω_LT (OCS §2.3).
    /// Simplified: Ω_LT ≈ 2GJ/(c²r³) for far-field.
    pub fn frame_dragging(&self, r: f64) -> f64 {
        if self.schwarzschild_r == 0.0 || self.kerr_spin_a == 0.0 {
            return 0.0;
        }
        // J = M*c*a, so GJ = G*M*c*a = (μ/c)*c*a = μ*a
        // Ω_LT = 2*G*J / (c² * r³) = 2*μ*a / (c² * r³)
        let c = 299_792_458.0;
        let c2 = c * c;
        2.0 * self.mu * self.kerr_spin_a / (c2 * r * r * r)
    }
}

/// A celestial body profile (OCS §2).
#[derive(Debug, Clone)]
pub struct CelestialBodyProfile {
    pub name: String,
    pub class: CelestialBodyClass,
    pub usri: String,
    pub mass_kg: f64,
    pub equatorial_radius_m: f64,
    pub rotation_period_s: f64,
    pub surface: ReferenceSurface,
    pub gravity: GravitationalField,
}

impl CelestialBodyProfile {
    /// Surface gravity g = GM/R² (m/s²).
    pub fn surface_gravity(&self) -> f64 {
        self.gravity.mu / (self.equatorial_radius_m * self.equatorial_radius_m)
    }

    /// Convert to Value::Record.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("class".into(), Value::String(self.class.as_str().into()));
        rec.insert("usri".into(), Value::String(self.usri.clone()));
        rec.insert("mass_kg".into(), Value::F64(self.mass_kg));
        rec.insert(
            "equatorial_radius_m".into(),
            Value::F64(self.equatorial_radius_m),
        );
        rec.insert(
            "rotation_period_s".into(),
            Value::F64(self.rotation_period_s),
        );
        rec.insert("surface_gravity".into(), Value::F64(self.surface_gravity()));
        Value::Record(rec)
    }
}

/// Known celestial body profiles (OCS §2 examples).
pub fn earth_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Earth".into(),
        class: CelestialBodyClass::TerrestrialPlanet,
        usri: "urn:omni:v1:physical:observable:standard:sol:earth".into(),
        mass_kg: 5.972e24,
        equatorial_radius_m: 6_378_137.0,
        rotation_period_s: 86_164.0905, // sidereal day
        surface: ReferenceSurface::Biaxial {
            a: 6_378_137.0,
            c: 6_356_752.3,
        },
        gravity: {
            let mut g = GravitationalField::newtonian(3.986004418e14, 6_378_137.0);
            g.jn = vec![-1.08263e-3, 2.57e-6]; // J2, J3
            g
        },
    }
}

pub fn mars_profile() -> CelestialBodyProfile {
    CelestialBodyProfile {
        name: "Mars".into(),
        class: CelestialBodyClass::TerrestrialPlanet,
        usri: "urn:omni:v1:physical:observable:standard:sol:mars".into(),
        mass_kg: 6.4171e23,
        equatorial_radius_m: 3_396_200.0,
        rotation_period_s: 88_642.66, // Martian sol
        surface: ReferenceSurface::Biaxial {
            a: 3_396_200.0,
            c: 3_376_200.0,
        },
        gravity: {
            let mut g = GravitationalField::newtonian(4.282837e13, 3_396_200.0);
            g.jn = vec![-1.96045e-3];
            g
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_surface_gravity() {
        let earth = earth_profile();
        let g = earth.surface_gravity();
        // Should be ~9.81 m/s²
        assert!((g - 9.81).abs() < 0.1, "got {} expected ~9.81", g);
    }

    #[test]
    fn mars_surface_gravity() {
        let mars = mars_profile();
        let g = mars.surface_gravity();
        // Should be ~3.71 m/s²
        assert!((g - 3.71).abs() < 0.1, "got {} expected ~3.71", g);
    }

    #[test]
    fn class_has_solid_surface() {
        assert!(CelestialBodyClass::TerrestrialPlanet.has_solid_surface());
        assert!(!CelestialBodyClass::GasGiant.has_solid_surface());
        assert!(CelestialBodyClass::Megastructure.has_solid_surface());
    }

    #[test]
    fn class_is_relativistic() {
        assert!(CelestialBodyClass::BlackHole.is_relativistic());
        assert!(CelestialBodyClass::CompactRelativistic.is_relativistic());
        assert!(!CelestialBodyClass::Star.is_relativistic());
    }

    #[test]
    fn class_round_trip() {
        for c in [
            CelestialBodyClass::TerrestrialPlanet,
            CelestialBodyClass::GasGiant,
            CelestialBodyClass::BlackHole,
            CelestialBodyClass::FictionalWorld,
        ] {
            assert_eq!(CelestialBodyClass::from_str(c.as_str()), Some(c));
        }
    }

    #[test]
    fn kerr_horizon_schwarzschild() {
        // Non-spinning black hole: a=0, horizon = r_s/2 + sqrt((r_s/2)²) = r_s
        let g = GravitationalField {
            mu: 1.0,
            r_ref: 1.0,
            jn: vec![],
            schwarzschild_r: 100.0,
            kerr_spin_a: 0.0,
        };
        // r₊ = 50 + sqrt(2500 - 0) = 50 + 50 = 100 = r_s
        assert!((g.kerr_horizon_r() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn kerr_horizon_spinning() {
        // Spinning: a=30, r_s=100 → r+ = 50 + sqrt(2500-900) = 50+40 = 90
        let g = GravitationalField {
            mu: 1.0,
            r_ref: 1.0,
            jn: vec![],
            schwarzschild_r: 100.0,
            kerr_spin_a: 30.0,
        };
        assert!((g.kerr_horizon_r() - 90.0).abs() < 1e-10);
    }

    #[test]
    fn kerr_ergosphere_at_equator() {
        // At equator θ=π/2, cos(θ)=0, so ergosphere = r_s/2 + sqrt((r_s/2)² - 0) = r_s
        let g = GravitationalField {
            mu: 1.0,
            r_ref: 1.0,
            jn: vec![],
            schwarzschild_r: 100.0,
            kerr_spin_a: 30.0,
        };
        let r_eq = g.kerr_ergosphere_r(std::f64::consts::PI / 2.0);
        // r_E = 50 + sqrt(2500 - 0) = 50 + 50 = 100 = r_s
        assert!((r_eq - 100.0).abs() < 1e-10);
    }

    #[test]
    fn kerr_ergosphere_at_pole() {
        // At pole θ=0, cos(θ)=1, so ergosphere = r_s/2 + sqrt((r_s/2)²-a²)
        // = 50 + sqrt(2500-900) = 90 (same as horizon for this case)
        // Actually at pole: r_E = r_s/2 + sqrt((r_s/2)² - a²) = 50+40 = 90
        let g = GravitationalField {
            mu: 1.0,
            r_ref: 1.0,
            jn: vec![],
            schwarzschild_r: 100.0,
            kerr_spin_a: 30.0,
        };
        let r_pole = g.kerr_ergosphere_r(0.0);
        assert!((r_pole - 90.0).abs() < 1e-10);
    }

    #[test]
    fn frame_dragging_nonzero() {
        let g = GravitationalField {
            mu: 3.986e14,
            r_ref: 6_378_137.0,
            jn: vec![],
            schwarzschild_r: 0.00887, // Earth's Schwarzschild radius ~8.87mm
            kerr_spin_a: 1.0,         // Small spin for testing
        };
        let omega = g.frame_dragging(6_800_000.0);
        // Should be very small but nonzero
        assert!(omega > 0.0);
        assert!(omega < 1e-6);
    }

    #[test]
    fn profile_to_value() {
        let earth = earth_profile();
        let v = earth.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("name"), Some(&Value::String("Earth".into())));
                assert!(r.contains_key("surface_gravity"));
            }
            _ => panic!("expected Record"),
        }
    }
}
