//! Universal Flora Archetypes & Raunkiær Life-Form Dynamics (`flora_archetype.rs`)
//!
//! Unifies woody trees, culinary herbs, grasses, climbing vines, succulents, and geophytes
//! into a single continuous biophysical parameter space:
//! - Biomechanical support: Lignified heartwood vs. Pressurized hydrostats vs. Hollow culms
//! - Raunkiær Life Forms: Phanerophytes, Chamaephytes, Hemicryptophytes, Cryptophytes, Therophytes
//! - Clonal network dynamics (horizontal stolons/rhizomes with adventitious rooting)
//! - Frost/drought pruning state machine (natural seasonal dieback)
//!
//! Strictly zero heap in hot evaluation loops (`AGENTS.md` §0).

/// Raunkiær life form based on the vertical height $z$ of surviving buds during the harsh season.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaunkiaerLifeForm {
    /// Trees & Tall Shrubs: Meristems high above ground ($z > 0.25\text{ m}$).
    Phanerophyte = 0,
    /// Woody Low Herbs/Dwarf Shrubs (e.g. Rosemary, Thyme): Meristems close to soil ($0 < z \le 0.25\text{ m}$).
    Chamaephyte = 1,
    /// Perennial Herbs & Tussock Grasses (e.g. Mint, Pasture): Above-ground shoots die back to collar ($z = 0$).
    Hemicryptophyte = 2,
    /// Geophytes/Bulbs/Rhizomes (e.g. Potato, Ginger, Ferns): Meristems survive underground ($z < 0$).
    Cryptophyte = 3,
    /// Annuals (e.g. Basil, Wheat, Poppy): Plant dies after fruiting; persists only as dormant seeds.
    Therophyte = 4,
}

/// Mechanical stem support regime.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StemSupportRegime {
    /// Lignified heartwood core (permanent structural stiffness).
    LignifiedWood = 0,
    /// Pressurized cellular hydrostat (collapses when turgor $P_t \to 0$).
    PressurizedHydrostat = 1,
    /// Hollow cylindrical monocot culm (high strength-to-weight ratio).
    HollowCulm = 2,
    /// Climbing vine / structural parasite (fails Euler buckling, coils upon contact).
    ClimbingVine = 3,
    /// 3D hydraulic capacitor (inflated water storage mesh).
    SucculentCapacitor = 4,
}

/// Continuous biophysical trait parameters defining any flora archetype.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloraArchetypeConfig {
    pub life_form: RaunkiaerLifeForm,
    pub support_regime: StemSupportRegime,
    /// Lignification coefficient $\lambda \in [0.0, 1.0]$: 1.0 = solid wood, 0.0 = pure herbaceous tissue
    pub lignification: f32,
    /// Culm internal cavity ratio $\kappa = r_{\text{inner}} / r_{\text{outer}} \in [0.0, 0.95]$ (0.0 for solid stem)
    pub cavity_ratio: f32,
    /// Gravitropism target angle $\theta_g$ in radians (0.0 = vertical upright, $\pi/2 \approx 1.57$ = horizontal stolon)
    pub gravitropism_angle_rad: f32,
    /// Lethal freezing temperature threshold in °C (triggers dieback)
    pub frost_lethality_temp_c: f32,
    /// Critical drought turgor threshold in MPa below which stems buckle
    pub turgor_buckling_threshold_mpa: f32,
    /// Water storage capacity in $kg / m^3$ (massive for succulents)
    pub hydraulic_capacitance: f32,
}

impl FloraArchetypeConfig {
    /// Woody Tree Archetype (Eucalyptus, Oak, Pine).
    pub const fn woody_tree() -> Self {
        Self {
            life_form: RaunkiaerLifeForm::Phanerophyte,
            support_regime: StemSupportRegime::LignifiedWood,
            lignification: 0.95,
            cavity_ratio: 0.0,
            gravitropism_angle_rad: 0.0,
            frost_lethality_temp_c: -12.0,
            turgor_buckling_threshold_mpa: 0.0, // Stays erect even when dead
            hydraulic_capacitance: 50.0,
        }
    }

    /// Herbaceous Culinary Herb Archetype (Basil, Mint, Tomato).
    pub const fn culinary_herb() -> Self {
        Self {
            life_form: RaunkiaerLifeForm::Hemicryptophyte,
            support_regime: StemSupportRegime::PressurizedHydrostat,
            lignification: 0.08,
            cavity_ratio: 0.25,
            gravitropism_angle_rad: 0.15,
            frost_lethality_temp_c: -1.0,
            turgor_buckling_threshold_mpa: 0.25,
            hydraulic_capacitance: 120.0,
        }
    }

    /// Tussock Grass Archetype (Poa labillardierei, Kangaroo Grass).
    pub const fn tussock_grass() -> Self {
        Self {
            life_form: RaunkiaerLifeForm::Hemicryptophyte,
            support_regime: StemSupportRegime::HollowCulm,
            lignification: 0.25,
            cavity_ratio: 0.75, // Thin hollow tube
            gravitropism_angle_rad: 0.05,
            frost_lethality_temp_c: -8.0,
            turgor_buckling_threshold_mpa: 0.15,
            hydraulic_capacitance: 75.0,
        }
    }

    /// Climbing Vine Archetype (Wonga Vine, Ivy).
    pub const fn climbing_vine() -> Self {
        Self {
            life_form: RaunkiaerLifeForm::Chamaephyte,
            support_regime: StemSupportRegime::ClimbingVine,
            lignification: 0.40,
            cavity_ratio: 0.10,
            gravitropism_angle_rad: 0.60,
            frost_lethality_temp_c: -4.0,
            turgor_buckling_threshold_mpa: 0.10,
            hydraulic_capacitance: 65.0,
        }
    }

    /// Succulent Archetype (Aloe, Pigface, Cactus).
    pub const fn succulent() -> Self {
        Self {
            life_form: RaunkiaerLifeForm::Chamaephyte,
            support_regime: StemSupportRegime::SucculentCapacitor,
            lignification: 0.05,
            cavity_ratio: 0.0,
            gravitropism_angle_rad: 0.10,
            frost_lethality_temp_c: -2.0,
            turgor_buckling_threshold_mpa: 0.05,
            hydraulic_capacitance: 650.0, // High hydraulic capacitance
        }
    }
}

/// Evaluator for stem mechanics and seasonal dieback across flora archetypes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloraBiomechanicsEvaluator {
    pub config: FloraArchetypeConfig,
    /// Current internal water potential / turgor in MPa
    pub turgor_pressure_mpa: f32,
    /// Current ambient temperature in °C
    pub ambient_temp_c: f32,
    /// Has the plant experienced lethal frost or drought pruning
    pub is_dieback_active: bool,
    /// Effective height of surviving crown in meters
    pub crown_height_m: f32,
}

impl FloraBiomechanicsEvaluator {
    pub fn new(config: FloraArchetypeConfig, initial_height_m: f32) -> Self {
        Self {
            config,
            turgor_pressure_mpa: 1.0,
            ambient_temp_c: 20.0,
            is_dieback_active: false,
            crown_height_m: initial_height_m,
        }
    }

    /// Evaluates effective second moment of area:
    /// - Solid stem: $I = \frac{\pi}{4} r_o^4$
    /// - Hollow culm: $I = \frac{\pi}{4} (r_o^4 - r_i^4) = \frac{\pi}{4} r_o^4 (1 - \kappa^4)$
    #[inline]
    pub fn second_moment_i(&self, outer_radius_m: f32) -> f32 {
        let r4 = outer_radius_m.powi(4);
        let kappa4 = self.config.cavity_ratio.clamp(0.0, 0.95).powi(4);
        (std::f32::consts::PI * 0.25) * r4 * (1.0 - kappa4)
    }

    /// Evaluates effective Young's modulus accounting for lignification and hydrostatic turgor:
    /// $E_{\text{eff}} = \lambda \cdot E_{\text{wood}} + (1 - \lambda) \cdot (E_{\text{tissue}} + k_{\text{hydro}} P_t)$
    pub fn effective_elastic_modulus_mpa(&self) -> f32 {
        let e_wood = 900.0; // 900 MPa wood
        let e_tissue = 8.0;  // 8 MPa flaccid parenchyma
        let k_hydro = 80.0; // 80 MPa per MPa turgor

        let e_hydrostat = e_tissue + k_hydro * self.turgor_pressure_mpa.max(0.0);
        let lambda = self.config.lignification.clamp(0.0, 1.0);

        lambda * e_wood + (1.0 - lambda) * e_hydrostat
    }

    /// Checks if internal turgor is below the Euler buckling limit (causing stem to collapse).
    #[inline]
    pub fn is_stem_buckled(&self) -> bool {
        if self.config.support_regime == StemSupportRegime::LignifiedWood {
            return false; // Wood never buckles from low turgor
        }
        self.turgor_pressure_mpa < self.config.turgor_buckling_threshold_mpa
    }

    /// Updates seasonal environmental conditions and triggers Raunkiær dieback pruning.
    pub fn step_environment(&mut self, temp_c: f32, soil_moisture: f32, full_height_m: f32) {
        self.ambient_temp_c = temp_c;
        self.turgor_pressure_mpa = (soil_moisture * 1.5).clamp(0.0, 1.6);

        let is_frost_lethal = temp_c < self.config.frost_lethality_temp_c;
        let is_drought_lethal = self.turgor_pressure_mpa < 0.05;

        if is_frost_lethal || is_drought_lethal {
            self.is_dieback_active = true;

            // Prune crown height down to Raunkiær bud survival zone
            self.crown_height_m = match self.config.life_form {
                RaunkiaerLifeForm::Phanerophyte => full_height_m * 0.9, // Trunk and main boughs survive
                RaunkiaerLifeForm::Chamaephyte => 0.20,                 // Stems pruned down to 20cm
                RaunkiaerLifeForm::Hemicryptophyte => 0.02,             // Die back to soil collar (2cm)
                RaunkiaerLifeForm::Cryptophyte => 0.0,                  // Above-ground parts vanish completely
                RaunkiaerLifeForm::Therophyte => 0.0,                   // Parent plant dead, only seeds persist
            };
        } else if self.is_dieback_active && temp_c > 10.0 && soil_moisture > 0.4 {
            // Warm spring recovery: regrow toward full height
            self.crown_height_m = (self.crown_height_m + 0.1 * full_height_m).min(full_height_m);
            if (self.crown_height_m - full_height_m).abs() < 0.05 {
                self.is_dieback_active = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_herb_vs_tree_buckling() {
        let mut herb = FloraBiomechanicsEvaluator::new(FloraArchetypeConfig::culinary_herb(), 0.6);
        let mut tree = FloraBiomechanicsEvaluator::new(FloraArchetypeConfig::woody_tree(), 12.0);

        // Under drought conditions: turgor = 0.05 MPa
        herb.turgor_pressure_mpa = 0.05;
        tree.turgor_pressure_mpa = 0.05;

        assert!(herb.is_stem_buckled(), "Herb must buckle without turgor pressure");
        assert!(!tree.is_stem_buckled(), "Tree wood must not buckle from turgor loss");
    }

    #[test]
    fn test_raunkiaer_frost_dieback() {
        let mut grass = FloraBiomechanicsEvaluator::new(FloraArchetypeConfig::tussock_grass(), 0.8);
        assert_eq!(grass.crown_height_m, 0.8);

        // Winter frost: -12°C
        grass.step_environment(-12.0, 0.5, 0.8);

        assert!(grass.is_dieback_active);
        // Hemicryptophyte must prune down to collar (2cm)
        assert!(grass.crown_height_m <= 0.05);
    }
}
