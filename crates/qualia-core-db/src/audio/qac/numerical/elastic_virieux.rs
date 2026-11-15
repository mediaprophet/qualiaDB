//! QAC Mode `0x0001` (`elastic_stress_velocity_continuum`) Wire Adapter & 3D Virieux Elastic Facade.
//!
//! Re-exports the canonical 3D Virieux 6-component Cauchy stress-velocity solver, active Hamilton
//! quaternion stress rotation, and Mode 03 normal-incidence impedance interface from
//! [`etalon_sim::acoustics_continuum::elastic_virieux`] and the WGSL compute kernels from
//! [`etalon_wgsl_forge::numerical::physics::acoustic_fdtd`], adding QAC batch wire codecs and
//! `f64` $\leftrightarrow$ `f32` GPU state buffer adapters.

pub use etalon_sim::acoustics_continuum::elastic_virieux::{
    step_elastic_virieux_in_place, CauchyStress6, ElasticBoundaryCondition, ElasticMedium,
    ElasticStressVelocityRecord, ElasticVirieuxConfig, ElasticVirieuxState,
    HamiltonQuaternionXyzw, NormalIncidenceInterface, TensorStrain6,
    ELASTIC_STRESS_VELOCITY_RECORD_STRIDE, ESV_DEFINED_VALIDITY_MASK, ESV_VALID_DISPLACEMENT,
    ESV_VALID_STRESS, ESV_VALID_VELOCITY, MODE_ELASTIC_STRESS_VELOCITY_CONTINUUM,
};
#[cfg(feature = "gpu-runtime")]
pub use etalon_wgsl_forge::numerical::physics::acoustic_fdtd::{
    elastic_virieux_fdtd_step_cpu, elastic_virieux_fdtd_step_gpu,
    elastic_virieux_fdtd_step_with_context, ElasticVirieuxFdtdGpuParams, ElasticVirieuxGpuBoundary,
    ELASTIC_VIRIEUX_CELL_STRIDE, ELASTIC_VIRIEUX_FDTD_ENTRY, ELASTIC_VIRIEUX_FDTD_WGSL,
};

use super::fdtd_fluid::NumericalError;

/// Encode a slice of Mode `0x0001` [`ElasticStressVelocityRecord`] values into contiguous 64-byte wire records.
pub fn encode_elastic_stress_velocity_records(
    records: &[ElasticStressVelocityRecord],
) -> Result<Vec<u8>, NumericalError> {
    let mut out = Vec::with_capacity(records.len() * ELASTIC_STRESS_VELOCITY_RECORD_STRIDE);
    for rec in records {
        let bytes = rec.to_wire_bytes()?;
        out.extend_from_slice(&bytes);
    }
    Ok(out)
}

/// Decode a contiguous byte slice of 64-byte Mode `0x0001` wire records into [`ElasticStressVelocityRecord`] values.
pub fn decode_elastic_stress_velocity_records(
    bytes: &[u8],
) -> Result<Vec<ElasticStressVelocityRecord>, NumericalError> {
    if bytes.len() % ELASTIC_STRESS_VELOCITY_RECORD_STRIDE != 0 {
        return Err(NumericalError::WireEncoding(
            "ElasticStressVelocityRecord stream byte length must be a multiple of 64",
        ));
    }
    let count = bytes.len() / ELASTIC_STRESS_VELOCITY_RECORD_STRIDE;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * ELASTIC_STRESS_VELOCITY_RECORD_STRIDE;
        out.push(ElasticStressVelocityRecord::from_wire_bytes(
            &bytes[off..off + ELASTIC_STRESS_VELOCITY_RECORD_STRIDE],
        )?);
    }
    Ok(out)
}

/// Convert an [`ElasticVirieuxConfig`] into [`ElasticVirieuxFdtdGpuParams`] for WGSL compute execution.
#[cfg(feature = "gpu-runtime")]
pub fn elastic_config_to_gpu_params(
    cfg: &ElasticVirieuxConfig,
) -> Result<ElasticVirieuxFdtdGpuParams, NumericalError> {
    cfg.validate()?;
    let boundary = match cfg.boundary {
        ElasticBoundaryCondition::Periodic => ElasticVirieuxGpuBoundary::Periodic,
        ElasticBoundaryCondition::FreeSurface => ElasticVirieuxGpuBoundary::FreeSurface,
    };
    Ok(ElasticVirieuxFdtdGpuParams {
        dims: [cfg.dims[0] as u32, cfg.dims[1] as u32, cfg.dims[2] as u32],
        spacing_m: [
            cfg.spacing_m[0] as f32,
            cfg.spacing_m[1] as f32,
            cfg.spacing_m[2] as f32,
        ],
        dt_s: cfg.dt_s as f32,
        density_kg_m3: cfg.medium.density_kg_m3 as f32,
        bulk_modulus_pa: cfg.medium.bulk_modulus_pa as f32,
        shear_modulus_pa: cfg.medium.shear_modulus_pa as f32,
        boundary,
    })
}

/// Pack an [`ElasticVirieuxState`] into the flat `f32` storage buffer layout
/// (`[vx, vy, vz, sxx, syy, szz, sxy, syz, sxz]` per cell) consumed by [`ELASTIC_VIRIEUX_FDTD_WGSL`].
#[cfg(feature = "gpu-runtime")]
pub fn pack_elastic_virieux_state_f32(state: &ElasticVirieuxState) -> Vec<f32> {
    let n = state.vx.len();
    let mut out = Vec::with_capacity(n * ELASTIC_VIRIEUX_CELL_STRIDE);
    for i in 0..n {
        out.push(state.vx[i] as f32);
        out.push(state.vy[i] as f32);
        out.push(state.vz[i] as f32);
        out.push(state.sxx[i] as f32);
        out.push(state.syy[i] as f32);
        out.push(state.szz[i] as f32);
        out.push(state.sxy[i] as f32);
        out.push(state.syz[i] as f32);
        out.push(state.sxz[i] as f32);
    }
    out
}

/// Unpack a flat `f32` storage buffer (`[vx, vy, vz, sxx, syy, szz, sxy, syz, sxz]` per cell)
/// back into an [`ElasticVirieuxState`].
#[cfg(feature = "gpu-runtime")]
pub fn unpack_elastic_virieux_state_f32(
    flat: &[f32],
    state: &mut ElasticVirieuxState,
) -> Result<(), NumericalError> {
    let n = state.vx.len();
    if flat.len() != n * ELASTIC_VIRIEUX_CELL_STRIDE {
        return Err(NumericalError::BufferLengthMismatch {
            buffer: "unpack_elastic_virieux_state_f32",
            expected: n * ELASTIC_VIRIEUX_CELL_STRIDE,
            actual: flat.len(),
        });
    }
    for i in 0..n {
        let base = i * ELASTIC_VIRIEUX_CELL_STRIDE;
        state.vx[i] = flat[base] as f64;
        state.vy[i] = flat[base + 1] as f64;
        state.vz[i] = flat[base + 2] as f64;
        state.sxx[i] = flat[base + 3] as f64;
        state.syy[i] = flat[base + 4] as f64;
        state.szz[i] = flat[base + 5] as f64;
        state.sxy[i] = flat[base + 6] as f64;
        state.syz[i] = flat[base + 7] as f64;
        state.sxz[i] = flat[base + 8] as f64;
    }
    state.step_index += 1;
    Ok(())
}
