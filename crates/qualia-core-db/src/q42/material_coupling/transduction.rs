//! Cross-domain multipole & transduction coupling:
//! 1. Photoacoustic thermoelastic transduction ($\Gamma = \frac{\beta c_s^2}{C_p}$, $p_0 = \Gamma \mu_a \Phi$, thermal & stress confinement).
//! 2. Piezoelectric / electro-elastic coupling ($D_i = \varepsilon_{ij}^S E_j + e_{im} S_m$, $T_m = c_{mn}^E S_n - e_{km} E_k$, $k^2 < 1$ passivity).
//! 3. Duhamel-Neumann thermo-elastic stress & coupled radar/sonar boundary scattering.
//! 4. C02-C08 explicit EMF ($\exp(-i\omega t)$) $\leftrightarrow$ QAC ($\exp(+i\omega t)$) I/Q carrier conjugation.

use std::f64::consts::PI;

use super::acoustic_coupling::{
    evaluate_planar_interface_scattering, AcousticCouplingError, CauchyStress6, ContinuumMedium,
    InterfaceScatteringCoefficients, TensorStrain6,
};
use super::constitutive::{Complex64, ConstitutiveError, ConstitutiveModel};
use super::dispersion::{evaluate_wave_propagation, DispersionError};

/// Errors arising during cross-domain transduction or carrier conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransductionError {
    NonFiniteParameter,
    InvalidPhotoacousticParameter,
    StressConfinementViolated {
        pulse_duration_s: f64,
        stress_relaxation_s: f64,
    },
    ThermalConfinementViolated {
        pulse_duration_s: f64,
        thermal_diffusion_s: f64,
    },
    ThermodynamicEnergyBoundExceeded {
        acoustic_energy_density_j_m3: f64,
        absorbed_optical_energy_density_j_m3: f64,
    },
    InvalidPiezoelectricStiffnessOrPermittivity,
    ElectromechanicalCouplingUnstable {
        k_squared: f64,
    },
    InvalidThermoElasticParameter,
    RawMemcpyBetweenOppositeCarrierConventionsForbidden,
    Acoustic(AcousticCouplingError),
    Constitutive(ConstitutiveError),
    Dispersion(DispersionError),
}

impl From<AcousticCouplingError> for TransductionError {
    fn from(e: AcousticCouplingError) -> Self {
        Self::Acoustic(e)
    }
}

impl From<ConstitutiveError> for TransductionError {
    fn from(e: ConstitutiveError) -> Self {
        Self::Constitutive(e)
    }
}

impl From<DispersionError> for TransductionError {
    fn from(e: DispersionError) -> Self {
        Self::Dispersion(e)
    }
}

/// Photoacoustic medium parameters in strict SI units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhotoacousticParameters {
    /// Optical absorption coefficient $\mu_a > 0$ in $\text{m}^{-1}$.
    pub optical_absorption_per_m: f64,
    /// Volumetric thermal expansion coefficient $\beta > 0$ in $\text{K}^{-1}$.
    pub thermal_expansion_per_k: f64,
    /// Isobaric specific heat capacity $C_p > 0$ in $\text{J}\cdot\text{kg}^{-1}\cdot\text{K}^{-1}$.
    pub specific_heat_cp_j_per_kg_k: f64,
    /// Mass density $\rho > 0$ in $\text{kg}\cdot\text{m}^{-3}$.
    pub density_kg_per_m3: f64,
    /// Acoustic speed of sound $c_s > 0$ in $\text{m}\cdot\text{s}^{-1}$.
    pub sound_speed_m_per_s: f64,
    /// Thermal diffusivity $D_T > 0$ in $\text{m}^2\cdot\text{s}^{-1}$.
    pub thermal_diffusivity_m2_per_s: f64,
    /// Characteristic heated spatial dimension $d_c > 0$ in $\text{m}$ (typically $\min(\text{beam diameter}, 1/\mu_a)$).
    pub characteristic_length_m: f64,
}

/// Evaluated photoacoustic state and confinement diagnostics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhotoacousticResponse {
    /// Dimensionless Grüneisen parameter $\Gamma = \frac{\beta c_s^2}{C_p}$.
    pub gruneisen_parameter: f64,
    /// Initial thermoelastic pressure rise $p_0 = \Gamma \mu_a \Phi$ in $\text{Pa}$.
    pub initial_pressure_pa: f64,
    /// Absorbed volumetric optical energy density $q_{\text{abs}} = \mu_a \Phi$ in $\text{J}\cdot\text{m}^{-3}$.
    pub absorbed_energy_density_j_per_m3: f64,
    /// Initial local temperature rise $\Delta T = \frac{\mu_a \Phi}{\rho C_p}$ in $\text{K}$.
    pub temperature_rise_k: f64,
    /// Initial stored acoustic mechanical potential energy density $u_{\text{ac},0} = \frac{p_0^2}{2\rho c_s^2}$ in $\text{J}\cdot\text{m}^{-3}$.
    pub acoustic_energy_density_j_per_m3: f64,
    /// Stress confinement time $\tau_{\text{st}} = \frac{d_c}{c_s}$ in $\text{s}$.
    pub stress_confinement_time_s: f64,
    /// Thermal confinement time $\tau_{\text{th}} = \frac{d_c^2}{4 D_T}$ in $\text{s}$.
    pub thermal_confinement_time_s: f64,
}

impl PhotoacousticParameters {
    pub fn validate(&self) -> Result<(), TransductionError> {
        let vals = [
            self.optical_absorption_per_m,
            self.thermal_expansion_per_k,
            self.specific_heat_cp_j_per_kg_k,
            self.density_kg_per_m3,
            self.sound_speed_m_per_s,
            self.thermal_diffusivity_m2_per_s,
            self.characteristic_length_m,
        ];
        if vals.iter().any(|v| !v.is_finite()) {
            return Err(TransductionError::NonFiniteParameter);
        }
        if vals.iter().any(|&v| v <= 0.0) {
            return Err(TransductionError::InvalidPhotoacousticParameter);
        }
        Ok(())
    }

    /// Dimensionless Grüneisen parameter $\Gamma = \frac{\beta c_s^2}{C_p}$.
    #[inline]
    pub fn gruneisen_parameter(&self) -> f64 {
        (self.thermal_expansion_per_k * self.sound_speed_m_per_s * self.sound_speed_m_per_s)
            / self.specific_heat_cp_j_per_kg_k
    }

    /// Evaluate photoacoustic pressure generation $p_0 = \Gamma \mu_a \Phi$ from optical fluence $\Phi$ ($\text{J}\cdot\text{m}^{-2}$)
    /// and verify both stress confinement ($\tau_p < \tau_{\text{st}}$) and thermal confinement ($\tau_p < \tau_{\text{th}}$)
    /// as well as thermodynamic energy conservation ($u_{\text{ac},0} \le q_{\text{abs}}$).
    pub fn evaluate_confined_excitation(
        &self,
        optical_fluence_j_per_m2: f64,
        laser_pulse_duration_s: f64,
    ) -> Result<PhotoacousticResponse, TransductionError> {
        self.validate()?;
        if !optical_fluence_j_per_m2.is_finite() || !laser_pulse_duration_s.is_finite() {
            return Err(TransductionError::NonFiniteParameter);
        }
        if optical_fluence_j_per_m2 < 0.0 || laser_pulse_duration_s <= 0.0 {
            return Err(TransductionError::InvalidPhotoacousticParameter);
        }
        let tau_st = self.characteristic_length_m / self.sound_speed_m_per_s;
        let tau_th = (self.characteristic_length_m * self.characteristic_length_m)
            / (4.0 * self.thermal_diffusivity_m2_per_s);

        if laser_pulse_duration_s >= tau_st {
            return Err(TransductionError::StressConfinementViolated {
                pulse_duration_s: laser_pulse_duration_s,
                stress_relaxation_s: tau_st,
            });
        }
        if laser_pulse_duration_s >= tau_th {
            return Err(TransductionError::ThermalConfinementViolated {
                pulse_duration_s: laser_pulse_duration_s,
                thermal_diffusion_s: tau_th,
            });
        }

        let gamma = self.gruneisen_parameter();
        let q_abs = self.optical_absorption_per_m * optical_fluence_j_per_m2;
        let p0 = gamma * q_abs;
        let delta_t = q_abs / (self.density_kg_per_m3 * self.specific_heat_cp_j_per_kg_k);
        let k_bulk = self.density_kg_per_m3 * self.sound_speed_m_per_s * self.sound_speed_m_per_s;
        let u_ac = (p0 * p0) / (2.0 * k_bulk);

        if u_ac > q_abs + 1e-15 {
            return Err(TransductionError::ThermodynamicEnergyBoundExceeded {
                acoustic_energy_density_j_m3: u_ac,
                absorbed_optical_energy_density_j_m3: q_abs,
            });
        }

        Ok(PhotoacousticResponse {
            gruneisen_parameter: gamma,
            initial_pressure_pa: p0,
            absorbed_energy_density_j_per_m3: q_abs,
            temperature_rise_k: delta_t,
            acoustic_energy_density_j_per_m3: u_ac,
            stress_confinement_time_s: tau_st,
            thermal_confinement_time_s: tau_th,
        })
    }
}

/// Piezoelectric / electro-elastic constitutive tensor model (stress-charge form in Voigt order
/// `[xx, yy, zz, xy, yz, xz]` using engineering shear strain $S_4 = 2\varepsilon_{xy}, S_5 = 2\varepsilon_{yz}, S_6 = 2\varepsilon_{xz}$):
/// $$D_i = \varepsilon_{ij}^S E_j + e_{im} S_m \quad (\text{C}\cdot\text{m}^{-2})$$
/// $$T_m = c_{mn}^E S_n - e_{km} E_k \quad (\text{Pa})$$
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PiezoelectricConstitutiveModel {
    /// Constant-electric-field elastic stiffness $c_{mn}^E$ ($6\times 6$ symmetric positive-definite, in $\text{Pa}$).
    pub stiffness_c_e_pa: [[f64; 6]; 6],
    /// Clamped (constant-strain) electric permittivity $\varepsilon_{ij}^S$ ($3\times 3$ symmetric positive-definite, in $\text{F}\cdot\text{m}^{-1}$).
    pub permittivity_eps_s_f_per_m: [[f64; 3]; 3],
    /// Piezoelectric stress coupling tensor $e_{im}$ ($3\times 6$, in $\text{C}\cdot\text{m}^{-2}$).
    pub piezo_e_c_per_m2: [[f64; 6]; 3],
}

/// Coupled piezoelectric stress, electric displacement, and internal energy state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PiezoelectricState {
    /// Cauchy stress $T_m$ (`[xx, yy, zz, xy, yz, xz]`) in $\text{Pa}$.
    pub stress_pa: CauchyStress6,
    /// Electric displacement vector $\mathbf{D}$ in $\text{C}\cdot\text{m}^{-2}$.
    pub electric_displacement_c_per_m2: [f64; 3],
    /// Mechanical elastic energy density $\frac{1}{2} S_m c_{mn}^E S_n$ in $\text{J}\cdot\text{m}^{-3}$.
    pub mechanical_energy_j_per_m3: f64,
    /// Clamped dielectric energy density $\frac{1}{2} E_i \varepsilon_{ij}^S E_j$ in $\text{J}\cdot\text{m}^{-3}$.
    pub dielectric_energy_j_per_m3: f64,
    /// Total internal electro-elastic energy density $U = \frac{1}{2}(T_m S_m + D_i E_i)$ in $\text{J}\cdot\text{m}^{-3}$.
    pub total_internal_energy_j_per_m3: f64,
}

impl PiezoelectricConstitutiveModel {
    /// Validate symmetry, positive diagonals, and axial electromechanical coupling factor $k_{im}^2 = \frac{e_{im}^2}{c_{mm}^E \varepsilon_{ii}^S} < 1$.
    pub fn validate(&self) -> Result<f64, TransductionError> {
        for r in &self.stiffness_c_e_pa {
            if r.iter().any(|v| !v.is_finite()) {
                return Err(TransductionError::NonFiniteParameter);
            }
        }
        for r in &self.permittivity_eps_s_f_per_m {
            if r.iter().any(|v| !v.is_finite()) {
                return Err(TransductionError::NonFiniteParameter);
            }
        }
        for r in &self.piezo_e_c_per_m2 {
            if r.iter().any(|v| !v.is_finite()) {
                return Err(TransductionError::NonFiniteParameter);
            }
        }

        for m in 0..6 {
            if self.stiffness_c_e_pa[m][m] <= 0.0 {
                return Err(TransductionError::InvalidPiezoelectricStiffnessOrPermittivity);
            }
            for n in (m + 1)..6 {
                if (self.stiffness_c_e_pa[m][n] - self.stiffness_c_e_pa[n][m]).abs() > 1e-6 {
                    return Err(TransductionError::InvalidPiezoelectricStiffnessOrPermittivity);
                }
            }
        }
        for i in 0..3 {
            if self.permittivity_eps_s_f_per_m[i][i] <= 0.0 {
                return Err(TransductionError::InvalidPiezoelectricStiffnessOrPermittivity);
            }
            for j in (i + 1)..3 {
                if (self.permittivity_eps_s_f_per_m[i][j] - self.permittivity_eps_s_f_per_m[j][i]).abs()
                    > 1e-20
                {
                    return Err(TransductionError::InvalidPiezoelectricStiffnessOrPermittivity);
                }
            }
        }

        let mut max_k_sq = 0.0f64;
        for i in 0..3 {
            for m in 0..6 {
                let e_im = self.piezo_e_c_per_m2[i][m];
                let k_sq = (e_im * e_im)
                    / (self.stiffness_c_e_pa[m][m] * self.permittivity_eps_s_f_per_m[i][i]);
                if k_sq >= 1.0 {
                    return Err(TransductionError::ElectromechanicalCouplingUnstable {
                        k_squared: k_sq,
                    });
                }
                max_k_sq = max_k_sq.max(k_sq);
            }
        }
        Ok(max_k_sq)
    }

    /// Evaluate coupled stress $T_m$, electric displacement $D_i$, and internal energy density $U$
    /// from tensor strain $\varepsilon_{ij}$ (converted to Voigt engineering shear $S_4 = 2\varepsilon_{xy}, S_5 = 2\varepsilon_{yz}, S_6 = 2\varepsilon_{xz}$)
    /// and electric field $\mathbf{E}$ ($\text{V}\cdot\text{m}^{-1}$).
    pub fn evaluate_coupled_state(
        &self,
        strain: &TensorStrain6,
        electric_field_v_per_m: [f64; 3],
    ) -> Result<PiezoelectricState, TransductionError> {
        self.validate()?;
        if strain.xx_yy_zz_xy_yz_xz.iter().any(|v| !v.is_finite())
            || electric_field_v_per_m.iter().any(|v| !v.is_finite())
        {
            return Err(TransductionError::NonFiniteParameter);
        }
        // Convert tensor shear to Voigt engineering strain S_m
        let [exx, eyy, ezz, exy, eyz, exz] = strain.xx_yy_zz_xy_yz_xz;
        let s = [exx, eyy, ezz, 2.0 * exy, 2.0 * eyz, 2.0 * exz];

        let mut t = [0.0; 6];
        for m in 0..6 {
            let mut sum_c = 0.0;
            for n in 0..6 {
                sum_c += self.stiffness_c_e_pa[m][n] * s[n];
            }
            let mut sum_e = 0.0;
            for k in 0..3 {
                sum_e += self.piezo_e_c_per_m2[k][m] * electric_field_v_per_m[k];
            }
            t[m] = sum_c - sum_e;
        }

        let mut d = [0.0; 3];
        for i in 0..3 {
            let mut sum_eps = 0.0;
            for j in 0..3 {
                sum_eps += self.permittivity_eps_s_f_per_m[i][j] * electric_field_v_per_m[j];
            }
            let mut sum_e = 0.0;
            for m in 0..6 {
                sum_e += self.piezo_e_c_per_m2[i][m] * s[m];
            }
            d[i] = sum_eps + sum_e;
        }

        let mut u_mech = 0.0;
        for m in 0..6 {
            for n in 0..6 {
                u_mech += 0.5 * s[m] * self.stiffness_c_e_pa[m][n] * s[n];
            }
        }
        let mut u_elec = 0.0;
        for i in 0..3 {
            for j in 0..3 {
                u_elec +=
                    0.5 * electric_field_v_per_m[i] * self.permittivity_eps_s_f_per_m[i][j] * electric_field_v_per_m[j];
            }
        }
        // Verify thermodynamic identity: 0.5 * (T_m S_m + D_i E_i) == u_mech + u_elec
        let mut u_total = 0.0;
        for m in 0..6 {
            u_total += 0.5 * t[m] * s[m];
        }
        for i in 0..3 {
            u_total += 0.5 * d[i] * electric_field_v_per_m[i];
        }

        Ok(PiezoelectricState {
            stress_pa: CauchyStress6 {
                xx_yy_zz_xy_yz_xz_pa: t,
            },
            electric_displacement_c_per_m2: d,
            mechanical_energy_j_per_m3: u_mech,
            dielectric_energy_j_per_m3: u_elec,
            total_internal_energy_j_per_m3: u_total,
        })
    }

    /// Compute open-circuit ($D = 0$) piezoelectrically stiffened axial modulus
    /// $c_{mm}^D = c_{mm}^E + \frac{e_{im}^2}{\varepsilon_{ii}^S}$ in $\text{Pa}$.
    pub fn open_circuit_stiffened_modulus_pa(&self, e_axis: usize, voigt_m: usize) -> f64 {
        let e_im = self.piezo_e_c_per_m2[e_axis][voigt_m];
        self.stiffness_c_e_pa[voigt_m][voigt_m]
            + (e_im * e_im) / self.permittivity_eps_s_f_per_m[e_axis][e_axis]
    }
}

/// Evaluate Duhamel-Neumann isotropic thermo-elastic Cauchy stress:
/// $\sigma_{ij} = \lambda \operatorname{tr}(\varepsilon)\delta_{ij} + 2\mu \varepsilon_{ij} - 3 K \alpha_L \Delta T \delta_{ij}$,
/// where $\alpha_L$ is the linear thermal expansion coefficient ($\text{K}^{-1}$) and $\Delta T = T - T_0$ ($\text{K}$).
pub fn evaluate_thermoelastic_stress(
    medium: &ContinuumMedium,
    strain: &TensorStrain6,
    linear_thermal_expansion_per_k: f64,
    delta_temperature_k: f64,
) -> Result<CauchyStress6, TransductionError> {
    medium.validate()?;
    if !linear_thermal_expansion_per_k.is_finite()
        || !delta_temperature_k.is_finite()
        || strain.xx_yy_zz_xy_yz_xz.iter().any(|v| !v.is_finite())
    {
        return Err(TransductionError::NonFiniteParameter);
    }
    if linear_thermal_expansion_per_k < 0.0 {
        return Err(TransductionError::InvalidThermoElasticParameter);
    }
    let mut stress = CauchyStress6::from_isotropic_strain(medium, strain);
    let thermal_pressure_pa = 3.0 * medium.bulk_modulus_pa * linear_thermal_expansion_per_k * delta_temperature_k;
    stress.xx_yy_zz_xy_yz_xz_pa[0] -= thermal_pressure_pa;
    stress.xx_yy_zz_xy_yz_xz_pa[1] -= thermal_pressure_pa;
    stress.xx_yy_zz_xy_yz_xz_pa[2] -= thermal_pressure_pa;
    Ok(stress)
}

/// Co-located electromagnetic (radar/GPR) and acoustic (sonar/seismic) normal-incidence
/// interface scattering summary across a shared material boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadarSonarInterfaceSummary {
    pub em_reflection_amplitude: Complex64,
    pub em_transmission_amplitude: Complex64,
    pub em_power_reflection: f64,
    pub em_power_transmission: f64,
    pub acoustic: InterfaceScatteringCoefficients,
}

/// Evaluate normal-incidence radar (EM Fresnel) and sonar (acoustic) scattering across the
/// boundary from `(em_1, ac_1)` to `(em_2, ac_2)`.
pub fn evaluate_radar_sonar_normal_interface(
    em_1: &ConstitutiveModel,
    ac_1: &ContinuumMedium,
    em_2: &ConstitutiveModel,
    ac_2: &ContinuumMedium,
    em_omega_rad_s: f64,
) -> Result<RadarSonarInterfaceSummary, TransductionError> {
    let prop_1 = evaluate_wave_propagation(em_1, em_omega_rad_s)?;
    let prop_2 = evaluate_wave_propagation(em_2, em_omega_rad_s)?;
    let z1 = prop_1.complex_impedance_ohms;
    let z2 = prop_2.complex_impedance_ohms;
    let sum_z = z2.add(z1);
    let r_em = z2
        .sub(z1)
        .div(sum_z)
        .ok_or(TransductionError::NonFiniteParameter)?;
    let t_em = z2
        .scale(2.0)
        .div(sum_z)
        .ok_or(TransductionError::NonFiniteParameter)?;

    let r_pow = r_em.norm_sq();
    // Active power transmission for lossless/low-loss normal incidence: Re(1/Z2*) / Re(1/Z1*) |t|^2
    let inv_z1_re = z1.re / z1.norm_sq();
    let inv_z2_re = z2.re / z2.norm_sq();
    let t_pow = if inv_z1_re > 0.0 {
        (inv_z2_re / inv_z1_re) * t_em.norm_sq()
    } else {
        0.0
    };

    let acoustic = evaluate_planar_interface_scattering(ac_1, ac_2, 0.0)?;

    Ok(RadarSonarInterfaceSummary {
        em_reflection_amplitude: r_em,
        em_transmission_amplitude: t_em,
        em_power_reflection: r_pow,
        em_power_transmission: t_pow,
        acoustic,
    })
}

/// Explicit I/Q complex baseband carrier convention per C02-C08:
/// - `EmfNegativeCarrier`: $x_{\text{RF}}(t) = \operatorname{Re}\left((I + iQ)\exp(-i(2\pi f_c t + \phi_0))\right)$
/// - `QacPositiveCarrier`: $p_{\text{RF}}(t) = \operatorname{Re}\left((I + iQ)\exp(+i(2\pi f_c t + \phi_0))\right)$
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IqCarrierConvention {
    EmfNegativeCarrier,
    QacPositiveCarrier,
}

/// Reconstruct the real physical passband waveform at elapsed time `dt_s` ($t - t_{\text{ref}}$)
/// under the declared `IqCarrierConvention`.
#[inline]
pub fn reconstruct_real_passband_sample(
    iq: Complex64,
    carrier_hz: f64,
    reference_phase_rad: f64,
    dt_s: f64,
    convention: IqCarrierConvention,
) -> f64 {
    let theta = 2.0 * PI * carrier_hz * dt_s + reference_phase_rad;
    let (sin_t, cos_t) = theta.sin_cos();
    match convention {
        // Re((I + iQ)(cos(theta) - i sin(theta))) = I cos(theta) + Q sin(theta)
        IqCarrierConvention::EmfNegativeCarrier => iq.re * cos_t + iq.im * sin_t,
        // Re((I + iQ)(cos(theta) + i sin(theta))) = I cos(theta) - Q sin(theta)
        IqCarrierConvention::QacPositiveCarrier => iq.re * cos_t - iq.im * sin_t,
    }
}

/// Convert a complex I/Q sample from `source_convention` (with phase origin `source_phase_rad`)
/// to `target_convention` (with phase origin `target_phase_rad`) per C02-C08 so that both
/// reconstruct the exact same real physical passband waveform at every instant $t$.
pub fn convert_iq_carrier_convention(
    iq: Complex64,
    source_convention: IqCarrierConvention,
    source_phase_rad: f64,
    target_convention: IqCarrierConvention,
    target_phase_rad: f64,
) -> Result<Complex64, TransductionError> {
    if !iq.is_finite() || !source_phase_rad.is_finite() || !target_phase_rad.is_finite() {
        return Err(TransductionError::NonFiniteParameter);
    }
    // First align to canonical zero-phase EMF representation z_0 = z_emf * exp(-i phi_src)
    let (sin_s, cos_s) = source_phase_rad.sin_cos();
    let z_emf_zero = match source_convention {
        IqCarrierConvention::EmfNegativeCarrier => {
            iq.mul(Complex64::new(cos_s, -sin_s))
        }
        IqCarrierConvention::QacPositiveCarrier => {
            iq.conj().mul(Complex64::new(cos_s, -sin_s))
        }
    };
    let (sin_t, cos_t) = target_phase_rad.sin_cos();
    let out = match target_convention {
        IqCarrierConvention::EmfNegativeCarrier => {
            z_emf_zero.mul(Complex64::new(cos_t, sin_t))
        }
        IqCarrierConvention::QacPositiveCarrier => {
            z_emf_zero.mul(Complex64::new(cos_t, sin_t)).conj()
        }
    };
    Ok(out)
}
