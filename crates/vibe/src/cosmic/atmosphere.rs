//! Magnetospheric and atmospheric profiles (OCS §2.4–2.5).
//!
//! Reference: OCS Specification v2.2.0 §2.4–2.5.

use crate::value::Value;
use std::collections::BTreeMap;

/// Atmospheric layer classification (OCS §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtmosphericLayer {
    Exosphere,
    Thermosphere,
    Mesosphere,
    Stratosphere,
    Troposphere,
}

impl AtmosphericLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exosphere => "exosphere",
            Self::Thermosphere => "thermosphere",
            Self::Mesosphere => "mesosphere",
            Self::Stratosphere => "stratosphere",
            Self::Troposphere => "troposphere",
        }
    }

    /// Typical altitude range base in km (OCS §2.5).
    pub fn base_altitude_km(&self) -> f64 {
        match self {
            Self::Exosphere => 600.0,
            Self::Thermosphere => 85.0,
            Self::Mesosphere => 50.0,
            Self::Stratosphere => 12.0,
            Self::Troposphere => 0.0,
        }
    }
}

/// An atmospheric profile for a celestial body (OCS §2.5).
#[derive(Debug, Clone, PartialEq)]
pub struct AtmosphericProfile {
    pub body_name: String,
    /// Surface pressure in Pascals
    pub surface_pressure_pa: f64,
    /// Surface temperature in Kelvin
    pub surface_temperature_k: f64,
    /// Molar mass of the atmosphere (kg/mol)
    pub molar_mass_kg_mol: f64,
    /// Scale height in meters
    pub scale_height_m: f64,
    /// Composition: list of (gas, fraction)
    pub composition: Vec<(&'static str, f64)>,
}

impl AtmosphericProfile {
    /// Earth's atmosphere (OCS §2.5).
    pub fn earth() -> Self {
        Self {
            body_name: "Earth".into(),
            surface_pressure_pa: 101_325.0,
            surface_temperature_k: 288.15,
            molar_mass_kg_mol: 0.0289644,
            scale_height_m: 8_500.0,
            composition: vec![
                ("N2", 0.78084),
                ("O2", 0.20946),
                ("Ar", 0.00934),
                ("CO2", 0.00042),
            ],
        }
    }

    /// Mars' atmosphere (OCS §2.5).
    pub fn mars() -> Self {
        Self {
            body_name: "Mars".into(),
            surface_pressure_pa: 636.0,
            surface_temperature_k: 210.0,
            molar_mass_kg_mol: 0.04334,
            scale_height_m: 11_100.0,
            composition: vec![("CO2", 0.960), ("N2", 0.019), ("Ar", 0.019)],
        }
    }

    /// Venus' atmosphere (OCS §2.5).
    pub fn venus() -> Self {
        Self {
            body_name: "Venus".into(),
            surface_pressure_pa: 9_200_000.0, // 92 bar
            surface_temperature_k: 737.0,
            molar_mass_kg_mol: 0.04345,
            scale_height_m: 15_900.0,
            composition: vec![("CO2", 0.965), ("N2", 0.035)],
        }
    }

    /// Compute pressure at a given altitude using the barometric formula (OCS §2.5).
    /// P(h) = P₀ * exp(-h / H)
    pub fn pressure_at_altitude(&self, altitude_m: f64) -> f64 {
        self.surface_pressure_pa * (-altitude_m / self.scale_height_m).exp()
    }

    /// Compute temperature at a given altitude (simplified adiabatic lapse, OCS §2.5).
    /// Uses a linear lapse rate of -6.5 K/km in the troposphere.
    pub fn temperature_at_altitude(&self, altitude_m: f64) -> f64 {
        let lapse_rate = 0.0065; // K/m
        let troposphere_height = 12_000.0; // 12 km
        if altitude_m < troposphere_height {
            self.surface_temperature_k - lapse_rate * altitude_m
        } else {
            // Above troposphere, temperature is roughly constant in stratosphere
            self.surface_temperature_k - lapse_rate * troposphere_height
        }
    }

    /// Determine which atmospheric layer an altitude falls into (OCS §2.5).
    pub fn layer_at_altitude(&self, altitude_m: f64) -> AtmosphericLayer {
        let alt_km = altitude_m / 1000.0;
        if alt_km >= 600.0 {
            AtmosphericLayer::Exosphere
        } else if alt_km >= 85.0 {
            AtmosphericLayer::Thermosphere
        } else if alt_km >= 50.0 {
            AtmosphericLayer::Mesosphere
        } else if alt_km >= 12.0 {
            AtmosphericLayer::Stratosphere
        } else {
            AtmosphericLayer::Troposphere
        }
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("body_name".into(), Value::String(self.body_name.clone()));
        rec.insert(
            "surface_pressure_pa".into(),
            Value::F64(self.surface_pressure_pa),
        );
        rec.insert(
            "surface_temperature_k".into(),
            Value::F64(self.surface_temperature_k),
        );
        rec.insert("scale_height_m".into(), Value::F64(self.scale_height_m));
        rec.insert(
            "composition".into(),
            Value::List(
                self.composition
                    .iter()
                    .map(|(gas, frac)| {
                        let mut r = BTreeMap::new();
                        r.insert("gas".into(), Value::String((*gas).into()));
                        r.insert("fraction".into(), Value::F64(*frac));
                        Value::Record(r)
                    })
                    .collect(),
            ),
        );
        Value::Record(rec)
    }
}

/// Magnetosphere classification (OCS §2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MagnetosphereType {
    /// No significant magnetic field (e.g. Venus, Mars)
    None,
    /// Weak induced magnetosphere
    Induced,
    /// Intrinsic dipole field (e.g. Earth, Jupiter)
    Dipole,
    /// Complex multi-pole field (e.g. Uranus, Neptune)
    Multipole,
}

impl MagnetosphereType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Induced => "induced",
            Self::Dipole => "dipole",
            Self::Multipole => "multipole",
        }
    }
}

/// A magnetospheric profile for a celestial body (OCS §2.4).
#[derive(Debug, Clone, PartialEq)]
pub struct MagnetosphereProfile {
    pub body_name: String,
    pub mag_type: MagnetosphereType,
    /// Surface equatorial field strength in Tesla
    pub surface_field_t: f64,
    /// Dipole tilt angle in degrees
    pub dipole_tilt_deg: f64,
    /// Magnetopause stand-off distance in meters (subsolar point)
    pub magnetopause_m: f64,
}

impl MagnetosphereProfile {
    /// Earth's magnetosphere (OCS §2.4).
    pub fn earth() -> Self {
        Self {
            body_name: "Earth".into(),
            mag_type: MagnetosphereType::Dipole,
            surface_field_t: 3.12e-5, // ~31,200 nT
            dipole_tilt_deg: 11.0,
            magnetopause_m: 1.0e8, // ~10 Earth radii
        }
    }

    /// Jupiter's magnetosphere — largest in the solar system (OCS §2.4).
    pub fn jupiter() -> Self {
        Self {
            body_name: "Jupiter".into(),
            mag_type: MagnetosphereType::Dipole,
            surface_field_t: 4.28e-4, // ~428,000 nT
            dipole_tilt_deg: 9.6,
            magnetopause_m: 7.5e9, // ~75 Jupiter radii
        }
    }

    /// Mars — no global field, only crustal remnants (OCS §2.4).
    pub fn mars() -> Self {
        Self {
            body_name: "Mars".into(),
            mag_type: MagnetosphereType::None,
            surface_field_t: 0.0,
            dipole_tilt_deg: 0.0,
            magnetopause_m: 0.0,
        }
    }

    /// Compute the magnetic field strength at a distance r from the center (OCS §2.4).
    /// Uses dipole approximation: B(r) = B₀ * (R / r)³
    pub fn field_at_distance(&self, r_m: f64, body_radius_m: f64) -> f64 {
        if self.surface_field_t == 0.0 {
            return 0.0;
        }
        self.surface_field_t * (body_radius_m / r_m).powi(3)
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("body_name".into(), Value::String(self.body_name.clone()));
        rec.insert(
            "mag_type".into(),
            Value::String(self.mag_type.as_str().into()),
        );
        rec.insert("surface_field_t".into(), Value::F64(self.surface_field_t));
        rec.insert("dipole_tilt_deg".into(), Value::F64(self.dipole_tilt_deg));
        rec.insert("magnetopause_m".into(), Value::F64(self.magnetopause_m));
        Value::Record(rec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_atmosphere_surface_pressure() {
        let atm = AtmosphericProfile::earth();
        let p = atm.pressure_at_altitude(0.0);
        assert!((p - 101_325.0).abs() < 1.0);
    }

    #[test]
    fn earth_atmosphere_altitude_pressure() {
        let atm = AtmosphericProfile::earth();
        // At 8.5 km (one scale height), pressure should be ~37% of surface
        let p = atm.pressure_at_altitude(8_500.0);
        let ratio = p / 101_325.0;
        assert!(
            (ratio - 0.368).abs() < 0.01,
            "got {} expected ~0.368",
            ratio
        );
    }

    #[test]
    fn earth_atmosphere_temperature_lapse() {
        let atm = AtmosphericProfile::earth();
        let t = atm.temperature_at_altitude(1_000.0);
        // 288.15 - 6.5 = 281.65 K
        assert!((t - 281.65).abs() < 0.1, "got {} expected ~281.65", t);
    }

    #[test]
    fn atmospheric_layer_classification() {
        let atm = AtmosphericProfile::earth();
        assert_eq!(atm.layer_at_altitude(0.0), AtmosphericLayer::Troposphere);
        assert_eq!(
            atm.layer_at_altitude(5_000.0),
            AtmosphericLayer::Troposphere
        );
        assert_eq!(
            atm.layer_at_altitude(20_000.0),
            AtmosphericLayer::Stratosphere
        );
        assert_eq!(
            atm.layer_at_altitude(60_000.0),
            AtmosphericLayer::Mesosphere
        );
        assert_eq!(
            atm.layer_at_altitude(100_000.0),
            AtmosphericLayer::Thermosphere
        );
        assert_eq!(
            atm.layer_at_altitude(700_000.0),
            AtmosphericLayer::Exosphere
        );
    }

    #[test]
    fn mars_atmosphere_thin() {
        let mars = AtmosphericProfile::mars();
        assert!(mars.surface_pressure_pa < 1_000.0); // < 1 kPa
    }

    #[test]
    fn venus_atmosphere_dense() {
        let venus = AtmosphericProfile::venus();
        assert!(venus.surface_pressure_pa > 1_000_000.0); // > 1 MPa
    }

    #[test]
    fn earth_magnetosphere_dipole() {
        let mag = MagnetosphereProfile::earth();
        assert_eq!(mag.mag_type, MagnetosphereType::Dipole);
        assert!(mag.surface_field_t > 0.0);
    }

    #[test]
    fn jupiter_magnetosphere_largest() {
        let jup = MagnetosphereProfile::jupiter();
        let earth = MagnetosphereProfile::earth();
        assert!(jup.magnetopause_m > earth.magnetopause_m);
        assert!(jup.surface_field_t > earth.surface_field_t);
    }

    #[test]
    fn mars_no_magnetosphere() {
        let mars = MagnetosphereProfile::mars();
        assert_eq!(mars.mag_type, MagnetosphereType::None);
        assert_eq!(mars.surface_field_t, 0.0);
    }

    #[test]
    fn dipole_field_falloff() {
        let mag = MagnetosphereProfile::earth();
        let earth_r = 6_371_000.0;
        // At 2 Earth radii, field should be 1/8 of surface
        let b = mag.field_at_distance(2.0 * earth_r, earth_r);
        let ratio = b / mag.surface_field_t;
        assert!(
            (ratio - 0.125).abs() < 0.001,
            "got {} expected 0.125",
            ratio
        );
    }

    #[test]
    fn atmosphere_to_value() {
        let atm = AtmosphericProfile::earth();
        let v = atm.to_value();
        match v {
            Value::Record(r) => {
                assert!(r.contains_key("surface_pressure_pa"));
                assert!(r.contains_key("composition"));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn magnetosphere_to_value() {
        let mag = MagnetosphereProfile::earth();
        let v = mag.to_value();
        match v {
            Value::Record(r) => {
                assert!(r.contains_key("surface_field_t"));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn atmospheric_layer_base_altitudes() {
        assert_eq!(AtmosphericLayer::Troposphere.base_altitude_km(), 0.0);
        assert!(AtmosphericLayer::Stratosphere.base_altitude_km() > 0.0);
        assert!(AtmosphericLayer::Exosphere.base_altitude_km() > 500.0);
    }
}
