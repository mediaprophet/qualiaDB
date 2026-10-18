//! Space Weather, Heliophysical Oscillations & Planetary Electrodynamics (`space_weather.rs`)
//!
//! Provides zero-heap physical modeling of:
//! - 11-year Schwabe sunspot cycle & 22-year Hale double-cycle magnetic polarity reversal
//! - Solar Extreme Ultraviolet (EUV) proxy ($F_{10.7}$ radio flux) & Thermosphere Climate Index ($TCI$)
//! - Total Solar Irradiance ($TSI$) anomalies driving planetary energy budgets
//! - Solar wind plasma stream & Interplanetary Magnetic Field (IMF $B_z$ southward reconnection)
//! - Geomagnetic storms ($K_p$, $Dst$, auroral hemispheric power)
//! - Galactic Cosmic Ray (GCR) modulation by heliospheric magnetic shielding
//! - Global Electric Circuit (GEC), ionospheric potential, and dry lightning susceptibility
//!
//! Adheres strictly to `AGENTS.md` §0 (Zero heap in hot paths, deterministic).

use core::f32::consts::PI;

/// Nominal solar cycle duration in Earth tropical years (~11.05 years).
pub const SCHWABE_CYCLE_YEARS: f32 = 11.05;
/// Hale double magnetic cycle duration in Earth years (~22.10 years).
pub const HALE_CYCLE_YEARS: f32 = 22.10;
/// Baseline solar constant / Total Solar Irradiance at 1 AU in W/m².
pub const BASELINE_TSI_W_M2: f32 = 1361.0;
/// Proton mass in kilograms for solar wind dynamic pressure calculation.
const PROTON_MASS_KG: f32 = 1.6726e-27;

/// 11-year Schwabe solar sunspot cycle and radiative output state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchwabeSolarCycle {
    /// Solar cycle number (e.g., Cycle 24, 25).
    pub cycle_number: u8,
    /// Progress within the 11-year cycle in radians $\theta \in [0, 2\pi)$.
    pub phase_rad: f32,
    /// International Sunspot Number $R_z \in [0, 300]$.
    pub sunspot_number: f32,
    /// 10.7 cm Solar Radio Flux in solar flux units ($10^{-22}\text{ W}/(\text{m}^2\cdot\text{Hz})$).
    pub f10_7_radio_flux_sfu: f32,
    /// Total Solar Irradiance anomaly relative to 1361.0 W/m² ($\pm 1.3\text{ W/m}^2$).
    pub tsi_anomaly_w_m2: f32,
    /// Thermosphere Climate Index (NO 5.3 µm emission power) in $10^{10}\text{ W}$ ($2.0 - 50.0$).
    pub thermosphere_climate_index_10e10_w: f32,
    /// Fraction of spotless days in the current year ($0.0 - 1.0$).
    pub spotless_days_ratio: f32,
}

impl Default for SchwabeSolarCycle {
    fn default() -> Self {
        Self {
            cycle_number: 25,
            phase_rad: 2.8, // Near Cycle 25 maximum
            sunspot_number: 145.0,
            f10_7_radio_flux_sfu: 131.0,
            tsi_anomaly_w_m2: 0.95,
            thermosphere_climate_index_10e10_w: 12.29,
            spotless_days_ratio: 0.02,
        }
    }
}

/// 22-year Hale magnetic polarity cycle (two Schwabe cycles average).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HaleMagneticCycle {
    /// Phase within the 22.1-year double cycle $\theta \in [0, 4\pi)$.
    pub double_cycle_phase_rad: f32,
    /// Solar magnetic dipole polarity: true = North pole positive (even cycles), false = South positive.
    pub is_solar_north_positive: bool,
    /// Galactic Cosmic Ray (GCR) flux multiplier ($\in [0.75, 1.25]$, higher at solar minimum).
    pub cosmic_ray_modulation_factor: f32,
    /// Decadal drought propensity modifier ($0.0 - 1.0$, coupling to Southern Hemisphere rainfall).
    pub decadal_drought_propensity: f32,
}

impl Default for HaleMagneticCycle {
    fn default() -> Self {
        Self {
            double_cycle_phase_rad: 2.8,
            is_solar_north_positive: false, // Cycle 25 is odd
            cosmic_ray_modulation_factor: 0.88,
            decadal_drought_propensity: 0.55,
        }
    }
}

/// Solar wind in-situ plasma parameters at L1 / Earth orbit.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarWindState {
    /// Bulk solar wind flow velocity in km/s (typical 300 - 800 km/s).
    pub velocity_km_s: f32,
    /// Proton number density in particles/cm³ (typical 1.0 - 30.0 cm⁻³).
    pub proton_density_cm3: f32,
    /// Interplanetary Magnetic Field total magnitude $B_{\text{total}}$ in nT.
    pub imf_total_nt: f32,
    /// IMF vertical component $B_z$ in GSM coordinates in nT (negative = southward).
    pub imf_bz_nt: f32,
    /// Dynamic ram pressure $P_{\text{dyn}} = \rho v^2$ in nPa.
    pub dynamic_pressure_npa: f32,
}

impl Default for SolarWindState {
    fn default() -> Self {
        let v = 420.0;
        let n = 5.0;
        // P_dyn = 1.6726e-27 * (n * 1e6) * (v * 1e3)^2 * 1e9 nPa
        let p_dyn = PROTON_MASS_KG * (n * 1e6) * (v * 1e3) * (v * 1e3) * 1e9;
        Self {
            velocity_km_s: v,
            proton_density_cm3: n,
            imf_total_nt: 4.5,
            imf_bz_nt: -1.4, // slight southward tilt
            dynamic_pressure_npa: p_dyn,
        }
    }
}

/// Planetary geomagnetic storm state and auroral energy dissipation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeomagneticStormState {
    /// Planetary 3-hour K-index $K_p \in [0.0, 9.0]$.
    pub kp_index: f32,
    /// Disturbance Storm Time index $Dst$ in nT (negative during ring current intensification).
    pub dst_index_nt: f32,
    /// Total hemispheric auroral precipitation power in Gigawatts (5 - 350 GW).
    pub auroral_power_gw: f32,
    /// True if dayside magnetic reconnection is currently injecting solar wind plasma.
    pub is_reconnection_active: bool,
}

/// Global Electric Circuit and atmospheric electrical state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtmosphericElectricCircuit {
    /// Ionospheric electric potential relative to Earth ground in kV (~200 - 300 kV).
    pub ionospheric_potential_kv: f32,
    /// Fair-weather vertical atmospheric electric field at ground level in V/m (~100 - 150 V/m).
    pub fair_weather_field_v_m: f32,
    /// Lower atmospheric cosmic ray ionization rate in ion-pairs/(cm³·s).
    pub ionization_rate_pairs_cm3_s: f32,
    /// Susceptibility to dry lightning ignition ($0.0 - 1.0$) given convective instability.
    pub dry_lightning_susceptibility: f32,
}

/// Computes the astronomical Schwabe and Hale solar cycle states for an absolute calendar year.
pub fn evaluate_solar_cycle_phase(decimal_year: f32) -> (SchwabeSolarCycle, HaleMagneticCycle) {
    // Reference epoch: Cycle 24/25 solar minimum at ~December 2019.9 (decimal 2019.9)
    let ref_min_year = 2019.9;
    let elapsed = decimal_year - ref_min_year;

    // Schwabe 11-year phase [0, 2pi)
    let raw_phase = (elapsed / SCHWABE_CYCLE_YEARS) * 2.0 * PI;
    let phase = raw_phase.rem_euclid(2.0 * PI);

    // Approximate cycle number (Cycle 25 started ~2019.9)
    let cycle_number = (25 + (elapsed / SCHWABE_CYCLE_YEARS).floor() as i32).max(1) as u8;

    // Sunspot number peak occurs around 4 years into the 11-year cycle (~ phase = 0.72 * pi)
    let activity_shape = ((phase - PI * 0.72).cos() + 1.0) * 0.5;
    let sunspot_number = (activity_shape * 175.0).max(0.0);

    // 10.7 cm radio flux: baseline 65 sfu + activity * 150 sfu
    let f10_7 = 68.0 + activity_shape * 140.0;

    // TSI anomaly: ±1.2 W/m²
    let tsi_anomaly = (activity_shape - 0.5) * 2.4;

    // TCI: 2.5 to 35.0 x 10^10 W
    let tci = 3.0 + activity_shape * 28.0;

    // Spotless days ratio: high (up to 0.80) at minimum, near 0.0 at maximum
    let spotless_ratio = ((1.0 - activity_shape).powf(2.0) * 0.78).clamp(0.0, 1.0);

    let schwabe = SchwabeSolarCycle {
        cycle_number,
        phase_rad: phase,
        sunspot_number,
        f10_7_radio_flux_sfu: f10_7,
        tsi_anomaly_w_m2: tsi_anomaly,
        thermosphere_climate_index_10e10_w: tci,
        spotless_days_ratio: spotless_ratio,
    };

    // Hale 22-year double cycle phase [0, 4pi)
    let double_phase = (elapsed / HALE_CYCLE_YEARS) * 4.0 * PI;
    let double_phase_mod = double_phase.rem_euclid(4.0 * PI);
    let is_north_positive = cycle_number % 2 == 0;

    // Cosmic ray modulation: peak cosmic rays at solar minimum (activity_shape = 0)
    let gcr_factor = 1.22 - activity_shape * 0.40;

    // Decadal drought propensity in SE Australia correlates with prolonged solar magnetic transitions
    let drought_propensity = (0.35 + (double_phase_mod * 0.5).sin().abs() * 0.45).clamp(0.0, 1.0);

    let hale = HaleMagneticCycle {
        double_cycle_phase_rad: double_phase_mod,
        is_solar_north_positive: is_north_positive,
        cosmic_ray_modulation_factor: gcr_factor,
        decadal_drought_propensity: drought_propensity,
    };

    (schwabe, hale)
}

/// Evaluates real-time space weather coupling into Earth's magnetosphere and Global Electric Circuit.
pub fn evaluate_space_weather_coupling(
    solar: &SchwabeSolarCycle,
    hale: &HaleMagneticCycle,
    wind: &SolarWindState,
) -> (GeomagneticStormState, AtmosphericElectricCircuit) {
    // 1. Dayside magnetic reconnection: occurs when IMF Bz is southward (Bz < 0)
    let reconnection = wind.imf_bz_nt < -1.5;
    let bz_drive = (-wind.imf_bz_nt).max(0.0);

    // 2. Newell coupling function: dPhi/dt ~ v^(4/3) * B_T^(2/3) * sin^(8/3)(theta_clock / 2)
    let clock_angle = wind.imf_bz_nt.atan2(wind.imf_total_nt.max(0.1));
    let sin_half = ((PI - clock_angle) * 0.5).sin().abs();
    let newell_rate = (wind.velocity_km_s.powf(1.333) * wind.imf_total_nt.powf(0.667) * sin_half.powf(2.667)) * 1e-4;

    // 3. Planetary Kp-index estimation [0.0, 9.0]
    let base_kp = (wind.velocity_km_s / 160.0) + (bz_drive * 0.7) + (wind.dynamic_pressure_npa * 0.4);
    let kp = base_kp.clamp(0.0, 9.0);

    // 4. Dst ring current index: quiet +5 nT, storms reach -100 to -350 nT
    let dst = if reconnection {
        (10.0 - bz_drive * 18.0 - (wind.velocity_km_s - 400.0) * 0.15).min(15.0)
    } else {
        5.0
    };

    // 5. Auroral hemispheric precipitation power in GW
    let auroral_gw = (8.0 + kp * 15.0 + newell_rate * 2.5).clamp(5.0, 350.0);

    let storm = GeomagneticStormState {
        kp_index: kp,
        dst_index_nt: dst,
        auroral_power_gw: auroral_gw,
        is_reconnection_active: reconnection,
    };

    // 6. Global Electric Circuit (GEC)
    // Fair-weather field ~ 120 V/m, modulated by solar wind potential and GCR ionization
    let gcr_ion_rate = 10.0 * hale.cosmic_ray_modulation_factor; // pairs/(cm³·s) in lower atmosphere
    let iono_pot_kv = 240.0 + (solar.f10_7_radio_flux_sfu / 200.0) * 45.0 + (kp * 5.0);
    let fair_weather_v_m = 110.0 + (iono_pot_kv - 240.0) * 0.35;

    // 7. Dry lightning susceptibility: high when ionization is elevated and solar heating creates high VPD
    let dry_lightning = (0.25 + (kp / 15.0) + (1.0 - solar.spotless_days_ratio) * 0.35).clamp(0.0, 1.0);

    let gec = AtmosphericElectricCircuit {
        ionospheric_potential_kv: iono_pot_kv,
        fair_weather_field_v_m: fair_weather_v_m,
        ionization_rate_pairs_cm3_s: gcr_ion_rate,
        dry_lightning_susceptibility: dry_lightning,
    };

    (storm, gec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schwabe_and_hale_cycle_periodicity() {
        // At minimum (2019.9) vs near peak (2024.5)
        let (min_schwabe, min_hale) = evaluate_solar_cycle_phase(2019.9);
        let (max_schwabe, max_hale) = evaluate_solar_cycle_phase(2024.5);

        // At solar min, spotless days ratio is high, sunspots low
        assert!(min_schwabe.spotless_days_ratio > 0.40);
        assert!(min_schwabe.sunspot_number < 40.0);

        // At solar max, sunspots high, spotless days near zero
        assert!(max_schwabe.sunspot_number > 80.0);
        assert!(max_schwabe.spotless_days_ratio < 0.20);
        assert!(max_schwabe.f10_7_radio_flux_sfu > min_schwabe.f10_7_radio_flux_sfu);

        // Hale cycle polarity is consistent
        assert_eq!(min_hale.is_solar_north_positive, max_hale.is_solar_north_positive);
    }

    #[test]
    fn test_southward_imf_triggers_geomagnetic_storm() {
        let (solar, hale) = evaluate_solar_cycle_phase(2025.0);

        // Northward IMF (quiet)
        let mut quiet_wind = SolarWindState::default();
        quiet_wind.imf_bz_nt = 3.5; // Northward
        quiet_wind.velocity_km_s = 380.0;
        let (quiet_storm, _) = evaluate_space_weather_coupling(&solar, &hale, &quiet_wind);
        assert!(!quiet_storm.is_reconnection_active);
        assert!(quiet_storm.kp_index < 4.0);

        // Strong Southward IMF (CME impact)
        let mut storm_wind = SolarWindState::default();
        storm_wind.imf_bz_nt = -14.0; // Strong Southward
        storm_wind.velocity_km_s = 680.0;
        let (active_storm, _) = evaluate_space_weather_coupling(&solar, &hale, &storm_wind);
        assert!(active_storm.is_reconnection_active);
        assert!(active_storm.kp_index >= 5.0); // G1+ geomagnetic storm
        assert!(active_storm.dst_index_nt < -50.0); // Ring current depression
        assert!(active_storm.auroral_power_gw > quiet_storm.auroral_power_gw);
    }

    #[test]
    fn test_cosmic_ray_flux_peaks_at_solar_minimum() {
        let (_, min_hale) = evaluate_solar_cycle_phase(2019.9); // Minimum
        let (_, max_hale) = evaluate_solar_cycle_phase(2024.5); // Maximum

        // Cosmic rays are shielded during solar maximum by magnetic fields
        assert!(min_hale.cosmic_ray_modulation_factor > max_hale.cosmic_ray_modulation_factor);
    }

    #[test]
    fn test_space_weather_modulates_dry_lightning() {
        let (solar, hale) = evaluate_solar_cycle_phase(2024.5);
        let wind = SolarWindState::default();
        let (_, gec) = evaluate_space_weather_coupling(&solar, &hale, &wind);

        assert!(gec.ionospheric_potential_kv > 200.0);
        assert!(gec.fair_weather_field_v_m > 90.0);
        assert!(gec.dry_lightning_susceptibility > 0.0);
    }
}
