//! QAC Mode `0x0000` (`pressure_velocity_continuum`) Wire Adapter & 3D Staggered Fluid FDTD Facade.
//!
//! Re-exports the canonical 3D staggered-grid fluid FDTD/CPML solver and Mode `0x0000` 64-byte
//! wire record types from [`etalon_sim::acoustics_continuum::fdtd_fluid`] and the WGSL compute
//! kernels from [`etalon_wgsl_forge::numerical::physics::acoustic_fdtd`], adding QAC batch wire
//! codecs and `f64` $\leftrightarrow$ `f32` GPU state buffer adapters.

pub use etalon_sim::acoustics_continuum::fdtd_fluid::{
    collocate_fluid_cell, evaluate_direct_energy_and_intensity, step_fluid_fdtd_in_place,
    FluidBoundaryCondition, FluidFdtdConfig, FluidFdtdState, FluidMedium, NumericalError,
    PressureVelocityRecord, StaggeredFluidOffsets, MODE_PRESSURE_VELOCITY_CONTINUUM,
    PRESSURE_VELOCITY_RECORD_STRIDE, PV_DEFINED_VALIDITY_MASK, PV_VALID_ENERGY_DENSITY,
    PV_VALID_INTENSITY, PV_VALID_POSITION, PV_VALID_PRESSURE, PV_VALID_VELOCITY,
};
#[cfg(feature = "gpu-runtime")]
pub use etalon_wgsl_forge::numerical::physics::acoustic_fdtd::{
    acoustic_fluid_fdtd_step_cpu, acoustic_fluid_fdtd_step_gpu,
    acoustic_fluid_fdtd_step_with_context, AcousticFluidFdtdGpuParams, AcousticFluidGpuBoundary,
    ACOUSTIC_FLUID_CELL_STRIDE, ACOUSTIC_FLUID_FDTD_ENTRY, ACOUSTIC_FLUID_FDTD_WGSL,
};

/// Encode a slice of Mode `0x0000` [`PressureVelocityRecord`] values into contiguous 64-byte wire records.
pub fn encode_pressure_velocity_records(
    records: &[PressureVelocityRecord],
) -> Result<Vec<u8>, NumericalError> {
    let mut out = Vec::with_capacity(records.len() * PRESSURE_VELOCITY_RECORD_STRIDE);
    for rec in records {
        let bytes = rec.to_wire_bytes()?;
        out.extend_from_slice(&bytes);
    }
    Ok(out)
}

/// Decode a contiguous byte slice of 64-byte Mode `0x0000` wire records into [`PressureVelocityRecord`] values.
pub fn decode_pressure_velocity_records(
    bytes: &[u8],
) -> Result<Vec<PressureVelocityRecord>, NumericalError> {
    if bytes.len() % PRESSURE_VELOCITY_RECORD_STRIDE != 0 {
        return Err(NumericalError::WireEncoding(
            "PressureVelocityRecord stream byte length must be a multiple of 64",
        ));
    }
    let count = bytes.len() / PRESSURE_VELOCITY_RECORD_STRIDE;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * PRESSURE_VELOCITY_RECORD_STRIDE;
        out.push(PressureVelocityRecord::from_wire_bytes(
            &bytes[off..off + PRESSURE_VELOCITY_RECORD_STRIDE],
        )?);
    }
    Ok(out)
}

/// Convert a [`FluidFdtdConfig`] into [`AcousticFluidFdtdGpuParams`] for WGSL compute execution.
#[cfg(feature = "gpu-runtime")]
pub fn fluid_config_to_gpu_params(
    cfg: &FluidFdtdConfig,
) -> Result<AcousticFluidFdtdGpuParams, NumericalError> {
    cfg.validate()?;
    let boundary = match cfg.boundary {
        FluidBoundaryCondition::Periodic => AcousticFluidGpuBoundary::Periodic,
        FluidBoundaryCondition::Rigid
        | FluidBoundaryCondition::EngquistMajdaFirstOrder
        | FluidBoundaryCondition::Cpml { .. } => AcousticFluidGpuBoundary::Rigid,
        FluidBoundaryCondition::PressureRelease => AcousticFluidGpuBoundary::PressureRelease,
    };
    Ok(AcousticFluidFdtdGpuParams {
        dims: [cfg.dims[0] as u32, cfg.dims[1] as u32, cfg.dims[2] as u32],
        spacing_m: [
            cfg.spacing_m[0] as f32,
            cfg.spacing_m[1] as f32,
            cfg.spacing_m[2] as f32,
        ],
        dt_s: cfg.dt_s as f32,
        density_kg_m3: cfg.medium.density_kg_m3 as f32,
        bulk_modulus_pa: cfg.medium.bulk_modulus_pa as f32,
        damping_per_s: cfg.medium.damping_per_s as f32,
        boundary,
    })
}

/// Pack a [`FluidFdtdState`] into the flat `f32` storage buffer layout (`[p, vx, vy, vz]` per cell)
/// consumed by [`ACOUSTIC_FLUID_FDTD_WGSL`].
#[cfg(feature = "gpu-runtime")]
pub fn pack_fluid_fdtd_state_f32(state: &FluidFdtdState) -> Vec<f32> {
    let n = state.p.len();
    let mut out = Vec::with_capacity(n * ACOUSTIC_FLUID_CELL_STRIDE);
    for i in 0..n {
        out.push(state.p[i] as f32);
        out.push(state.vx[i] as f32);
        out.push(state.vy[i] as f32);
        out.push(state.vz[i] as f32);
    }
    out
}

/// Unpack a flat `f32` storage buffer (`[p, vx, vy, vz]` per cell) back into a [`FluidFdtdState`].
#[cfg(feature = "gpu-runtime")]
pub fn unpack_fluid_fdtd_state_f32(
    flat: &[f32],
    state: &mut FluidFdtdState,
) -> Result<(), NumericalError> {
    let n = state.p.len();
    if flat.len() != n * ACOUSTIC_FLUID_CELL_STRIDE {
        return Err(NumericalError::BufferLengthMismatch {
            buffer: "unpack_fluid_fdtd_state_f32",
            expected: n * ACOUSTIC_FLUID_CELL_STRIDE,
            actual: flat.len(),
        });
    }
    state.prev_vx.copy_from_slice(&state.vx);
    state.prev_vy.copy_from_slice(&state.vy);
    state.prev_vz.copy_from_slice(&state.vz);
    for i in 0..n {
        let base = i * ACOUSTIC_FLUID_CELL_STRIDE;
        state.p[i] = flat[base] as f64;
        state.vx[i] = flat[base + 1] as f64;
        state.vy[i] = flat[base + 2] as f64;
        state.vz[i] = flat[base + 3] as f64;
    }
    state.step_index += 1;
    Ok(())
}
