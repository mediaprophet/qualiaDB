//! QAC Continuum Numerical Solvers, Frequency-Domain Helmholtz, Room Acoustics & Kramers-Kronig Dispersion.
//!
//! Implements Packet **N02 (`acoustic_numerical_engineer`)** for the QAC v1 evolution contract,
//! delegating reusable 3D continuum solvers to [`etalon_sim::acoustics_continuum`] and WGSL
//! compute kernels to [`etalon_wgsl_forge::numerical::physics::acoustic_fdtd`], while providing
//! QAC v1 Mode `0x0000` (`pressure_velocity_continuum`) and Mode `0x0001`
//! (`elastic_stress_velocity_continuum`) 64-byte wire record codecs and GPU buffer adapters:
//! - [`fdtd_fluid`]: Mode `0x0000` (`PressureVelocityRecord`) + 3D staggered-grid fluid FDTD/CPML
//!   solver and WGSL compute kernel adapter.
//! - [`elastic_virieux`]: Mode `0x0001` (`ElasticStressVelocityRecord`) + 3D Virieux 6-component
//!   Cauchy stress-velocity solver, Hamilton quaternion stress rotation, normal-incidence impedance
//!   interface, and WGSL compute kernel adapter.
//! - [`helmholtz`]: 3D frequency-domain Helmholtz operator (7pt/27pt stencils), Jacobi/BiCGSTAB
//!   solvers, 3D Green's function, and cavity eigenmode oracles.
//! - [`room_acoustics`]: Sabine, Eyring-Norris, and Millington-Sette $T_{60}$ calculators and
//!   ISO 3382-1 Schroeder backward integration ($\text{EDT}, T_{20}, T_{30}, C_{50}, C_{80}, D_{50}, T_s$).
//! - [`attenuation_kk`]: Kramers-Kronig causal attenuation-dispersion consistency for power-law
//!   media ($0 < y < 3$, including $y=1$) and multi-relaxation atmospheric/seawater profiles.

pub mod attenuation_kk;
pub mod elastic_virieux;
pub mod fdtd_fluid;
pub mod helmholtz;
pub mod poroelastic_bioheat;
pub mod room_acoustics;

#[cfg(test)]
mod tests;

pub use attenuation_kk::{MultiRelaxationMedium, PowerLawAttenuation, RelaxationProcess};
pub use poroelastic_bioheat::*;
pub use elastic_virieux::{
    decode_elastic_stress_velocity_records, encode_elastic_stress_velocity_records,
    step_elastic_virieux_in_place, CauchyStress6, ElasticBoundaryCondition, ElasticMedium,
    ElasticStressVelocityRecord, ElasticVirieuxConfig, ElasticVirieuxState, HamiltonQuaternionXyzw,
    NormalIncidenceInterface, TensorStrain6, ELASTIC_STRESS_VELOCITY_RECORD_STRIDE,
    ESV_DEFINED_VALIDITY_MASK, ESV_VALID_DISPLACEMENT, ESV_VALID_STRESS, ESV_VALID_VELOCITY,
    MODE_ELASTIC_STRESS_VELOCITY_CONTINUUM,
};
#[cfg(feature = "gpu-runtime")]
pub use elastic_virieux::{
    elastic_config_to_gpu_params, elastic_virieux_fdtd_step_cpu, elastic_virieux_fdtd_step_gpu,
    elastic_virieux_fdtd_step_with_context, pack_elastic_virieux_state_f32,
    unpack_elastic_virieux_state_f32, ElasticVirieuxFdtdGpuParams, ElasticVirieuxGpuBoundary,
    ELASTIC_VIRIEUX_CELL_STRIDE, ELASTIC_VIRIEUX_FDTD_ENTRY, ELASTIC_VIRIEUX_FDTD_WGSL,
};
pub use etalon_sim::acoustics_continuum::{
    assert_b32_scalar_budget, b32_scalar_tolerance, B32_UNIT_ROUNDOFF,
};
pub use fdtd_fluid::{
    collocate_fluid_cell, decode_pressure_velocity_records, encode_pressure_velocity_records,
    evaluate_direct_energy_and_intensity, step_fluid_fdtd_in_place, FluidBoundaryCondition,
    FluidFdtdConfig, FluidFdtdState, FluidMedium, NumericalError, PressureVelocityRecord,
    StaggeredFluidOffsets, MODE_PRESSURE_VELOCITY_CONTINUUM, PRESSURE_VELOCITY_RECORD_STRIDE,
    PV_DEFINED_VALIDITY_MASK, PV_VALID_ENERGY_DENSITY, PV_VALID_INTENSITY, PV_VALID_POSITION,
    PV_VALID_PRESSURE, PV_VALID_VELOCITY,
};
#[cfg(feature = "gpu-runtime")]
pub use fdtd_fluid::{
    acoustic_fluid_fdtd_step_cpu, acoustic_fluid_fdtd_step_gpu,
    acoustic_fluid_fdtd_step_with_context, fluid_config_to_gpu_params, pack_fluid_fdtd_state_f32,
    unpack_fluid_fdtd_state_f32, AcousticFluidFdtdGpuParams, AcousticFluidGpuBoundary,
    ACOUSTIC_FLUID_CELL_STRIDE, ACOUSTIC_FLUID_FDTD_ENTRY, ACOUSTIC_FLUID_FDTD_WGSL,
};
pub use helmholtz::{
    apply_helmholtz_operator, helmholtz_greens_function_3d,
    rectangular_cavity_eigenfrequency_hz, rectangular_cavity_eigenmode_value,
    solve_helmholtz_bicgstab, step_helmholtz_jacobi, Complex64, HelmholtzBoundary,
    HelmholtzConfig, HelmholtzSolveReport, HelmholtzStencil, HelmholtzWorkspace,
};
pub use room_acoustics::{
    analyze_room_impulse_response, fit_decay_rt60_s, schroeder_backward_integration_db,
    RirAcousticsReport, RoomGeometry, SurfacePatch, RT60_NUMERATOR_55_25,
    RT60_NUMERATOR_EXACT_24LN10,
};
