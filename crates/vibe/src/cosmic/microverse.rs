//! Subatomic microverses and scaling lens (OCS §12.4).
//!
//! The OCS supports "microverse" realm classes — self-contained
//! realities at subatomic scales, useful for quantum simulations,
//! particle physics visualizations, and fictional narratives that
//! operate at microscopic scales.
//!
//! Reference: OCS Specification v2.2.0 §12.4.

use crate::cosmic::cb_usri::HierarchyLevel;
use crate::cosmic::usri::RealmClass;
use crate::value::Value;
use std::collections::BTreeMap;

/// Fundamental physical constants used for scaling (OCS §12.4).
pub mod constants {
    /// Planck length in meters
    pub const PLANCK_LENGTH_M: f64 = 1.616255e-35;
    /// Planck time in seconds
    pub const PLANCK_TIME_S: f64 = 5.391247e-44;
    /// Planck mass in kg
    pub const PLANCK_MASS_KG: f64 = 2.176434e-8;
    /// Planck energy in Joules
    pub const PLANCK_ENERGY_J: f64 = 1.9561e9;
    /// Planck temperature in Kelvin
    pub const PLANCK_TEMP_K: f64 = 1.416784e32;
    /// Reduced Planck constant ℏ in J·s
    pub const HBAR_J_S: f64 = 1.054571817e-34;
    /// Boltzmann constant in J/K
    pub const BOLTZMANN_J_K: f64 = 1.380649e-23;
    /// Speed of light in m/s
    pub const C_M_S: f64 = 299_792_458.0;
    /// Electron mass in kg
    pub const ELECTRON_MASS_KG: f64 = 9.1093837015e-31;
    /// Proton mass in kg
    pub const PROTON_MASS_KG: f64 = 1.67262192369e-27;
    /// Bohr radius in meters
    pub const BOHR_RADIUS_M: f64 = 5.29177210903e-11;
    /// Compton wavelength of electron in meters
    pub const ELECTRON_COMPTON_M: f64 = 2.42631023867e-12;
    /// Fine-structure constant (dimensionless)
    pub const FINE_STRUCTURE: f64 = 7.2973525693e-3;
}

/// A scaling lens — maps between hierarchy levels (OCS §12.4).
///
/// The OCS spans 61 orders of magnitude. The scaling lens provides
/// conversion factors between adjacent levels, enabling a "zoom"
/// operation that preserves physical relationships.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScalingLens {
    /// Source hierarchy level
    pub from_level: HierarchyLevel,
    /// Target hierarchy level
    pub to_level: HierarchyLevel,
    /// Linear scale factor (from → to)
    pub scale_factor: f64,
}

impl ScalingLens {
    /// Create a scaling lens between two hierarchy levels (OCS §12.4).
    pub fn between(from: HierarchyLevel, to: HierarchyLevel) -> Self {
        let scale = scale_factor_between(from, to);
        Self {
            from_level: from,
            to_level: to,
            scale_factor: scale,
        }
    }

    /// Transform a length from the source level to the target level.
    pub fn transform_length(&self, length: f64) -> f64 {
        length * self.scale_factor
    }

    /// Transform a length back from target to source.
    pub fn inverse_length(&self, length: f64) -> f64 {
        length / self.scale_factor
    }
}

/// Compute the scale factor between two hierarchy levels (OCS §12.4).
///
/// Returns the ratio: 1 unit at `from` level = X units at `to` level.
/// Based on typical length scales at each level.
pub fn scale_factor_between(from: HierarchyLevel, to: HierarchyLevel) -> f64 {
    typical_length(from) / typical_length(to)
}

/// Typical length scale at each hierarchy level (in meters) (OCS §12.4).
pub fn typical_length(level: HierarchyLevel) -> f64 {
    match level {
        HierarchyLevel::LNeg2 => constants::PLANCK_LENGTH_M, // ~1.6e-35 m
        HierarchyLevel::LNeg1 => 1e-34,                      // String scale
        HierarchyLevel::L0 => constants::ELECTRON_COMPTON_M, // ~2.4e-12 m
        HierarchyLevel::L1 => 1e-15,                         // Femtometer (nuclear)
        HierarchyLevel::L2 => constants::BOHR_RADIUS_M,      // ~5.3e-11 m (atomic)
        HierarchyLevel::L3 => 1e-9,                          // Nanometer (macromolecular)
        HierarchyLevel::L4 => 1e-5,                          // 10 μm (cellular)
        HierarchyLevel::L5 => 1e3,                           // 1 km (geodetic)
        HierarchyLevel::L6 => 1e3,                           // 1 km (local AR)
        HierarchyLevel::L7 => 1e11,                          // 100 Gm (planetary system)
        HierarchyLevel::L8 => 1e16,                          // 10 light-years (interstellar)
        HierarchyLevel::L9 => 1e21,                          // 100 kpc (galactic)
        HierarchyLevel::L10 => 1e23,                         // 10 Mpc (cluster)
        HierarchyLevel::L11 => 1e24,                         // 100 Mpc (supercluster)
        HierarchyLevel::L12 => 8.8e26,                       // ~93 Gly (cosmological horizon)
    }
}

/// A subatomic particle classification for microverse realms (OCS §12.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleClass {
    Fermion,
    Boson,
    Quark,
    Lepton,
    Hadron,
    Meson,
    Baryon,
    GaugeBoson,
    Hypothetical,
}

impl ParticleClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fermion => "fermion",
            Self::Boson => "boson",
            Self::Quark => "quark",
            Self::Lepton => "lepton",
            Self::Hadron => "hadron",
            Self::Meson => "meson",
            Self::Baryon => "baryon",
            Self::GaugeBoson => "gauge_boson",
            Self::Hypothetical => "hypothetical",
        }
    }
}

/// A particle profile for microverse simulations (OCS §12.4).
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleProfile {
    pub name: String,
    pub class: ParticleClass,
    pub mass_kg: f64,
    pub charge_e: f64, // In units of elementary charge
    pub spin: f64,     // In units of ℏ
    pub lifetime_s: f64,
    pub usri: String,
}

impl ParticleProfile {
    /// Electron (OCS §12.4).
    pub fn electron() -> Self {
        Self {
            name: "Electron".into(),
            class: ParticleClass::Lepton,
            mass_kg: constants::ELECTRON_MASS_KG,
            charge_e: -1.0,
            spin: 0.5,
            lifetime_s: f64::INFINITY, // Stable
            usri: "urn:omni:v1:physical:observable:standard:particle:electron".into(),
        }
    }

    /// Proton (OCS §12.4).
    pub fn proton() -> Self {
        Self {
            name: "Proton".into(),
            class: ParticleClass::Baryon,
            mass_kg: constants::PROTON_MASS_KG,
            charge_e: 1.0,
            spin: 0.5,
            lifetime_s: f64::INFINITY, // Stable
            usri: "urn:omni:v1:physical:observable:standard:particle:proton".into(),
        }
    }

    /// Neutron (OCS §12.4).
    pub fn neutron() -> Self {
        Self {
            name: "Neutron".into(),
            class: ParticleClass::Baryon,
            mass_kg: 1.67492749804e-27,
            charge_e: 0.0,
            spin: 0.5,
            lifetime_s: 879.4, // Free neutron half-life
            usri: "urn:omni:v1:physical:observable:standard:particle:neutron".into(),
        }
    }

    /// Photon (OCS §12.4).
    pub fn photon() -> Self {
        Self {
            name: "Photon".into(),
            class: ParticleClass::GaugeBoson,
            mass_kg: 0.0,
            charge_e: 0.0,
            spin: 1.0,
            lifetime_s: f64::INFINITY,
            usri: "urn:omni:v1:physical:observable:standard:particle:photon".into(),
        }
    }

    /// Up quark (OCS §12.4).
    pub fn up_quark() -> Self {
        Self {
            name: "Up Quark".into(),
            class: ParticleClass::Quark,
            mass_kg: 4.1e-30, // ~2.2 MeV/c²
            charge_e: 2.0 / 3.0,
            spin: 0.5,
            lifetime_s: 0.0, // Confined, never free
            usri: "urn:omni:v1:physical:observable:standard:particle:quark:up".into(),
        }
    }

    /// Higgs boson (OCS §12.4).
    pub fn higgs_boson() -> Self {
        Self {
            name: "Higgs Boson".into(),
            class: ParticleClass::Boson,
            mass_kg: 2.209e-25, // ~125 GeV/c²
            charge_e: 0.0,
            spin: 0.0,
            lifetime_s: 1.56e-22, // Very short-lived
            usri: "urn:omni:v1:physical:observable:standard:particle:higgs".into(),
        }
    }

    /// Compute the Compton wavelength of a particle (OCS §12.4).
    /// λ = h / (m * c)  where h = 2πℏ
    pub fn compton_wavelength(&self) -> f64 {
        if self.mass_kg == 0.0 {
            return f64::INFINITY; // Massless particles
        }
        let h = 2.0 * std::f64::consts::PI * constants::HBAR_J_S;
        h / (self.mass_kg * constants::C_M_S)
    }

    /// Compute the rest energy in Joules (OCS §12.4).
    /// E = m * c²
    pub fn rest_energy_j(&self) -> f64 {
        self.mass_kg * constants::C_M_S * constants::C_M_S
    }

    /// Compute the de Broglie wavelength at a given velocity (OCS §12.4).
    /// λ = ℏ / (m * v)
    pub fn de_broglie_wavelength(&self, velocity_m_s: f64) -> f64 {
        if velocity_m_s == 0.0 {
            return f64::INFINITY;
        }
        constants::HBAR_J_S / (self.mass_kg * velocity_m_s)
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert("class".into(), Value::String(self.class.as_str().into()));
        rec.insert("mass_kg".into(), Value::F64(self.mass_kg));
        rec.insert("charge_e".into(), Value::F64(self.charge_e));
        rec.insert("spin".into(), Value::F64(self.spin));
        if self.lifetime_s.is_finite() {
            rec.insert("lifetime_s".into(), Value::F64(self.lifetime_s));
        } else {
            rec.insert("lifetime_s".into(), Value::String("stable".into()));
        }
        rec.insert(
            "compton_wavelength_m".into(),
            Value::F64(self.compton_wavelength()),
        );
        rec.insert("rest_energy_j".into(), Value::F64(self.rest_energy_j()));
        Value::Record(rec)
    }
}

/// A microverse realm — a subatomic-scale reality (OCS §12.4).
#[derive(Debug, Clone, PartialEq)]
pub struct MicroverseRealm {
    pub usri: String,
    pub name: String,
    pub realm_class: RealmClass,
    pub level: HierarchyLevel,
    /// The particle or system this microverse represents
    pub focus_particle: String,
}

impl MicroverseRealm {
    /// Create a quantum-scale microverse (OCS §12.4).
    pub fn quantum(name: &str, focus: &str) -> Self {
        Self {
            usri: format!("urn:omni:v1:microverse:quantum:{}", focus),
            name: name.into(),
            realm_class: RealmClass::Microverse,
            level: HierarchyLevel::L0,
            focus_particle: focus.into(),
        }
    }

    /// Create an atomic-scale microverse (OCS §12.4).
    pub fn atomic(name: &str, focus: &str) -> Self {
        Self {
            usri: format!("urn:omni:v1:microverse:atomic:{}", focus),
            name: name.into(),
            realm_class: RealmClass::Microverse,
            level: HierarchyLevel::L2,
            focus_particle: focus.into(),
        }
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("usri".into(), Value::String(self.usri.clone()));
        rec.insert("name".into(), Value::String(self.name.clone()));
        rec.insert(
            "realm_class".into(),
            Value::String(self.realm_class.as_str().into()),
        );
        rec.insert("level".into(), Value::F64(self.level.as_u8() as f64));
        rec.insert(
            "focus_particle".into(),
            Value::String(self.focus_particle.clone()),
        );
        Value::Record(rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planck_length_is_tiny() {
        assert!(constants::PLANCK_LENGTH_M < 1e-34);
        assert!(constants::PLANCK_LENGTH_M > 1e-36);
    }

    #[test]
    fn scale_factor_atomic_to_geodetic() {
        let lens = ScalingLens::between(HierarchyLevel::L2, HierarchyLevel::L5);
        // L2 (~5e-11 m) → L5 (~1e3 m): scale = 5e-11 / 1e3 ≈ 5e-14 (small)
        assert!(lens.scale_factor < 1e-10);
    }

    #[test]
    fn scale_factor_geodetic_to_atomic() {
        let lens = ScalingLens::between(HierarchyLevel::L5, HierarchyLevel::L2);
        // L5 (~1e3 m) → L2 (~5e-11 m): scale = 1e3 / 5e-11 ≈ 2e13 (large)
        assert!(lens.scale_factor > 1e10);
    }

    #[test]
    fn scaling_lens_round_trip() {
        let lens = ScalingLens::between(HierarchyLevel::L2, HierarchyLevel::L5);
        let original = 1e-10; // 1 Angstrom
        let scaled = lens.transform_length(original);
        let recovered = lens.inverse_length(scaled);
        assert!((recovered - original).abs() < 1e-20);
    }

    #[test]
    fn electron_compton_wavelength() {
        let e = ParticleProfile::electron();
        let lambda = e.compton_wavelength();
        // Should be ~2.43e-12 m
        assert!(
            (lambda - 2.43e-12).abs() < 0.1e-12,
            "got {} expected ~2.43e-12",
            lambda
        );
    }

    #[test]
    fn proton_rest_energy() {
        let p = ParticleProfile::proton();
        let e = p.rest_energy_j();
        // Should be ~1.503e-10 J (~938 MeV)
        assert!(
            (e - 1.503e-10).abs() < 0.01e-10,
            "got {} expected ~1.503e-10",
            e
        );
    }

    #[test]
    fn photon_massless() {
        let ph = ParticleProfile::photon();
        assert_eq!(ph.mass_kg, 0.0);
        assert!(ph.compton_wavelength().is_infinite());
    }

    #[test]
    fn electron_de_broglie() {
        let e = ParticleProfile::electron();
        // At 1 m/s
        let lambda = e.de_broglie_wavelength(1.0);
        assert!(lambda > 0.0);
        // At higher velocity, wavelength is shorter
        let lambda_fast = e.de_broglie_wavelength(1000.0);
        assert!(lambda_fast < lambda);
    }

    #[test]
    fn neutron_unstable() {
        let n = ParticleProfile::neutron();
        assert!(n.lifetime_s.is_finite());
        assert!(n.lifetime_s > 800.0); // ~879 seconds
    }

    #[test]
    fn higgs_boson_short_lived() {
        let h = ParticleProfile::higgs_boson();
        assert!(h.lifetime_s < 1e-20);
    }

    #[test]
    fn microverse_quantum_realm() {
        let m = MicroverseRealm::quantum("Electron Field", "electron");
        assert_eq!(m.realm_class, RealmClass::Microverse);
        assert_eq!(m.level, HierarchyLevel::L0);
    }

    #[test]
    fn microverse_atomic_realm() {
        let m = MicroverseRealm::atomic("Hydrogen Atom", "hydrogen");
        assert_eq!(m.level, HierarchyLevel::L2);
    }

    #[test]
    fn particle_to_value() {
        let e = ParticleProfile::electron();
        let v = e.to_value();
        match v {
            Value::Record(r) => {
                assert!(r.contains_key("mass_kg"));
                assert!(r.contains_key("compton_wavelength_m"));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn typical_length_monotonic() {
        // Length should generally increase with level
        let l0 = typical_length(HierarchyLevel::L0);
        let l5 = typical_length(HierarchyLevel::L5);
        let l12 = typical_length(HierarchyLevel::L12);
        assert!(l0 < l5);
        assert!(l5 < l12);
    }

    #[test]
    fn up_quark_fractional_charge() {
        let u = ParticleProfile::up_quark();
        assert!((u.charge_e - 2.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn microverse_to_value() {
        let m = MicroverseRealm::quantum("Test", "electron");
        let v = m.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(
                    r.get("realm_class"),
                    Some(&Value::String("microverse".into()))
                );
            }
            _ => panic!("expected Record"),
        }
    }
}
