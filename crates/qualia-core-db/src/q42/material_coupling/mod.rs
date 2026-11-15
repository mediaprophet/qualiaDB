//! Physical and cryptographic material coupling across EMF v4 (`Q42EMF64`), QAC v1 (`qac\0\0\0\0\0`),
//! and `.m64` (`Q42ConstituentLutEntry` / `M64StrataColumnRecord`) containers (Packet M01).
//!
//! Implements:
//! - [`constitutive`]: Schema 133 multi-pole Debye + Lorentz + ohmic conductivity $\varepsilon(\omega)$
//!   and $\mu(\omega)$, full $3\times 3$ symmetric/Hermitian anisotropic constitutive tensors, and
//!   C02-C04 active Hamilton quaternion frame rotations (`(((x*x+y*y)+z*z)+w*w)` with `1e-12` admission).
//! - [`dispersion`]: Numerical Kramers-Kronig passivity ($\omega\operatorname{Im}\varepsilon \ge 0$,
//!   $\omega\operatorname{Im}\mu \ge 0$), Cauchy principal-value Hilbert integral consistency checks,
//!   Brillouin dispersive stored energy density, and Mode 0 instantaneous/phasor field state evaluation.
//! - [`acoustic_coupling`]: QAC Schema 200 fluid and isotropic/orthotropic elastic media, 6-component
//!   Cauchy stress and tensor strain (`p = -trace(sigma)/3`), planar normal/oblique fluid-fluid and
//!   fluid-solid Zoeppritz energy partitioning ($R_E + T_E = 1$), power-law attenuation dispersion,
//!   and C02-C06 EMF vs QAC material namespace separation.
//! - [`transduction`]: Photoacoustic thermoelastic pressure generation ($p_0 = \Gamma \mu_a \Phi$)
//!   with thermal/stress confinement checks, piezoelectric electro-elastic coupling ($k^2 < 1$),
//!   Duhamel-Neumann thermo-elastic stress, coupled radar/sonar interface scattering, and C02-C08
//!   explicit EMF ($\exp(-i\omega t)$) $\leftrightarrow$ QAC ($\exp(+i\omega t)$) I/Q carrier conjugation.
//! - [`binding`]: Schema 105 cryptographic `.m64` container & 16-KB `BankedConstituentLut` BLAKE3-256
//!   verification, Bank 1 epoch and canonical material hash checks, Schema 106 proper-rigid vs
//!   general-affine transform separation, and C02-C03 signed-`i128` rational clock `[from, until)`
//!   validity verification.

pub mod acoustic_coupling;
pub mod binding;
pub mod constitutive;
pub mod dispersion;
pub mod transduction;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_binding;

pub use acoustic_coupling::{
    evaluate_fluid_solid_oblique_partition, evaluate_planar_interface_scattering,
    resolve_emf_material_entry, resolve_qac_material_ref, AcousticCouplingError, CauchyStress6,
    ContinuumMedium, EmfMaterialZeroPolicy, FluidSolidObliquePartition,
    InterfaceScatteringCoefficients, MediumKind, OrthotropicElasticMedium,
    PowerLawAcousticAttenuation, ResolvedEmfMaterial, TensorStrain6,
};
pub use binding::{
    compute_blake3_256, reject_spatial_key_only_join, CoordinateTransform4x4,
    CoordinateTransformKind, HalfOpenValidityInterval, MaterialBinding, MaterialBindingError,
    MaterialEntryIdentity, RationalClock, RationalInstant, VerifiedBoundMaterial,
    TRANSFORM_ORTHOGONALITY_TOLERANCE,
};
pub use constitutive::{
    ActiveHamiltonRotation, AnisotropicConstitutiveTensor3x3, Complex64, ConstitutiveError,
    ConstitutiveModel, DebyePole, LorentzPole, C_0_M_PER_S, EPSILON_0_F_PER_M, MU_0_H_PER_M,
    QUATERNION_NORM_SQ_TOLERANCE, ROTATION_ORTHOGONALITY_BUDGET, Z_0_OHMS,
};
pub use dispersion::{
    evaluate_brillouin_energy_density_j_per_m3, evaluate_isotropic_eb_state,
    evaluate_wave_propagation, kramers_kronig_im_from_re, kramers_kronig_re_from_im,
    verify_kramers_kronig_consistency, verify_passivity_on_grid, DispersionError,
    EmAmplitudeConvention, EmWavePropagation, InstantaneousOrPhasorFieldState,
    KramersKronigReport,
};
pub use transduction::{
    convert_iq_carrier_convention, evaluate_radar_sonar_normal_interface,
    evaluate_thermoelastic_stress, reconstruct_real_passband_sample, IqCarrierConvention,
    PhotoacousticParameters, PhotoacousticResponse, PiezoelectricConstitutiveModel,
    PiezoelectricState, RadarSonarInterfaceSummary, TransductionError,
};
