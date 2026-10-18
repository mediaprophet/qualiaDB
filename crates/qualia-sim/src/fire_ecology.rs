//! Forest Fuel Accumulation, Moisture Dynamics & Fireline Intensity (`fire_ecology.rs`)
//!
//! Provides zero-heap physical modeling of:
//! - Multi-stratum forest fuel loads (surface litter, near-surface shrubs, bark hazard, canopy)
//! - Olson biomass accumulation and litter decomposition curves ($W(t) = W_{\max}(1 - e^{-kt})$)
//! - Dead Fine Fuel Moisture ($DFMC$) responding to atmospheric VPD, solar radiation, and drought
//! - Byram's Fireline Intensity ($I = H \cdot w \cdot r$ in kW/m)
//! - Decoupling of Fuel Load from Fire Occurrence and Intensity:
//!   * High fuel load under high moisture = benign creeping surface burn (Indigenous cultural mosaic burn)
//!   * Moderate fuel load under extreme solar drought + gale wind = catastrophic crowning pyroCb storm
//!
//! Adheres strictly to `AGENTS.md` §0 (Zero heap in hot paths, deterministic).

/// Heat of combustion for Australian dry sclerophyll eucalypt fuels in kJ/kg (~18,600 kJ/kg).
pub const EUCALYPT_HEAT_YIELD_KJ_KG: f32 = 18_600.0;
/// Extinction fuel moisture fraction (fuels above this moisture content will not sustain flaming).
pub const DEAD_FUEL_EXTINCTION_MOISTURE: f32 = 0.16;

/// Fuel stratum structural breakdown for an ecological vegetation community.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForestFuelStratum {
    /// Surface fine fuel load (dead leaves, twigs <6mm) in tonnes per hectare (t/ha).
    pub surface_litter_t_ha: f32,
    /// Near-surface grass and herbaceous fuel load in t/ha.
    pub near_surface_grass_t_ha: f32,
    /// Elevated shrub fuel load (shrubs 0.5 - 2.0m) in t/ha.
    pub elevated_shrub_t_ha: f32,
    /// Bark fuel hazard score (0.0 = smooth gum, 1.0 = heavy fibrous stringybark with loose ribbons).
    pub bark_hazard_score: f32,
    /// Overstory canopy foliage biomass in t/ha (available only during crowning runs).
    pub canopy_foliage_t_ha: f32,
}

impl Default for ForestFuelStratum {
    fn default() -> Self {
        Self::mature_dry_sclerophyll(8.0) // 8 years post-fire
    }
}

impl ForestFuelStratum {
    /// Evaluates fuel accumulation using the Olson (1963) steady-state litter equation:
    /// $W(t) = W_{\max} (1 - e^{-k_{\text{decay}} t})$
    pub fn mature_dry_sclerophyll(years_since_fire: f32) -> Self {
        let t = years_since_fire.max(0.0);
        // Steady-state parameters for E. sieberi / E. agglomerata dry sclerophyll
        let w_max_litter = 22.0; // t/ha
        let k_decay = 0.22; // annual decomposition rate
        let litter = w_max_litter * (1.0 - (-k_decay * t).exp());

        let grass = (3.5 * (1.0 - (-0.4 * t).exp())).min(3.5);
        let shrubs = (4.0 * (1.0 - (-0.15 * t).exp())).min(4.0);
        let bark = (0.2 + 0.1 * t).min(1.0);

        Self {
            surface_litter_t_ha: litter,
            near_surface_grass_t_ha: grass,
            elevated_shrub_t_ha: shrubs,
            bark_hazard_score: bark,
            canopy_foliage_t_ha: 12.0,
        }
    }

    /// Total surface and near-surface fine fuel load in kg/m² ($1\text{ t/ha} = 0.1\text{ kg/m}^2$).
    #[inline]
    pub fn total_fine_fuel_kg_m2(&self) -> f32 {
        (self.surface_litter_t_ha + self.near_surface_grass_t_ha) * 0.1
    }
}

/// Atmospheric and edaphic fuel dryness state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuelMoistureState {
    /// Dead Fine Fuel Moisture content fraction ($DFMC \in [0.02, 0.40]$).
    pub dead_fuel_moisture_fraction: f32,
    /// Keetch-Byram Drought Index (0 = saturated, 200 = extreme deep soil drought).
    pub kbdi_drought_index: f32,
    /// Curing percentage of herbaceous grasses ($0\% = \text{lush green}, 100\% = \text{straw dead}$).
    pub grass_curing_percentage: f32,
}

impl Default for FuelMoistureState {
    fn default() -> Self {
        Self {
            dead_fuel_moisture_fraction: 0.10,
            kbdi_drought_index: 85.0,
            grass_curing_percentage: 65.0,
        }
    }
}

/// Dynamic fireline behavior classification.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirelineRegime {
    /// No combustion possible (fuel too wet, $DFMC \ge 16\%$).
    Extinguished = 0,
    /// Low-intensity creeping surface trickle ($I < 500\text{ kW/m}$, scorch $< 2\text{ m}$).
    /// Characteristic of Indigenous cultural mosaic burning; retains mature canopy.
    CulturalCoolBurn = 1,
    /// Moderate understory fire ($500 \le I < 2,000\text{ kW/m}$, scorch $2 - 6\text{ m}$).
    ModerateUnderstory = 2,
    /// High-intensity surface fire with torching ($2,000 \le I < 10,000\text{ kW/m}$).
    HighIntensityTorching = 3,
    /// Catastrophic convective crowning firestorm ($I \ge 10,000\text{ kW/m}$, up to $60,000+\text{ kW/m}$).
    /// Generates fire whirls, extreme spotting, and pyrocumulonimbus (pyroCb) storm clouds.
    CatastrophicCrowningPyroCb = 4,
}

/// Result of Byram fireline intensity evaluation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirelineVerdict {
    /// Fire behavior classification.
    pub regime: FirelineRegime,
    /// Byram's Fireline Intensity $I = H \cdot w \cdot r$ in kW/m.
    pub fireline_intensity_kw_m: f32,
    /// Forward rate of spread in meters per second (m/s).
    pub forward_spread_rate_m_s: f32,
    /// Fuel mass consumed in the active flaming front in kg/m².
    pub fuel_consumed_kg_m2: f32,
    /// Expected flame length in meters (derived from Byram's empirical formula $L_f = 0.0775 \cdot I^{0.46}$).
    pub flame_length_m: f32,
    /// Maximum spotting distance from burning bark ribbons in kilometers.
    pub spotting_distance_km: f32,
}

/// Computes Dead Fine Fuel Moisture ($DFMC$) from ambient weather and solar exposure.
pub fn evaluate_dead_fine_fuel_moisture(
    temp_c: f32,
    relative_humidity: f32,
    solar_radiation_w_m2: f32,
    kbdi: f32,
) -> f32 {
    let rh_pct = (relative_humidity * 100.0).clamp(5.0, 100.0);
    let t = temp_c.clamp(0.0, 50.0);

    // Equilibrium Moisture Content (EMC) approximation (Simard 1968 formulation)
    let emc = if rh_pct < 10.0 {
        0.03229 + 0.281073 * rh_pct - 0.000578 * rh_pct * t
    } else if rh_pct <= 50.0 {
        2.22749 + 0.160107 * rh_pct - 0.014784 * t
    } else {
        21.0606 + 0.005565 * rh_pct * rh_pct - 0.00035 * rh_pct * t - 0.483199 * rh_pct
    };

    // Solar radiation heats the fuel surface above ambient air, further lowering moisture
    let solar_drying = (solar_radiation_w_m2 / 1000.0) * 1.8;
    // Deep drought (KBDI) reduces soil moisture replenishment from beneath
    let drought_factor = (kbdi / 200.0) * 1.2;

    let dfmc_pct = (emc - solar_drying - drought_factor).clamp(2.0, 35.0);
    dfmc_pct / 100.0
}

/// Evaluates Byram's Fireline Intensity, distinguishing fuel load from weather-driven intensity.
pub fn evaluate_fireline_intensity(
    fuels: &ForestFuelStratum,
    moisture: &FuelMoistureState,
    wind_speed_10m_m_s: f32,
    slope_angle_deg: f32,
) -> FirelineVerdict {
    let dfmc = moisture.dead_fuel_moisture_fraction;

    // 1. Extinction threshold: if fuels are too wet, fire cannot sustain
    if dfmc >= DEAD_FUEL_EXTINCTION_MOISTURE {
        return FirelineVerdict {
            regime: FirelineRegime::Extinguished,
            fireline_intensity_kw_m: 0.0,
            forward_spread_rate_m_s: 0.0,
            fuel_consumed_kg_m2: 0.0,
            flame_length_m: 0.0,
            spotting_distance_km: 0.0,
        };
    }

    // 2. Base rate of spread in dry sclerophyll (Noble, Bary, Gill 1980 / McArthur Mk 5)
    // Fuel dryness factor F_m ~ exp(-0.1 * DFMC_pct)
    let dfmc_pct = dfmc * 100.0;
    let moisture_factor = (-0.115 * dfmc_pct).exp();

    // Wind multiplier: exponential increase with 10m wind speed
    let wind_factor = (0.069 * wind_speed_10m_m_s * 3.6).exp();

    // Slope multiplier: forward spread doubles for every 10 degrees upslope
    let slope_factor = (0.0693 * slope_angle_deg.clamp(-20.0, 35.0)).exp();

    // Base surface rate of spread in m/s
    let mut rate_of_spread_m_s = 0.0012 * moisture_factor * wind_factor * slope_factor;

    // 3. Fuel consumed in surface fire (kg/m²)
    let surface_fine_kg = fuels.total_fine_fuel_kg_m2();
    let mut fuel_consumed = surface_fine_kg * ((1.0 - (dfmc / DEAD_FUEL_EXTINCTION_MOISTURE)).clamp(0.2, 1.0));

    // 4. Initial surface fireline intensity I_surface = H * w * r (in kW/m)
    let mut intensity = EUCALYPT_HEAT_YIELD_KJ_KG * fuel_consumed * rate_of_spread_m_s;

    // 5. Crown fire transition criterion (Van Wagner 1977 critical intensity I_init)
    // In dry sclerophyll eucalypts, crowning initiates when surface intensity exceeds ~4,000 kW/m
    let mut regime = if intensity < 500.0 {
        FirelineRegime::CulturalCoolBurn
    } else if intensity < 2_000.0 {
        FirelineRegime::ModerateUnderstory
    } else {
        FirelineRegime::HighIntensityTorching
    };

    // In dry sclerophyll eucalypts, crowning initiates when surface intensity exceeds ~2,500 kW/m,
    // or under extreme wind and drought (DFMC < 5%, wind > 10 m/s)
    if (intensity >= 2_500.0 || (dfmc < 0.05 && wind_speed_10m_m_s > 10.0)) && dfmc < 0.07 {
        // Full convective crowning: canopy biomass engages
        regime = FirelineRegime::CatastrophicCrowningPyroCb;
        let canopy_kg = fuels.canopy_foliage_t_ha * 0.1;
        fuel_consumed += canopy_kg * 0.85;
        // Crowning multiplies rate of spread by 2.0 - 3.5x
        rate_of_spread_m_s *= 2.8;
        intensity = EUCALYPT_HEAT_YIELD_KJ_KG * fuel_consumed * rate_of_spread_m_s;
    }

    // 6. Flame length: Byram's equation L = 0.0775 * I^0.46
    let flame_len = 0.0775 * intensity.max(1.0).powf(0.46);

    // 7. Spotting distance: driven by bark hazard and wind velocity
    let spotting_km = if intensity > 500.0 {
        fuels.bark_hazard_score * (wind_speed_10m_m_s * 0.18) * (intensity / 5_000.0).min(2.5)
    } else {
        0.0
    };

    FirelineVerdict {
        regime,
        fireline_intensity_kw_m: intensity,
        forward_spread_rate_m_s: rate_of_spread_m_s,
        fuel_consumed_kg_m2: fuel_consumed,
        flame_length_m: flame_len,
        spotting_distance_km: spotting_km,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_olson_fuel_accumulation_asymptote() {
        // 1 year post fire: light fuel
        let f1 = ForestFuelStratum::mature_dry_sclerophyll(1.0);
        // 15 years post fire: steady state mature litter
        let f15 = ForestFuelStratum::mature_dry_sclerophyll(15.0);

        assert!(f1.surface_litter_t_ha < 6.0);
        assert!(f15.surface_litter_t_ha > 18.0);
        assert!(f15.surface_litter_t_ha <= 22.0); // Asymptote
    }

    #[test]
    fn test_high_fuel_wet_conditions_yields_cool_or_extinguished_burn() {
        // High fuel load (15 years accumulation = 21 t/ha)
        let mature_fuels = ForestFuelStratum::mature_dry_sclerophyll(15.0);

        // Wet autumn/winter conditions: DFMC = 18% (> extinction 16%)
        let wet_moisture = FuelMoistureState {
            dead_fuel_moisture_fraction: 0.18,
            kbdi_drought_index: 20.0,
            grass_curing_percentage: 40.0,
        };
        let verdict = evaluate_fireline_intensity(&mature_fuels, &wet_moisture, 3.0, 0.0);
        assert_eq!(verdict.regime, FirelineRegime::Extinguished);
        assert_eq!(verdict.fireline_intensity_kw_m, 0.0);

        // Damp morning conditions: DFMC = 13% (< extinction, but cool burn)
        let damp_moisture = FuelMoistureState {
            dead_fuel_moisture_fraction: 0.13,
            kbdi_drought_index: 40.0,
            grass_curing_percentage: 50.0,
        };
        let cool_verdict = evaluate_fireline_intensity(&mature_fuels, &damp_moisture, 2.0, 0.0);
        assert_eq!(cool_verdict.regime, FirelineRegime::CulturalCoolBurn);
        assert!(cool_verdict.fireline_intensity_kw_m < 500.0);
        assert!(cool_verdict.flame_length_m < 1.5);
    }

    #[test]
    fn test_moderate_fuel_extreme_drought_yields_catastrophic_crowning() {
        // Only moderate fuel (5 years accumulation ~ 14 t/ha)
        let moderate_fuels = ForestFuelStratum::mature_dry_sclerophyll(5.0);

        // Black Summer extreme solar drought / heatwave conditions: DFMC = 3.5%, gale wind 16 m/s
        let extreme_moisture = FuelMoistureState {
            dead_fuel_moisture_fraction: 0.035,
            kbdi_drought_index: 180.0,
            grass_curing_percentage: 100.0,
        };
        let verdict = evaluate_fireline_intensity(&moderate_fuels, &extreme_moisture, 16.0, 15.0);

        // Crowning firestorm: intensity exceeds 10,000 kW/m easily
        assert_eq!(verdict.regime, FirelineRegime::CatastrophicCrowningPyroCb);
        assert!(verdict.fireline_intensity_kw_m > 15_000.0);
        assert!(verdict.spotting_distance_km > 1.0);
    }
}
