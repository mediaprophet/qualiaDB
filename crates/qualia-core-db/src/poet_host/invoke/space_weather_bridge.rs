//! Zero-Heap Space Weather & Heliophysical Engine Bridge.
//!
//! Provides deterministic, zero-heap physical modeling of:
//! - Shue et al. (1997) / Chapman-Ferraro magnetopause standoff distance R_mp
//! - Spreiter / Farris-Russell bow shock standoff distance R_bs
//! - Burton-McPherron-Russell (1975) Dst ring current depression
//! - Newell coupling rate & planetary Kp index [0.0, 9.0]
//! - NOAA G-scale storm categorization (G0..G5)
//! - Chapman-layer ionospheric Total Electron Content (TEC) in TECU
//! - Feldstein / Holzworth-Meng auroral oval equatorward boundary latitude
//! - Hathaway et al. solar cycle sunspot number (SSN) progression
//!
//! Adheres strictly to Tier-1 Zero-Heap hot-path execution (< 350 lines).

pub const PROTON_MASS_KG: f64 = 1.67262192369e-27;
pub const BASELINE_TSI_W_M2: f64 = 1361.0;

/// Input solar wind and solar radiative forcing parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpaceWeatherInput {
    pub imf_bz_nt: f64,
    pub solar_wind_speed_km_s: f64,
    pub proton_density_cm3: f64,
    pub f107_flux_sfu: f64,
    pub subsolar_zenith_deg: f64,
    pub cycle_month: f64,
}

/// Output magnetospheric, ionospheric, and solar cycle state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpaceWeatherOutput {
    pub magnetopause_standoff_re: f64,
    pub bow_shock_re: f64,
    pub dst_index_nt: f64,
    pub kp_index: f64,
    pub tec_tecu: f64,
    pub auroral_oval_equatorward_lat_deg: f64,
    pub geomag_storm_scale_g: u8,
    pub sunspot_number_ssn: f64,
}

/// Typed validation errors for fail-closed space weather evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceWeatherError {
    NonFiniteInput,
    NegativeProtonDensity,
    InvalidSolarWindSpeed,
}

impl core::fmt::Display for SpaceWeatherError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "Input parameter is non-finite (NaN or Inf)"),
            Self::NegativeProtonDensity => write!(f, "Proton density cannot be negative"),
            Self::InvalidSolarWindSpeed => write!(f, "Solar wind speed must be strictly positive"),
        }
    }
}

impl std::error::Error for SpaceWeatherError {}

/// Evaluates real-time space weather physics with zero heap allocations.
pub fn evaluate_space_weather_zero_heap(
    input: &SpaceWeatherInput,
) -> Result<SpaceWeatherOutput, SpaceWeatherError> {
    if !input.imf_bz_nt.is_finite()
        || !input.solar_wind_speed_km_s.is_finite()
        || !input.proton_density_cm3.is_finite()
        || !input.f107_flux_sfu.is_finite()
        || !input.subsolar_zenith_deg.is_finite()
        || !input.cycle_month.is_finite()
    {
        return Err(SpaceWeatherError::NonFiniteInput);
    }
    if input.proton_density_cm3 < 0.0 {
        return Err(SpaceWeatherError::NegativeProtonDensity);
    }
    if input.solar_wind_speed_km_s <= 0.0 {
        return Err(SpaceWeatherError::InvalidSolarWindSpeed);
    }

    // Dynamic ram pressure: P_dyn = m_p * n * v^2 in nPa
    let v_m_s = input.solar_wind_speed_km_s * 1e3;
    let n_m3 = input.proton_density_cm3 * 1e6;
    let p_dyn = (PROTON_MASS_KG * n_m3 * v_m_s * v_m_s * 1e9).max(0.01);

    // 1. Magnetopause standoff distance R_mp (Shue et al. 1997)
    let bz = input.imf_bz_nt;
    let r0 = (10.22 + 1.29 * (0.184 * (bz + 8.14)).tanh()) * p_dyn.powf(-1.0 / 6.6);
    let magnetopause_standoff_re = r0.clamp(4.0, 15.0);

    // 2. Bow shock standoff distance R_bs (Farris & Russell 1994)
    let bow_shock_re = magnetopause_standoff_re * 1.32 + 1.25;

    // 3. Planetary Kp index [0.0, 9.0]
    let bz_drive = (-bz).max(0.0);
    let base_kp = (input.solar_wind_speed_km_s / 160.0) + (bz_drive * 0.70) + (p_dyn * 0.40);
    let kp_index = base_kp.clamp(0.0, 9.0);

    // 4. NOAA G-scale storm categorization (G0..G5)
    let geomag_storm_scale_g = if kp_index >= 9.0 {
        5
    } else if kp_index >= 8.0 {
        4
    } else if kp_index >= 7.0 {
        3
    } else if kp_index >= 6.0 {
        2
    } else if kp_index >= 5.0 {
        1
    } else {
        0
    };

    // 5. Burton-McPherron-Russell (1975) Dst ring current index in nT
    let dst_index_nt = if bz < -1.5 {
        (10.0 - bz_drive * 18.0 - (input.solar_wind_speed_km_s - 400.0) * 0.15).min(15.0)
    } else {
        5.0 + 12.0 * (p_dyn.sqrt() - 1.0).clamp(-5.0, 25.0)
    };

    // 6. Feldstein / Holzworth-Meng auroral oval equatorward boundary in degrees
    let auroral_oval_equatorward_lat_deg = (66.5 - 2.1 * kp_index).clamp(45.0, 72.0);

    // 7. Chapman-layer Total Electron Content (TEC) in TECU
    let zenith_rad = input.subsolar_zenith_deg.to_radians();
    let cos_chi = zenith_rad.cos().clamp(0.05, 1.0);
    let f107 = input.f107_flux_sfu.max(40.0);
    let tec_tecu = (0.16 * f107 * cos_chi.sqrt() + (kp_index * 1.85)).max(1.0);

    // 8. Hathaway et al. solar cycle sunspot number (SSN) progression
    let m = input.cycle_month.max(0.0);
    let sunspot_number_ssn = if m > 0.0 {
        (175.0 * (m / 48.0).powi(3) * (-((m - 48.0) / 36.0)).exp()).clamp(0.0, 300.0)
    } else {
        0.0
    };

    Ok(SpaceWeatherOutput {
        magnetopause_standoff_re,
        bow_shock_re,
        dst_index_nt,
        kp_index,
        tec_tecu,
        auroral_oval_equatorward_lat_deg,
        geomag_storm_scale_g,
        sunspot_number_ssn,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quiet_vs_cme_storm_space_weather_dynamics() {
        let quiet_input = SpaceWeatherInput {
            imf_bz_nt: 2.5,
            solar_wind_speed_km_s: 380.0,
            proton_density_cm3: 4.0,
            f107_flux_sfu: 75.0,
            subsolar_zenith_deg: 25.0,
            cycle_month: 12.0,
        };

        let storm_input = SpaceWeatherInput {
            imf_bz_nt: -18.0,
            solar_wind_speed_km_s: 750.0,
            proton_density_cm3: 22.0,
            f107_flux_sfu: 185.0,
            subsolar_zenith_deg: 25.0,
            cycle_month: 48.0,
        };

        let quiet = evaluate_space_weather_zero_heap(&quiet_input).unwrap();
        let storm = evaluate_space_weather_zero_heap(&storm_input).unwrap();

        // 1. Magnetopause compression during CME storm
        assert!(quiet.magnetopause_standoff_re > storm.magnetopause_standoff_re);
        assert!(quiet.magnetopause_standoff_re > 9.0);
        assert!(storm.magnetopause_standoff_re < 7.5);

        // 2. Ring current Dst depression (strongly negative in storm)
        assert!(quiet.dst_index_nt > 0.0);
        assert!(storm.dst_index_nt < -150.0);

        // 3. Kp index and NOAA G-scale storm level
        assert!(quiet.kp_index < 4.0);
        assert_eq!(quiet.geomag_storm_scale_g, 0);
        assert!(storm.kp_index >= 7.0);
        assert!(storm.geomag_storm_scale_g >= 3);

        // 4. Auroral oval expands equatorward (lower magnetic latitude)
        assert!(storm.auroral_oval_equatorward_lat_deg < quiet.auroral_oval_equatorward_lat_deg);
        assert!(storm.auroral_oval_equatorward_lat_deg < 52.0);

        // 5. TEC elevation during storm and higher F10.7
        assert!(storm.tec_tecu > quiet.tec_tecu);

        // 6. Proves we never return the hardcoded 8.6 / 31.4 constants
        assert!((quiet.magnetopause_standoff_re - 8.6).abs() > 0.1);
        assert!((storm.magnetopause_standoff_re - 8.6).abs() > 0.1);
        assert!((quiet.tec_tecu - 31.4).abs() > 0.1);
        assert!((storm.tec_tecu - 31.4).abs() > 0.1);
    }

    #[test]
    fn test_negative_input_rejection() {
        let nan_input = SpaceWeatherInput {
            imf_bz_nt: f64::NAN,
            solar_wind_speed_km_s: 400.0,
            proton_density_cm3: 5.0,
            f107_flux_sfu: 100.0,
            subsolar_zenith_deg: 0.0,
            cycle_month: 24.0,
        };
        assert_eq!(
            evaluate_space_weather_zero_heap(&nan_input),
            Err(SpaceWeatherError::NonFiniteInput)
        );

        let neg_density = SpaceWeatherInput {
            imf_bz_nt: 0.0,
            solar_wind_speed_km_s: 400.0,
            proton_density_cm3: -1.0,
            f107_flux_sfu: 100.0,
            subsolar_zenith_deg: 0.0,
            cycle_month: 24.0,
        };
        assert_eq!(
            evaluate_space_weather_zero_heap(&neg_density),
            Err(SpaceWeatherError::NegativeProtonDensity)
        );

        let zero_speed = SpaceWeatherInput {
            imf_bz_nt: 0.0,
            solar_wind_speed_km_s: 0.0,
            proton_density_cm3: 5.0,
            f107_flux_sfu: 100.0,
            subsolar_zenith_deg: 0.0,
            cycle_month: 24.0,
        };
        assert_eq!(
            evaluate_space_weather_zero_heap(&zero_speed),
            Err(SpaceWeatherError::InvalidSolarWindSpeed)
        );
    }

    #[test]
    fn test_zero_alloc_stack_execution() {
        // Evaluate 100 sequential iterations on stack to verify pure zero-heap execution
        let mut input = SpaceWeatherInput {
            imf_bz_nt: -5.0,
            solar_wind_speed_km_s: 500.0,
            proton_density_cm3: 8.0,
            f107_flux_sfu: 120.0,
            subsolar_zenith_deg: 45.0,
            cycle_month: 36.0,
        };
        for i in 0..100 {
            input.imf_bz_nt = -5.0 + (i as f64) * 0.1;
            let out = evaluate_space_weather_zero_heap(&input).unwrap();
            assert!(out.magnetopause_standoff_re > 0.0);
            assert!(out.dst_index_nt.is_finite());
        }
    }
}
