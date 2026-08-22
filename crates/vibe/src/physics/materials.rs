//! Material Signatures — physical continuant properties and response traits.
//!
//! Encodes mechanical, optical, acoustic, chemical, and thermal facets on continuants
//! as unit-bearing signed records without polluting 10D tensor coordinates.
//!
//! Reference: `docs/plans/vibe-design/20260819_fields-materials-and-creator-physics.md` §2.2.

/// Mechanical response facet.
#[derive(Debug, Clone, PartialEq)]
pub struct MechanicalFacet {
    /// Elastic yield strength threshold (kPa).
    pub yield_kpa: f64,
    /// Young's modulus of elasticity (GPa).
    pub youngs_modulus_gpa: f64,
    /// Mass density (kg/m³).
    pub density_kg_m3: f64,
    /// Poisson's ratio (dimensionless, typically 0.0 to 0.5).
    pub poisson_ratio: f64,
}

/// Optical response facet.
#[derive(Debug, Clone, PartialEq)]
pub struct OpticalFacet {
    /// Surface diffuse albedo / reflectance in [0.0, 1.0].
    pub albedo: f64,
    /// Refractive index (IOR).
    pub ior: f64,
    /// Optical absorption coefficient (1/m).
    pub absorption: f64,
}

/// Acoustic response facet.
#[derive(Debug, Clone, PartialEq)]
pub struct AcousticFacet {
    /// Characteristic acoustic impedance (Pa·s/m or Rayls).
    pub impedance_rayls: f64,
    /// Acoustic absorption coefficient in [0.0, 1.0].
    pub absorption_coeff: f64,
}

/// Chemical and dissolution response facet.
#[derive(Debug, Clone, PartialEq)]
pub struct ChemicalFacet {
    /// Solvent species IRI in which this material dissolves (e.g. `did:q42:species:H2O`).
    pub soluble_in: Option<String>,
    /// Dissolution rate constant (1/s) under standard reference conditions.
    pub dissolve_rate_per_s: f64,
    /// Resulting solution/dissolved species IRI (e.g. `did:q42:species:sucrose(aq)`).
    pub dissolve_products: Option<String>,
    /// List of immiscible species/materials (e.g. oil vs water interface barrier).
    pub immiscible_with: Vec<String>,
}

/// Thermal response facet.
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalFacet {
    /// Thermal conductivity (W/(m·K)).
    pub conductivity_w_mk: f64,
    /// Specific heat capacity (J/(kg·K)).
    pub specific_heat_j_kgk: f64,
    /// Melting point (Kelvin).
    pub melt_point_k: Option<f64>,
    /// Boiling / vaporization point (Kelvin).
    pub boil_point_k: Option<f64>,
}

/// A complete, multi-faceted Material Signature attached to a Continuant.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialSignature {
    /// Canonical material signature IRI (e.g. `<did:q42:material:sucrose-cube-v1>`).
    pub id: String,
    /// Human-readable material name.
    pub name: String,
    /// Mechanical properties.
    pub mechanical: Option<MechanicalFacet>,
    /// Optical & EM properties.
    pub optical: Option<OpticalFacet>,
    /// Acoustic properties.
    pub acoustic: Option<AcousticFacet>,
    /// Chemical solubility & miscibility traits.
    pub chemical: Option<ChemicalFacet>,
    /// Thermal & thermodynamic properties.
    pub thermal: Option<ThermalFacet>,
}

impl MaterialSignature {
    /// Sugar Cube (Sucrose) canonical specification from primer §2.2.
    pub fn sugar_cube() -> Self {
        Self {
            id: "did:q42:material:sucrose-cube-v1".to_string(),
            name: "Sucrose Cube".to_string(),
            mechanical: Some(MechanicalFacet {
                yield_kpa: 50.0,
                youngs_modulus_gpa: 10.0,
                density_kg_m3: 1580.0,
                poisson_ratio: 0.28,
            }),
            optical: Some(OpticalFacet {
                albedo: 0.85,
                ior: 1.537,
                absorption: 0.05,
            }),
            acoustic: Some(AcousticFacet {
                impedance_rayls: 2.3e6,
                absorption_coeff: 0.15,
            }),
            chemical: Some(ChemicalFacet {
                soluble_in: Some("did:q42:species:H2O".to_string()),
                dissolve_rate_per_s: 0.5,
                dissolve_products: Some("did:q42:species:sucrose(aq)".to_string()),
                immiscible_with: Vec::new(),
            }),
            thermal: Some(ThermalFacet {
                conductivity_w_mk: 0.58,
                specific_heat_j_kgk: 1250.0,
                melt_point_k: Some(459.0), // 186 °C
                boil_point_k: None,
            }),
        }
    }

    /// Liquid Water canonical specification.
    pub fn liquid_water() -> Self {
        Self {
            id: "did:q42:species:H2O".to_string(),
            name: "Liquid Water".to_string(),
            mechanical: Some(MechanicalFacet {
                yield_kpa: 0.0,
                youngs_modulus_gpa: 2.2, // Bulk modulus
                density_kg_m3: 1000.0,
                poisson_ratio: 0.5,
            }),
            optical: Some(OpticalFacet {
                albedo: 0.05,
                ior: 1.333,
                absorption: 0.01,
            }),
            acoustic: Some(AcousticFacet {
                impedance_rayls: 1.48e6,
                absorption_coeff: 0.01,
            }),
            chemical: Some(ChemicalFacet {
                soluble_in: None,
                dissolve_rate_per_s: 0.0,
                dissolve_products: None,
                immiscible_with: vec!["did:q42:material:mineral-oil-v1".to_string()],
            }),
            thermal: Some(ThermalFacet {
                conductivity_w_mk: 0.6,
                specific_heat_j_kgk: 4184.0,
                melt_point_k: Some(273.15),
                boil_point_k: Some(373.15),
            }),
        }
    }

    /// Mineral Oil specification (immiscible with water).
    pub fn mineral_oil() -> Self {
        Self {
            id: "did:q42:material:mineral-oil-v1".to_string(),
            name: "Mineral Oil".to_string(),
            mechanical: Some(MechanicalFacet {
                yield_kpa: 0.0,
                youngs_modulus_gpa: 1.5,
                density_kg_m3: 850.0,
                poisson_ratio: 0.5,
            }),
            optical: Some(OpticalFacet {
                albedo: 0.1,
                ior: 1.47,
                absorption: 0.02,
            }),
            acoustic: Some(AcousticFacet {
                impedance_rayls: 1.2e6,
                absorption_coeff: 0.02,
            }),
            chemical: Some(ChemicalFacet {
                soluble_in: None,
                dissolve_rate_per_s: 0.0,
                dissolve_products: None,
                immiscible_with: vec!["did:q42:species:H2O".to_string()],
            }),
            thermal: Some(ThermalFacet {
                conductivity_w_mk: 0.14,
                specific_heat_j_kgk: 2000.0,
                melt_point_k: Some(243.0),
                boil_point_k: Some(573.0),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sugar_cube_signature() {
        let sugar = MaterialSignature::sugar_cube();
        assert_eq!(sugar.name, "Sucrose Cube");
        let mech = sugar.mechanical.as_ref().unwrap();
        assert_eq!(mech.yield_kpa, 50.0);
        let chem = sugar.chemical.as_ref().unwrap();
        assert_eq!(chem.soluble_in.as_deref(), Some("did:q42:species:H2O"));
        assert_eq!(chem.dissolve_rate_per_s, 0.5);
    }

    #[test]
    fn test_immiscibility_relation() {
        let water = MaterialSignature::liquid_water();
        let oil = MaterialSignature::mineral_oil();

        let water_chem = water.chemical.as_ref().unwrap();
        let oil_chem = oil.chemical.as_ref().unwrap();

        assert!(water_chem.immiscible_with.contains(&oil.id));
        assert!(oil_chem.immiscible_with.contains(&water.id));
    }
}
