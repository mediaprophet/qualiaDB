//! Specialized independent physics oracles for EMF Modes 3, 5, 6, 7 and QAC Modes 04, 05, 06, 07,
//! plus the C02-C08 cross-domain I/Q carrier convention bridge.
//!
//! Every oracle is evaluated from closed-form first-principles equations in independent `f64`
//! arithmetic without calling candidate solver implementations.

use crate::q42::science_validation::stokes_mueller::{
    scaled_hypot3, Complex64, VACUUM_SPEED_OF_LIGHT_M_PER_S,
};

/// Exact SI 2019 elementary charge $e = 1.602\,176\,634 \times 10^{-19}\text{ C}$ ($\text{J/eV}$).
pub const ELEMENTARY_CHARGE_J_PER_EV: f64 = 1.602_176_634e-19;

/// Haptic record role per QAC Schema 207 (C02 Mode 06 & C02-C07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HapticRole {
    Measurement,
    ReceptorModel,
    ActuatorCommand,
}

/// Structured rejection reasons for specialized physics oracles.
#[derive(Debug, Clone, PartialEq)]
pub enum SpecializedOracleError {
    /// Non-finite or out-of-domain physical parameter.
    InvalidParameter { name: &'static str, value: f64 },
    /// Attempted to infer absorbed dose from XCOM photon attenuation alone without absorbed
    /// energy/mass or a qualified energy-absorption transport model (C02 Mode 6 / EMF-A05).
    DoseFromAttenuationAloneForbidden,
    /// Attempted to equate absorbed dose (Gy) to equivalent dose (Sv) without an explicit
    /// radiation/tissue weighting model URI (C02 Mode 6).
    EquivalentDoseWithoutWeightingModelForbidden,
    /// Attempted to evaluate the point-dipole forward formula at or near the dipole source
    /// location $\mathbf{r} = \mathbf{r}_0$ (C02 Mode 7 / EMF-A06).
    SingularBioelectricDipoleAtSource { distance_m: f64 },
    /// Extracellular potential was claimed present without an explicit observation electrode
    /// position or reference gauge (C02 Mode 7 / EMF-A06).
    MissingBioelectricObservationReference,
    /// Haptic actuation command rejected due to non-command role, missing authority receipt,
    /// or expired timestamp $t \ge t_{\text{expiry}}$ (C02 Mode 06 & C02-C07).
    HapticCommandUnauthorizedOrExpired {
        role: HapticRole,
        current_tick: i128,
        expiry_tick: i128,
        has_authority_receipt: bool,
    },
    /// Attempted to infer ultrasound Thermal Index (TI) from MI or pressure when TI model URI
    /// is absent (C02 Mode 07 / AQ-05).
    ThermalIndexInferenceWithoutModelForbidden,
    /// Unconverted direct `memcpy` of $Q$ between EMF negative-carrier and QAC positive-carrier
    /// profiles is rejected (C02-C08).
    UnconvertedIqCarrierSignMismatch {
        reconstructed_emf: f64,
        reconstructed_qac: f64,
    },
    /// Attempted to apply a spectral density Jacobian to dimensionless reflectance (C02 Mode 3).
    ReflectanceJacobianForbidden,
}

/// Mode `0x0106` (EMF ionizing radiation) attenuation and dose oracle result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IonizingRadiationOracleResult {
    /// Linear attenuation coefficient $\mu = (\mu/\rho)\cdot\rho$ ($\text{m}^{-1}$).
    pub linear_attenuation_per_m: f64,
    /// Uncollided monoenergetic photon fluence $\Phi(x) = \Phi_0 e^{-\mu x}$ ($\text{m}^{-2}$).
    pub transmitted_fluence_per_m2: f64,
}

/// Evaluate C02 Mode 6 (`0x0106`) monoenergetic uncollided Beer-Lambert photon attenuation:
/// $\mu = (\mu/\rho)\rho$, $\Phi(x) = \Phi_0 e^{-\mu x}$.
///
/// Fixture: $\rho=1000\text{ kg/m}^3, \mu/\rho=0.01\text{ m}^2/\text{kg}, x=0.2\text{ m}, \Phi_0=100\text{ m}^{-2} \implies \mu=10\text{ m}^{-1}, \Phi(0.2)=100 e^{-2}=13.53352832366127\text{ m}^{-2}$.
pub fn evaluate_ionizing_beer_lambert_oracle(
    density_kg_per_m3: f64,
    mass_attenuation_m2_per_kg: f64,
    path_length_m: f64,
    incident_fluence_per_m2: f64,
    infer_dose_from_attenuation_only: bool,
) -> Result<IonizingRadiationOracleResult, SpecializedOracleError> {
    if infer_dose_from_attenuation_only {
        return Err(SpecializedOracleError::DoseFromAttenuationAloneForbidden);
    }
    if !density_kg_per_m3.is_finite() || density_kg_per_m3 <= 0.0 {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "density_kg_per_m3",
            value: density_kg_per_m3,
        });
    }
    if !mass_attenuation_m2_per_kg.is_finite() || mass_attenuation_m2_per_kg < 0.0 {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "mass_attenuation_m2_per_kg",
            value: mass_attenuation_m2_per_kg,
        });
    }
    if !path_length_m.is_finite() || path_length_m < 0.0 {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "path_length_m",
            value: path_length_m,
        });
    }
    if !incident_fluence_per_m2.is_finite() || incident_fluence_per_m2 < 0.0 {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "incident_fluence_per_m2",
            value: incident_fluence_per_m2,
        });
    }
    let mu = mass_attenuation_m2_per_kg * density_kg_per_m3;
    let transmitted = incident_fluence_per_m2 * (-mu * path_length_m).exp();
    Ok(IonizingRadiationOracleResult {
        linear_attenuation_per_m: mu,
        transmitted_fluence_per_m2: transmitted,
    })
}

/// Evaluate independent absorbed dose $D = \Delta E / \Delta m$ ($\text{Gy} = \text{J/kg}$)
/// from explicitly deposited energy and mass (C02 Mode 6).
///
/// Fixture: $\Delta E = 0.002\text{ J}, \Delta m = 0.01\text{ kg} \implies D = 0.2\text{ Gy}$.
pub fn evaluate_absorbed_dose_gy(
    deposited_energy_j: f64,
    mass_kg: f64,
) -> Result<f64, SpecializedOracleError> {
    if !deposited_energy_j.is_finite() || deposited_energy_j < 0.0 {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "deposited_energy_j",
            value: deposited_energy_j,
        });
    }
    if !mass_kg.is_finite() || mass_kg <= 0.0 {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "mass_kg",
            value: mass_kg,
        });
    }
    Ok(deposited_energy_j / mass_kg)
}

/// Convert photon energy in kiloelectron-volts ($\text{keV}$) to Joules ($\text{J}$):
/// $E\text{ [J]} = E\text{ [keV]} \times 10^3 \times 1.602\,176\,634\times 10^{-19}$.
///
/// Fixture: $100\text{ keV} = 1.602\,176\,634\times 10^{-14}\text{ J}$.
pub fn convert_kev_to_joules(energy_kev: f64) -> Result<f64, SpecializedOracleError> {
    if !energy_kev.is_finite() || energy_kev <= 0.0 {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "energy_kev",
            value: energy_kev,
        });
    }
    Ok(energy_kev * 1.0e3 * ELEMENTARY_CHARGE_J_PER_EV)
}

/// Verify equivalent dose ($\text{Sv}$) admission: requires a non-empty weighting model URI and
/// positive weighting factor $w_R > 0$.
pub fn evaluate_equivalent_dose_sv(
    absorbed_dose_gy: f64,
    radiation_weighting_factor: f64,
    weighting_model_uri: Option<&str>,
) -> Result<f64, SpecializedOracleError> {
    let uri = match weighting_model_uri {
        Some(u) if !u.trim().is_empty() => u,
        _ => return Err(SpecializedOracleError::EquivalentDoseWithoutWeightingModelForbidden),
    };
    let _ = uri;
    if !absorbed_dose_gy.is_finite()
        || absorbed_dose_gy < 0.0
        || !radiation_weighting_factor.is_finite()
        || radiation_weighting_factor <= 0.0
    {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "radiation_weighting_factor",
            value: radiation_weighting_factor,
        });
    }
    Ok(absorbed_dose_gy * radiation_weighting_factor)
}

/// Evaluate C02 Mode 7 (`0x0107`) quasi-static bioelectric current dipole forward potential
/// in a homogeneous infinite isotropic volume conductor of conductivity $\sigma > 0$:
///
/// $$\phi(\mathbf{r}) = \frac{\mathbf{q}\cdot(\mathbf{r} - \mathbf{r}_0)}{4\pi\sigma \|\mathbf{r} - \mathbf{r}_0\|^3}.$$
///
/// Strictly rejects evaluation at the source position ($\|\mathbf{r} - \mathbf{r}_0\| \le 10^{-15}\text{ m}$)
/// or when the observation reference electrode is absent.
///
/// Fixture: $\mathbf{q}=(0,0,10^{-8})\text{ A m}, \mathbf{r}_0=(0,0,0), \sigma=1\text{ S/m}, \mathbf{r}=(0,0,0.1)\text{ m} \implies \phi = 7.957747154594767\times 10^{-8}\text{ V}$.
pub fn evaluate_bioelectric_dipole_potential_v(
    dipole_moment_a_m: [f64; 3],
    source_position_m: [f64; 3],
    observation_position_m: Option<[f64; 3]>,
    has_reference_electrode: bool,
    conductivity_s_per_m: f64,
) -> Result<f64, SpecializedOracleError> {
    if !has_reference_electrode {
        return Err(SpecializedOracleError::MissingBioelectricObservationReference);
    }
    let obs = match observation_position_m {
        Some(p) => p,
        None => return Err(SpecializedOracleError::MissingBioelectricObservationReference),
    };
    for &v in dipole_moment_a_m
        .iter()
        .chain(source_position_m.iter())
        .chain(obs.iter())
    {
        if !v.is_finite() {
            return Err(SpecializedOracleError::InvalidParameter {
                name: "bioelectric_vector_component",
                value: v,
            });
        }
    }
    if !conductivity_s_per_m.is_finite() || conductivity_s_per_m <= 0.0 {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "conductivity_s_per_m",
            value: conductivity_s_per_m,
        });
    }
    let dr = [
        obs[0] - source_position_m[0],
        obs[1] - source_position_m[1],
        obs[2] - source_position_m[2],
    ];
    let dist = scaled_hypot3(dr[0], dr[1], dr[2]);
    if !dist.is_finite() || dist <= 1.0e-15 {
        return Err(SpecializedOracleError::SingularBioelectricDipoleAtSource {
            distance_m: dist,
        });
    }
    let q_dot_dr = dipole_moment_a_m[0] * dr[0]
        + dipole_moment_a_m[1] * dr[1]
        + dipole_moment_a_m[2] * dr[2];
    let denom = 4.0 * core::f64::consts::PI * conductivity_s_per_m * dist * dist * dist;
    Ok(q_dot_dr / denom)
}

/// Mode `0x0106` (QAC haptic mechanical) oracle result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HapticMechanicalOracleResult {
    /// Contact pressure $P = F_n / A$ ($\text{Pa}$).
    pub contact_pressure_pa: f64,
    /// Simulator linear Hooke restoring force $F = -k x$ ($\text{N}$).
    pub restoring_force_n: f64,
    /// Local force vector rotated into parent frame $\mathbf{f}_{\text{parent}} = \mathbf{R}\mathbf{f}_{\text{local}}$ ($\text{N}$).
    pub parent_force_n: [f64; 3],
}

/// Evaluate C02 QAC Mode 06 (`0x0106`) haptic mechanical oracle:
/// - Contact pressure $P = F_n / A$ ($2\text{ N} / 10^{-4}\text{ m}^2 = 20{,}000\text{ Pa}$).
/// - Linear spring restoring force $F = -k x$ ($k = 1000\text{ N/m}, x = 0.002\text{ m} \implies F = -2\text{ N}$).
/// - Active local-to-parent force vector rotation.
/// - Role and expiry authority gate when `request_actuation` is true.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_haptic_mechanical_oracle(
    normal_force_n: f64,
    contact_area_m2: f64,
    spring_stiffness_n_per_m: f64,
    displacement_m: f64,
    local_force_n: [f64; 3],
    parent_from_local_rot: &[[f64; 3]; 3],
    role: HapticRole,
    request_actuation: bool,
    has_authority_receipt: bool,
    current_tick: i128,
    expiry_tick: i128,
) -> Result<HapticMechanicalOracleResult, SpecializedOracleError> {
    if request_actuation {
        if role != HapticRole::ActuatorCommand
            || !has_authority_receipt
            || current_tick >= expiry_tick
        {
            return Err(SpecializedOracleError::HapticCommandUnauthorizedOrExpired {
                role,
                current_tick,
                expiry_tick,
                has_authority_receipt,
            });
        }
    }
    if !normal_force_n.is_finite()
        || !contact_area_m2.is_finite()
        || contact_area_m2 <= 0.0
        || !spring_stiffness_n_per_m.is_finite()
        || spring_stiffness_n_per_m < 0.0
        || !displacement_m.is_finite()
        || local_force_n.iter().any(|v| !v.is_finite())
    {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "haptic_parameter",
            value: contact_area_m2,
        });
    }
    let contact_pressure_pa = normal_force_n / contact_area_m2;
    let restoring_force_n = -spring_stiffness_n_per_m * displacement_m;
    let mut parent_force_n = [0.0f64; 3];
    for r in 0..3 {
        parent_force_n[r] = parent_from_local_rot[r][0] * local_force_n[0]
            + parent_from_local_rot[r][1] * local_force_n[1]
            + parent_from_local_rot[r][2] * local_force_n[2];
    }
    Ok(HapticMechanicalOracleResult {
        contact_pressure_pa,
        restoring_force_n,
        parent_force_n,
    })
}

/// Mode `0x0107` (QAC ultrasound / sonar) oracle result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UltrasoundOracleResult {
    /// Pulse-echo round-trip time of flight $\tau = 2R/c$ ($\text{s}$).
    pub round_trip_tof_s: f64,
    /// Exact sample delay $\tau \cdot f_s$ at acquisition rate $f_s$.
    pub round_trip_samples: f64,
    /// Derated Mechanical Index $\text{MI} = \frac{p_{r,\text{derated}}\text{ [MPa]}}{\sqrt{f_c\text{ [MHz]}}}$.
    pub mechanical_index: f64,
    /// Monostatic pulse-echo Doppler shift $\Delta f = +\frac{2 v_r f_c}{c}$ ($\text{Hz}$, positive toward transducer).
    pub doppler_shift_hz: f64,
    /// Validated externally reported Thermal Index (`None` when no TI model URI is declared).
    pub reported_thermal_index: Option<f64>,
}

/// Evaluate C02 QAC Mode 07 (`0x0107`) ultrasound pulse-echo time-of-flight, Mechanical Index (MI),
/// and monostatic Doppler oracle:
/// - $\tau = \frac{2R}{c}$ ($c=1500\text{ m/s}, R=0.015\text{ m} \implies \tau = 20\,\mu\text{s} = 400\text{ samples at } 20\text{ MHz}$).
/// - $\text{MI} = \frac{p_{r,\text{derated}}\text{ [MPa]}}{\sqrt{f_c\text{ [MHz]}}}$ ($1\text{ MPa}, 4\text{ MHz} \implies \text{MI} = 0.5$).
/// - $\Delta f = +\frac{2 v_r f_c}{c}$ ($v_r = +0.15\text{ m/s}, f_c = 2\text{ MHz}, c = 1500\text{ m/s} \implies +400\text{ Hz}$).
/// - Strict guard rejecting TI inference when `thermal_model_uri` is absent.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_ultrasound_mode07_oracle(
    sound_speed_m_per_s: f64,
    target_range_m: f64,
    rf_sample_rate_hz: f64,
    center_frequency_hz: f64,
    derated_rarefactional_pressure_pa: f64,
    toward_transducer_velocity_m_per_s: f64,
    reported_ti: Option<f64>,
    thermal_model_uri: Option<&str>,
    infer_ti_from_mi_or_pressure: bool,
) -> Result<UltrasoundOracleResult, SpecializedOracleError> {
    if infer_ti_from_mi_or_pressure {
        return Err(SpecializedOracleError::ThermalIndexInferenceWithoutModelForbidden);
    }
    let has_ti_model = matches!(thermal_model_uri, Some(u) if !u.trim().is_empty());
    if reported_ti.is_some() && !has_ti_model {
        return Err(SpecializedOracleError::ThermalIndexInferenceWithoutModelForbidden);
    }
    if !sound_speed_m_per_s.is_finite()
        || sound_speed_m_per_s <= 0.0
        || !target_range_m.is_finite()
        || target_range_m < 0.0
        || !rf_sample_rate_hz.is_finite()
        || rf_sample_rate_hz <= 0.0
        || !center_frequency_hz.is_finite()
        || center_frequency_hz <= 0.0
        || !derated_rarefactional_pressure_pa.is_finite()
        || derated_rarefactional_pressure_pa < 0.0
        || !toward_transducer_velocity_m_per_s.is_finite()
    {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "ultrasound_parameter",
            value: center_frequency_hz,
        });
    }

    let round_trip_tof_s = (2.0 * target_range_m) / sound_speed_m_per_s;
    let round_trip_samples = round_trip_tof_s * rf_sample_rate_hz;
    let pr_mpa = derated_rarefactional_pressure_pa * 1.0e-6;
    let fc_mhz = center_frequency_hz * 1.0e-6;
    let mechanical_index = pr_mpa / fc_mhz.sqrt();
    let doppler_shift_hz =
        (2.0 * toward_transducer_velocity_m_per_s * center_frequency_hz) / sound_speed_m_per_s;

    Ok(UltrasoundOracleResult {
        round_trip_tof_s,
        round_trip_samples,
        mechanical_index,
        doppler_shift_hz,
        reported_thermal_index: reported_ti,
    })
}

/// Convert ultrasound intensity in $\text{mW/cm}^2$ to SI $\text{W/m}^2$ ($1\text{ mW/cm}^2 = 10\text{ W/m}^2$).
pub fn convert_mw_per_cm2_to_w_per_m2(
    intensity_mw_per_cm2: f64,
) -> Result<f64, SpecializedOracleError> {
    if !intensity_mw_per_cm2.is_finite() || intensity_mw_per_cm2 < 0.0 {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "intensity_mw_per_cm2",
            value: intensity_mw_per_cm2,
        });
    }
    Ok(intensity_mw_per_cm2 * 10.0)
}

/// Reconstruct real RF signal under EMF Mode 5 negative-carrier convention (C02-C08):
/// $x_{\text{RF}}(t) = \operatorname{Re}\{(I + iQ) e^{-i(2\pi f_c(t - t_{\text{LO}}) + \phi_{\text{LO}})}\} = I\cos\theta + Q\sin\theta$.
#[inline]
pub fn reconstruct_emf_negative_carrier_rf(
    z_emf: Complex64,
    carrier_hz: f64,
    dt_from_lo_s: f64,
    lo_phase_rad: f64,
) -> f64 {
    let theta = 2.0 * core::f64::consts::PI * carrier_hz * dt_from_lo_s + lo_phase_rad;
    z_emf.re * theta.cos() + z_emf.im * theta.sin()
}

/// Reconstruct real acoustic/ultrasound waveform under QAC Mode 07 positive-carrier convention (C02-C08):
/// $p(t) = \operatorname{Re}\{(I + iQ) e^{+i(2\pi f_c(t - t_{\text{LO}}) + \phi_{\text{LO}})}\} = I\cos\theta - Q\sin\theta$.
#[inline]
pub fn reconstruct_qac_positive_carrier_rf(
    z_qac: Complex64,
    carrier_hz: f64,
    dt_from_lo_s: f64,
    lo_phase_rad: f64,
) -> f64 {
    let theta = 2.0 * core::f64::consts::PI * carrier_hz * dt_from_lo_s + lo_phase_rad;
    z_qac.re * theta.cos() - z_qac.im * theta.sin()
}

/// Convert complex baseband sample between EMF negative-carrier and QAC positive-carrier
/// profiles via complex conjugation $z_{\text{QAC}} = z_{\text{EMF}}^*$ (C02-C08), and verify that
/// reconstructed real waveforms agree within `64 * f64::EPSILON * |z|`.
pub fn convert_and_verify_iq_carrier_bridge(
    z_emf: Complex64,
    candidate_z_qac: Complex64,
    carrier_hz: f64,
    dt_from_lo_s: f64,
    lo_phase_rad: f64,
) -> Result<f64, SpecializedOracleError> {
    let emf_real =
        reconstruct_emf_negative_carrier_rf(z_emf, carrier_hz, dt_from_lo_s, lo_phase_rad);
    let qac_real = reconstruct_qac_positive_carrier_rf(
        candidate_z_qac,
        carrier_hz,
        dt_from_lo_s,
        lo_phase_rad,
    );
    let scale = z_emf.abs().max(1.0);
    let tol = 64.0 * f64::EPSILON * scale;
    if (emf_real - qac_real).abs() > tol {
        return Err(SpecializedOracleError::UnconvertedIqCarrierSignMismatch {
            reconstructed_emf: emf_real,
            reconstructed_qac: qac_real,
        });
    }
    Ok(emf_real)
}

/// Convert spectral irradiance density per Hz $F_f(f)$ ($\text{W m}^{-2}\text{ Hz}^{-1}$) to
/// spectral density per metre wavelength $F_\lambda(\lambda) = F_f(c/\lambda)\frac{c}{\lambda^2}$ ($\text{W m}^{-3}$)
/// at vacuum wavelength $\lambda = c_0 / f$ (C02 Mode 3).
///
/// Rejects applying the density Jacobian when `is_dimensionless_reflectance` is true.
pub fn convert_spectral_density_hz_to_wavelength_m(
    f_hz: f64,
    density_per_hz: f64,
    is_dimensionless_reflectance: bool,
) -> Result<(f64, f64), SpecializedOracleError> {
    if is_dimensionless_reflectance {
        return Err(SpecializedOracleError::ReflectanceJacobianForbidden);
    }
    if !f_hz.is_finite() || f_hz <= 0.0 || !density_per_hz.is_finite() {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "f_hz",
            value: f_hz,
        });
    }
    let lambda_m = VACUUM_SPEED_OF_LIGHT_M_PER_S / f_hz;
    let jacobian = VACUUM_SPEED_OF_LIGHT_M_PER_S / (lambda_m * lambda_m);
    Ok((lambda_m, density_per_hz * jacobian))
}

/// Evaluate QAC Mode 04 (`0x0104`) linear damped modal oscillator:
/// $y(t) = A e^{-d t} \sin(2\pi f t + \phi)$ and amplitude $T_{60} = \frac{\ln(1000)}{d}$
/// (returning `None` for $T_{60}$ when $d = 0$, never $+\infty$).
pub fn evaluate_qac_modal_oscillator_oracle(
    amplitude_pa: f64,
    frequency_hz: f64,
    decay_rate_per_s: f64,
    phase_rad: f64,
    time_s: f64,
) -> Result<(f64, Option<f64>), SpecializedOracleError> {
    if !amplitude_pa.is_finite()
        || !frequency_hz.is_finite()
        || frequency_hz <= 0.0
        || !decay_rate_per_s.is_finite()
        || decay_rate_per_s < 0.0
        || !phase_rad.is_finite()
        || !time_s.is_finite()
        || time_s < 0.0
    {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "modal_parameter",
            value: decay_rate_per_s,
        });
    }
    let sample = amplitude_pa
        * (-decay_rate_per_s * time_s).exp()
        * (2.0 * core::f64::consts::PI * frequency_hz * time_s + phase_rad).sin();
    let t60 = if decay_rate_per_s == 0.0 {
        None
    } else {
        Some(1000.0f64.ln() / decay_rate_per_s)
    };
    Ok((sample, t60))
}

/// Evaluate QAC Mode 05 (`0x0105`) analytical dB-SPL from RMS pressure $p_{\text{rms}}$ and
/// reference pressure $p_{\text{ref}} > 0$:
/// $\text{SPL} = 20 \log_{10}(p_{\text{rms}} / p_{\text{ref}})$.
///
/// For $p_{\text{rms}} == 0$, returns `Ok(None)` (representing `silence` status, never $-\infty$).
pub fn evaluate_qac_db_spl_oracle(
    rms_pressure_pa: f64,
    reference_pressure_pa: f64,
) -> Result<Option<f64>, SpecializedOracleError> {
    if !rms_pressure_pa.is_finite()
        || rms_pressure_pa < 0.0
        || !reference_pressure_pa.is_finite()
        || reference_pressure_pa <= 0.0
    {
        return Err(SpecializedOracleError::InvalidParameter {
            name: "rms_or_reference_pressure_pa",
            value: rms_pressure_pa,
        });
    }
    if rms_pressure_pa == 0.0 {
        return Ok(None);
    }
    Ok(Some(20.0 * (rms_pressure_pa / reference_pressure_pa).log10()))
}
