//! Acoustic & continuum elastic material coupling (QAC Schema 200, Modes 0x0000/0x0001/0x0103,
//! `.m64` constituent LUT & strata records, and C02-C06 namespace separation).

use std::f64::consts::PI;

use super::constitutive::ActiveHamiltonRotation;
use crate::q42::m64_matter::{M64StrataColumnRecord, Q42ConstituentLutEntry};

/// Errors arising during acoustic or elastic medium admission, wave evaluation, or namespace resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AcousticCouplingError {
    NonFiniteParameter,
    InvalidDensity { rho_kg_per_m3: f64 },
    InvalidBulkModulus { bulk_modulus_pa: f64 },
    InvalidShearModulus { shear_modulus_pa: f64 },
    SolidRequiresPositiveShearModulus,
    FluidRequiresZeroShearModulus { shear_modulus_pa: f64 },
    EmfMaterialZeroUnbound,
    EmfVacuumCannotPropagateMechanicalWave,
    QacUniformMediumNotDeclared,
    MaterialEntryOutOfBounds { index: u32, len: usize },
    InvalidIncidenceAngle { theta_rad: f64 },
    InvalidAttenuationExponent { y: f64 },
    InvalidFrequency { omega_rad_s: f64 },
    NonPositiveDefiniteOrthotropicStiffness,
}

/// Explicit medium classification for QAC Schema 200 (`Medium`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediumKind {
    /// Inviscid or acoustic fluid ($\mu = 0$ exactly).
    Fluid,
    /// Isotropic linear elastic solid ($\mu > 0$; negative Lamé $\lambda$ permitted if $K > 0, \mu > 0$).
    IsotropicSolid,
}

/// Validated fluid or isotropic elastic continuum medium (QAC Schema 200).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContinuumMedium {
    pub kind: MediumKind,
    /// Mass density $\rho > 0$ in $\text{kg}\cdot\text{m}^{-3}$.
    pub density_kg_per_m3: f64,
    /// Adiabatic bulk modulus $K > 0$ in $\text{Pa}$.
    pub bulk_modulus_pa: f64,
    /// Shear modulus $\mu \ge 0$ in $\text{Pa}$ ($\mu = 0$ for `Fluid`, $\mu > 0$ for `IsotropicSolid`).
    pub shear_modulus_pa: f64,
    /// Ambient static pressure $p_0$ in $\text{Pa}$.
    pub ambient_pressure_pa: f64,
    /// Stationary background flow velocity $\mathbf{v}_0$ in $\text{m}\cdot\text{s}^{-1}$ (zero for initial stationary solvers).
    pub flow_velocity_m_per_s: [f64; 3],
}

impl ContinuumMedium {
    /// Admit a fluid medium ($\rho > 0, K > 0, \mu = 0$). Rejects $\rho \le 0$ (vacuum is never $\rho = 0$).
    pub fn new_fluid(
        density_kg_per_m3: f64,
        bulk_modulus_pa: f64,
        ambient_pressure_pa: f64,
    ) -> Result<Self, AcousticCouplingError> {
        let m = Self {
            kind: MediumKind::Fluid,
            density_kg_per_m3,
            bulk_modulus_pa,
            shear_modulus_pa: 0.0,
            ambient_pressure_pa,
            flow_velocity_m_per_s: [0.0; 3],
        };
        m.validate()?;
        Ok(m)
    }

    /// Admit an isotropic elastic solid medium ($\rho > 0, K > 0, \mu > 0$).
    /// Note per C02-C06: negative Lamé $\lambda = K - \frac{2}{3}\mu$ is admissible when $K > 0$ and $\mu > 0$.
    pub fn new_isotropic_solid(
        density_kg_per_m3: f64,
        bulk_modulus_pa: f64,
        shear_modulus_pa: f64,
        ambient_pressure_pa: f64,
    ) -> Result<Self, AcousticCouplingError> {
        let m = Self {
            kind: MediumKind::IsotropicSolid,
            density_kg_per_m3,
            bulk_modulus_pa,
            shear_modulus_pa,
            ambient_pressure_pa,
            flow_velocity_m_per_s: [0.0; 3],
        };
        m.validate()?;
        Ok(m)
    }

    /// Construct from an `.m64` `Q42ConstituentLutEntry`.
    /// Converts Young's modulus $E$ ($\text{MPa}$) and Poisson's ratio $\nu$ (or $c_P$ for fluids when $\nu = 0.50$).
    pub fn from_m64_lut_entry(entry: &Q42ConstituentLutEntry) -> Result<Self, AcousticCouplingError> {
        let rho = entry.bulk_density_kg_m3 as f64;
        if rho <= 0.0 {
            return Err(AcousticCouplingError::InvalidDensity { rho_kg_per_m3: rho });
        }
        let nu = (entry.poissons_ratio_x100 as f64) * 0.01;
        let e_pa = (entry.youngs_modulus_mpa as f64) * 1e6;
        if entry.poissons_ratio_x100 >= 50 || e_pa == 0.0 {
            // Fluid limit: derive bulk modulus from acoustic P-wave velocity if E is not a solid Young's modulus
            let cp = entry.acoustic_p_wave_m_s as f64;
            let k_pa = if cp > 0.0 { rho * cp * cp } else { e_pa };
            Self::new_fluid(rho, k_pa, 101_325.0)
        } else {
            let k_pa = e_pa / (3.0 * (1.0 - 2.0 * nu));
            let mu_pa = e_pa / (2.0 * (1.0 + nu));
            Self::new_isotropic_solid(rho, k_pa, mu_pa, 101_325.0)
        }
    }

    /// Construct an equivalent fluid/poroelastic P-wave medium from an `.m64` `M64StrataColumnRecord`
    /// and its referenced LUT grain density.
    pub fn from_m64_strata_record(
        strata: &M64StrataColumnRecord,
        bulk_density_kg_per_m3: f64,
    ) -> Result<Self, AcousticCouplingError> {
        let cp = strata.acoustic_p_wave_m_s as f64;
        let mu_pa = (strata.bulk_shear_strength_kpa as f64) * 1e3;
        let p_mod = bulk_density_kg_per_m3 * cp * cp;
        let k_pa = p_mod - (4.0 / 3.0) * mu_pa;
        if mu_pa > 0.0 {
            Self::new_isotropic_solid(bulk_density_kg_per_m3, k_pa, mu_pa, 101_325.0)
        } else {
            Self::new_fluid(bulk_density_kg_per_m3, p_mod, 101_325.0)
        }
    }

    /// Validate physical bounds per C02-C06.
    pub fn validate(&self) -> Result<(), AcousticCouplingError> {
        if !self.density_kg_per_m3.is_finite()
            || !self.bulk_modulus_pa.is_finite()
            || !self.shear_modulus_pa.is_finite()
            || !self.ambient_pressure_pa.is_finite()
            || self.flow_velocity_m_per_s.iter().any(|v| !v.is_finite())
        {
            return Err(AcousticCouplingError::NonFiniteParameter);
        }
        if self.density_kg_per_m3 <= 0.0 {
            return Err(AcousticCouplingError::InvalidDensity {
                rho_kg_per_m3: self.density_kg_per_m3,
            });
        }
        if self.bulk_modulus_pa <= 0.0 {
            return Err(AcousticCouplingError::InvalidBulkModulus {
                bulk_modulus_pa: self.bulk_modulus_pa,
            });
        }
        if self.shear_modulus_pa < 0.0 {
            return Err(AcousticCouplingError::InvalidShearModulus {
                shear_modulus_pa: self.shear_modulus_pa,
            });
        }
        match self.kind {
            MediumKind::Fluid => {
                if self.shear_modulus_pa != 0.0 {
                    return Err(AcousticCouplingError::FluidRequiresZeroShearModulus {
                        shear_modulus_pa: self.shear_modulus_pa,
                    });
                }
            }
            MediumKind::IsotropicSolid => {
                if self.shear_modulus_pa <= 0.0 {
                    return Err(AcousticCouplingError::SolidRequiresPositiveShearModulus);
                }
            }
        }
        Ok(())
    }

    /// First Lamé parameter $\lambda = K - \frac{2}{3}\mu$ in $\text{Pa}$.
    #[inline]
    pub fn lame_lambda_pa(&self) -> f64 {
        self.bulk_modulus_pa - (2.0 / 3.0) * self.shear_modulus_pa
    }

    /// P-wave (plane-strain longitudinal) modulus $M = K + \frac{4}{3}\mu = \lambda + 2\mu$ in $\text{Pa}$.
    #[inline]
    pub fn p_wave_modulus_pa(&self) -> f64 {
        self.bulk_modulus_pa + (4.0 / 3.0) * self.shear_modulus_pa
    }

    /// Longitudinal (compressional P-wave) phase speed $c_P = \sqrt{\frac{K + \frac{4}{3}\mu}{\rho}}$ in $\text{m}\cdot\text{s}^{-1}$.
    #[inline]
    pub fn longitudinal_speed_m_per_s(&self) -> f64 {
        (self.p_wave_modulus_pa() / self.density_kg_per_m3).sqrt()
    }

    /// Transverse (shear S-wave) phase speed $c_S = \sqrt{\frac{\mu}{\rho}}$ in $\text{m}\cdot\text{s}^{-1}$ ($0$ for fluids).
    #[inline]
    pub fn shear_speed_m_per_s(&self) -> f64 {
        (self.shear_modulus_pa / self.density_kg_per_m3).sqrt()
    }

    /// Characteristic longitudinal acoustic impedance $Z_P = \rho c_P$ in $\text{Pa}\cdot\text{s}\cdot\text{m}^{-1}$ ($\text{Rayl}$).
    #[inline]
    pub fn characteristic_impedance_rayl(&self) -> f64 {
        self.density_kg_per_m3 * self.longitudinal_speed_m_per_s()
    }

    /// Characteristic shear impedance $Z_S = \rho c_S$ in $\text{Pa}\cdot\text{s}\cdot\text{m}^{-1}$.
    #[inline]
    pub fn shear_impedance_rayl(&self) -> f64 {
        self.density_kg_per_m3 * self.shear_speed_m_per_s()
    }
}

/// Six-component symmetric Cauchy stress tensor (`[xx, yy, zz, xy, yz, xz]`, tensile-positive) per C02-C04 / QAC Mode 01.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CauchyStress6 {
    pub xx_yy_zz_xy_yz_xz_pa: [f64; 6],
}

/// Six-component symmetric small-strain tensor (`[xx, yy, zz, xy, yz, xz]` using tensor shear $\varepsilon_{xy}$,
/// NOT engineering shear $\gamma_{xy} = 2\varepsilon_{xy}$) per C02-C04.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TensorStrain6 {
    pub xx_yy_zz_xy_yz_xz: [f64; 6],
}

impl TensorStrain6 {
    /// Volumetric dilatation $\operatorname{tr}(\varepsilon) = \varepsilon_{xx} + \varepsilon_{yy} + \varepsilon_{zz}$.
    #[inline]
    pub fn trace(&self) -> f64 {
        self.xx_yy_zz_xy_yz_xz[0] + self.xx_yy_zz_xy_yz_xz[1] + self.xx_yy_zz_xy_yz_xz[2]
    }

    /// Convert to full $3\times 3$ symmetric matrix.
    #[inline]
    pub fn to_matrix(&self) -> [[f64; 3]; 3] {
        let [xx, yy, zz, xy, yz, xz] = self.xx_yy_zz_xy_yz_xz;
        [[xx, xy, xz], [xy, yy, yz], [xz, yz, zz]]
    }
}

impl CauchyStress6 {
    /// Compute isotropic Cauchy stress from tensor strain:
    /// $\sigma_{ij} = \lambda \operatorname{tr}(\varepsilon)\delta_{ij} + 2\mu \varepsilon_{ij}$.
    pub fn from_isotropic_strain(medium: &ContinuumMedium, strain: &TensorStrain6) -> Self {
        let lambda = medium.lame_lambda_pa();
        let two_mu = 2.0 * medium.shear_modulus_pa;
        let tr = strain.trace();
        let [exx, eyy, ezz, exy, eyz, exz] = strain.xx_yy_zz_xy_yz_xz;
        Self {
            xx_yy_zz_xy_yz_xz_pa: [
                lambda * tr + two_mu * exx,
                lambda * tr + two_mu * eyy,
                lambda * tr + two_mu * ezz,
                two_mu * exy,
                two_mu * eyz,
                two_mu * exz,
            ],
        }
    }

    /// Trace $\operatorname{tr}(\sigma) = \sigma_{xx} + \sigma_{yy} + \sigma_{zz}$ in $\text{Pa}$.
    #[inline]
    pub fn trace_pa(&self) -> f64 {
        self.xx_yy_zz_xy_yz_xz_pa[0] + self.xx_yy_zz_xy_yz_xz_pa[1] + self.xx_yy_zz_xy_yz_xz_pa[2]
    }

    /// Mechanical hydrostatic pressure $p = -\frac{1}{3}\operatorname{tr}(\sigma)$ in $\text{Pa}$ (compressive-positive).
    #[inline]
    pub fn mechanical_pressure_pa(&self) -> f64 {
        -self.trace_pa() / 3.0
    }

    /// Convert to full $3\times 3$ symmetric matrix.
    #[inline]
    pub fn to_matrix(&self) -> [[f64; 3]; 3] {
        let [xx, yy, zz, xy, yz, xz] = self.xx_yy_zz_xy_yz_xz_pa;
        [[xx, xy, xz], [xy, yy, yz], [xz, yz, zz]]
    }

    /// Construct from a $3\times 3$ symmetric matrix.
    #[inline]
    pub fn from_matrix(m: &[[f64; 3]; 3]) -> Self {
        Self {
            xx_yy_zz_xy_yz_xz_pa: [
                m[0][0],
                m[1][1],
                m[2][2],
                0.5 * (m[0][1] + m[1][0]),
                0.5 * (m[1][2] + m[2][1]),
                0.5 * (m[0][2] + m[2][0]),
            ],
        }
    }

    /// Rotate stress tensor into parent frame via $R(q)\,\sigma\,R(q)^T$ per C02-C04.
    pub fn rotate_to_parent(&self, rot: &ActiveHamiltonRotation) -> Self {
        let rotated = rot.rotate_tensor_real(&self.to_matrix());
        Self::from_matrix(&rotated)
    }
}

/// Orthotropic 9-constant elastic stiffness tensor in Voigt order `[xx, yy, zz, xy, yz, xz]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrthotropicElasticMedium {
    pub density_kg_per_m3: f64,
    pub c11: f64,
    pub c22: f64,
    pub c33: f64,
    pub c12: f64,
    pub c13: f64,
    pub c23: f64,
    pub c44: f64,
    pub c55: f64,
    pub c66: f64,
}

impl OrthotropicElasticMedium {
    /// Validate thermodynamic stability (Sylvester's criterion on upper $3\times 3$ block and shear diagonals).
    pub fn validate(&self) -> Result<(), AcousticCouplingError> {
        let vals = [
            self.density_kg_per_m3,
            self.c11,
            self.c22,
            self.c33,
            self.c12,
            self.c13,
            self.c23,
            self.c44,
            self.c55,
            self.c66,
        ];
        if vals.iter().any(|v| !v.is_finite()) {
            return Err(AcousticCouplingError::NonFiniteParameter);
        }
        if self.density_kg_per_m3 <= 0.0 {
            return Err(AcousticCouplingError::InvalidDensity {
                rho_kg_per_m3: self.density_kg_per_m3,
            });
        }
        let m1 = self.c11;
        let m2 = self.c11 * self.c22 - self.c12 * self.c12;
        let m3 = self.c11 * (self.c22 * self.c33 - self.c23 * self.c23)
            - self.c12 * (self.c12 * self.c33 - self.c23 * self.c13)
            + self.c13 * (self.c12 * self.c23 - self.c22 * self.c13);
        if m1 <= 0.0 || m2 <= 0.0 || m3 <= 0.0 || self.c44 <= 0.0 || self.c55 <= 0.0 || self.c66 <= 0.0 {
            return Err(AcousticCouplingError::NonPositiveDefiniteOrthotropicStiffness);
        }
        Ok(())
    }

    /// Evaluate Cauchy stress from tensor strain (`[exx, eyy, ezz, exy, eyz, exz]`).
    pub fn evaluate_stress(&self, strain: &TensorStrain6) -> CauchyStress6 {
        let [exx, eyy, ezz, exy, eyz, exz] = strain.xx_yy_zz_xy_yz_xz;
        CauchyStress6 {
            xx_yy_zz_xy_yz_xz_pa: [
                self.c11 * exx + self.c12 * eyy + self.c13 * ezz,
                self.c12 * exx + self.c22 * eyy + self.c23 * ezz,
                self.c13 * exx + self.c23 * eyy + self.c33 * ezz,
                2.0 * self.c66 * exy,
                2.0 * self.c44 * eyz,
                2.0 * self.c55 * exz,
            ],
        }
    }
}

/// Planar acoustic interface reflection and transmission coefficients (pressure amplitude & energy flux).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InterfaceScatteringCoefficients {
    /// Pressure reflection coefficient $r_p = p_{\text{refl}} / p_{\text{inc}}$.
    pub pressure_reflection: f64,
    /// Pressure transmission coefficient $t_p = p_{\text{trans}} / p_{\text{inc}}$ (can exceed $1.0$ when $Z_2 > Z_1$).
    pub pressure_transmission: f64,
    /// Reflected normal intensity/energy fraction $R_E \in [0, 1]$.
    pub energy_reflection: f64,
    /// Transmitted normal intensity/energy fraction $T_E \in [0, 1]$, satisfying $R_E + T_E = 1$.
    pub energy_transmission: f64,
    /// True if incidence angle exceeds critical angle $\theta_c = \arcsin(c_1 / c_2)$ (total internal reflection).
    pub total_internal_reflection: bool,
}

/// Evaluate normal or oblique planar interface reflection and transmission between two media
/// using longitudinal impedances $Z_1 = \rho_1 c_{P,1}$ and $Z_2 = \rho_2 c_{P,2}$.
pub fn evaluate_planar_interface_scattering(
    medium_1: &ContinuumMedium,
    medium_2: &ContinuumMedium,
    theta_incident_rad: f64,
) -> Result<InterfaceScatteringCoefficients, AcousticCouplingError> {
    medium_1.validate()?;
    medium_2.validate()?;
    if !theta_incident_rad.is_finite() || !(0.0..(0.5 * PI)).contains(&theta_incident_rad) {
        return Err(AcousticCouplingError::InvalidIncidenceAngle {
            theta_rad: theta_incident_rad,
        });
    }
    let c1 = medium_1.longitudinal_speed_m_per_s();
    let c2 = medium_2.longitudinal_speed_m_per_s();
    let z1 = medium_1.characteristic_impedance_rayl();
    let z2 = medium_2.characteristic_impedance_rayl();

    let cos_theta_1 = theta_incident_rad.cos();
    let sin_theta_1 = theta_incident_rad.sin();
    let sin_theta_2 = (c2 / c1) * sin_theta_1;

    if sin_theta_2 >= 1.0 {
        // Total internal reflection: evanescent transmitted wave carries zero normal active power
        return Ok(InterfaceScatteringCoefficients {
            pressure_reflection: 1.0,
            pressure_transmission: 0.0,
            energy_reflection: 1.0,
            energy_transmission: 0.0,
            total_internal_reflection: true,
        });
    }

    let cos_theta_2 = (1.0 - sin_theta_2 * sin_theta_2).sqrt();
    let z1_obl = z1 / cos_theta_1;
    let z2_obl = z2 / cos_theta_2;

    let rp = (z2_obl - z1_obl) / (z2_obl + z1_obl);
    let tp = (2.0 * z2_obl) / (z2_obl + z1_obl);
    let re = rp * rp;
    let te = (z1_obl / z2_obl) * tp * tp;

    Ok(InterfaceScatteringCoefficients {
        pressure_reflection: rp,
        pressure_transmission: tp,
        energy_reflection: re,
        energy_transmission: te,
        total_internal_reflection: false,
    })
}

/// Oblique fluid-solid interface energy partitioning (incident fluid P-wave into reflected fluid P,
/// transmitted solid P, and mode-converted solid SV shear wave).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluidSolidObliquePartition {
    pub pressure_reflection: f64,
    pub energy_reflection_p: f64,
    pub energy_transmission_p: f64,
    pub energy_transmission_sv: f64,
}

/// Evaluate oblique fluid-to-isotropic-solid wave partitioning via effective normal impedances
/// $Z_{\text{eff}} = Z_P \cos^2(2\theta_S) + Z_S \sin^2(2\theta_S)$.
pub fn evaluate_fluid_solid_oblique_partition(
    fluid: &ContinuumMedium,
    solid: &ContinuumMedium,
    theta_incident_rad: f64,
) -> Result<FluidSolidObliquePartition, AcousticCouplingError> {
    fluid.validate()?;
    solid.validate()?;
    if fluid.kind != MediumKind::Fluid {
        return Err(AcousticCouplingError::FluidRequiresZeroShearModulus {
            shear_modulus_pa: fluid.shear_modulus_pa,
        });
    }
    if solid.kind != MediumKind::IsotropicSolid {
        return Err(AcousticCouplingError::SolidRequiresPositiveShearModulus);
    }
    if !theta_incident_rad.is_finite() || !(0.0..(0.5 * PI)).contains(&theta_incident_rad) {
        return Err(AcousticCouplingError::InvalidIncidenceAngle {
            theta_rad: theta_incident_rad,
        });
    }
    let cf = fluid.longitudinal_speed_m_per_s();
    let cp = solid.longitudinal_speed_m_per_s();
    let cs = solid.shear_speed_m_per_s();

    let sin_i = theta_incident_rad.sin();
    let cos_i = theta_incident_rad.cos();
    let zf_obl = fluid.characteristic_impedance_rayl() / cos_i;

    let sin_p = (cp / cf) * sin_i;
    let sin_s = (cs / cf) * sin_i;

    if sin_p >= 1.0 && sin_s >= 1.0 {
        return Ok(FluidSolidObliquePartition {
            pressure_reflection: 1.0,
            energy_reflection_p: 1.0,
            energy_transmission_p: 0.0,
            energy_transmission_sv: 0.0,
        });
    }

    let cos_2s = 1.0 - 2.0 * sin_s * sin_s;
    let sin_2s = 2.0 * sin_s * (1.0 - sin_s * sin_s).max(0.0).sqrt();

    let zp_part = if sin_p < 1.0 {
        let cos_p = (1.0 - sin_p * sin_p).sqrt();
        (solid.characteristic_impedance_rayl() / cos_p) * cos_2s * cos_2s
    } else {
        0.0
    };
    let zs_part = if sin_s < 1.0 {
        let cos_s = (1.0 - sin_s * sin_s).sqrt();
        (solid.shear_impedance_rayl() / cos_s) * sin_2s * sin_2s
    } else {
        0.0
    };

    let z_solid_eff = zp_part + zs_part;
    let rp = (z_solid_eff - zf_obl) / (z_solid_eff + zf_obl);
    let re = rp * rp;
    let trans_total = 1.0 - re;
    let (tp_e, tsv_e) = if z_solid_eff > 0.0 {
        (
            trans_total * (zp_part / z_solid_eff),
            trans_total * (zs_part / z_solid_eff),
        )
    } else {
        (0.0, 0.0)
    };

    Ok(FluidSolidObliquePartition {
        pressure_reflection: rp,
        energy_reflection_p: re,
        energy_transmission_p: tp_e,
        energy_transmission_sv: tsv_e,
    })
}

/// Power-law acoustic attenuation & Kramers-Kronig causal dispersion model (Szabo / Waters law):
/// $\alpha(\omega) = \alpha_0 |\omega|^y$ ($\text{Np}\cdot\text{m}^{-1}$).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerLawAcousticAttenuation {
    /// Reference angular frequency $\omega_0 > 0$ ($\text{rad}\cdot\text{s}^{-1}$).
    pub omega_0_rad_s: f64,
    /// Phase speed $c(\omega_0) > 0$ at reference frequency ($\text{m}\cdot\text{s}^{-1}$).
    pub c_0_m_per_s: f64,
    /// Power-law prefactor $\alpha_0 \ge 0$ in $\text{Np}\cdot\text{m}^{-1}\cdot(\text{rad/s})^{-y}$.
    pub alpha_0_np_per_m_rad_s_y: f64,
    /// Power-law exponent $y \in (0, 2]$.
    pub exponent_y: f64,
}

impl PowerLawAcousticAttenuation {
    pub fn validate(&self) -> Result<(), AcousticCouplingError> {
        if !self.omega_0_rad_s.is_finite()
            || !self.c_0_m_per_s.is_finite()
            || !self.alpha_0_np_per_m_rad_s_y.is_finite()
            || !self.exponent_y.is_finite()
        {
            return Err(AcousticCouplingError::NonFiniteParameter);
        }
        if self.omega_0_rad_s <= 0.0 {
            return Err(AcousticCouplingError::InvalidFrequency {
                omega_rad_s: self.omega_0_rad_s,
            });
        }
        if self.c_0_m_per_s <= 0.0 {
            return Err(AcousticCouplingError::InvalidBulkModulus {
                bulk_modulus_pa: self.c_0_m_per_s,
            });
        }
        if self.alpha_0_np_per_m_rad_s_y < 0.0 || self.exponent_y <= 0.0 || self.exponent_y > 2.0 {
            return Err(AcousticCouplingError::InvalidAttenuationExponent { y: self.exponent_y });
        }
        Ok(())
    }

    /// Attenuation $\alpha(\omega) = \alpha_0 \omega^y$ in $\text{Np}\cdot\text{m}^{-1}$.
    pub fn attenuation_np_per_m(&self, omega_rad_s: f64) -> Result<f64, AcousticCouplingError> {
        self.validate()?;
        if !omega_rad_s.is_finite() || omega_rad_s <= 0.0 {
            return Err(AcousticCouplingError::InvalidFrequency { omega_rad_s });
        }
        Ok(self.alpha_0_np_per_m_rad_s_y * omega_rad_s.powf(self.exponent_y))
    }

    /// Kramers-Kronig causal phase velocity $c(\omega)$ in $\text{m}\cdot\text{s}^{-1}$:
    /// - For $y = 1$: $\frac{1}{c(\omega)} = \frac{1}{c(\omega_0)} - \frac{2\alpha_0}{\pi}\ln\left(\frac{\omega}{\omega_0}\right)$
    /// - For $y \ne 1, y < 2$: $\frac{1}{c(\omega)} = \frac{1}{c(\omega_0)} + \alpha_0 \tan\left(\frac{\pi y}{2}\right)\left(\omega^{y-1} - \omega_0^{y-1}\right)$
    /// - For $y = 2$ (thermoviscous Stokes limit): nondispersive to leading order, $c(\omega) = c(\omega_0)$.
    pub fn phase_velocity_m_per_s(&self, omega_rad_s: f64) -> Result<f64, AcousticCouplingError> {
        self.validate()?;
        if !omega_rad_s.is_finite() || omega_rad_s <= 0.0 {
            return Err(AcousticCouplingError::InvalidFrequency { omega_rad_s });
        }
        let inv_c0 = 1.0 / self.c_0_m_per_s;
        let delta_slowness = if (self.exponent_y - 1.0).abs() < 1e-9 {
            -(2.0 * self.alpha_0_np_per_m_rad_s_y / PI) * (omega_rad_s / self.omega_0_rad_s).ln()
        } else if (self.exponent_y - 2.0).abs() < 1e-9 {
            0.0
        } else {
            self.alpha_0_np_per_m_rad_s_y
                * (0.5 * PI * self.exponent_y).tan()
                * (omega_rad_s.powf(self.exponent_y - 1.0)
                    - self.omega_0_rad_s.powf(self.exponent_y - 1.0))
        };
        let inv_c = inv_c0 + delta_slowness;
        if inv_c <= 0.0 {
            return Err(AcousticCouplingError::InvalidFrequency { omega_rad_s });
        }
        Ok(1.0 / inv_c)
    }
}

/// C02-C06 EMF `material_entry == 0` policy: `0` is unbound unless the stream profile explicitly
/// declares `0 = vacuum` (e.g. Mode 0x0101 `polarimetric_ray_v1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmfMaterialZeroPolicy {
    UnboundRejected,
    ExplicitVacuumAdmitted,
}

/// Resolved EMF material reference under C02-C06.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolvedEmfMaterial {
    ExplicitVacuum,
    TableEntry { one_based_index: u32 },
}

/// Resolve an EMF `material_entry` (`u32`) under C02-C06 without conflating `0` with QAC's uniform medium.
pub fn resolve_emf_material_entry(
    material_entry: u32,
    table_len: usize,
    zero_policy: EmfMaterialZeroPolicy,
) -> Result<ResolvedEmfMaterial, AcousticCouplingError> {
    if material_entry == 0 {
        match zero_policy {
            EmfMaterialZeroPolicy::UnboundRejected => {
                Err(AcousticCouplingError::EmfMaterialZeroUnbound)
            }
            EmfMaterialZeroPolicy::ExplicitVacuumAdmitted => Ok(ResolvedEmfMaterial::ExplicitVacuum),
        }
    } else if (material_entry as usize) > table_len {
        Err(AcousticCouplingError::MaterialEntryOutOfBounds {
            index: material_entry,
            len: table_len,
        })
    } else {
        Ok(ResolvedEmfMaterial::TableEntry {
            one_based_index: material_entry,
        })
    }
}

/// Resolve a QAC `material_ref` (`u32`) under C02-C06:
/// `material_ref == 0` resolves to the explicitly declared uniform `ContinuumMedium` (Schema 200),
/// NEVER vacuum or $\rho = 0$.
pub fn resolve_qac_material_ref<'a>(
    material_ref: u32,
    declared_uniform_medium: Option<&'a ContinuumMedium>,
    material_table: &'a [ContinuumMedium],
) -> Result<&'a ContinuumMedium, AcousticCouplingError> {
    if material_ref == 0 {
        let m = declared_uniform_medium.ok_or(AcousticCouplingError::QacUniformMediumNotDeclared)?;
        m.validate()?;
        Ok(m)
    } else {
        let idx = (material_ref as usize) - 1;
        let m = material_table
            .get(idx)
            .ok_or(AcousticCouplingError::MaterialEntryOutOfBounds {
                index: material_ref,
                len: material_table.len(),
            })?;
        m.validate()?;
        Ok(m)
    }
}
