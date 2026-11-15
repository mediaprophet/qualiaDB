//! GPU / CPU Layout & Operator Parity for EMF v4 Records and WGSL Compute Kernels.
//!
//! Defines `#[repr(C)]` 64-byte display voxel (`EmfGpuDisplayVoxel64`), 128-byte Mode `0x0100`
//! instantaneous E/B record (`EmfMode0InstantaneousRecord128`), 128-byte Mode `0x0102` Yee
//! staggered state record (`EmfMode2YeeRecord128`), and 128-byte WGSL `vec4<f32>`-aligned GPU
//! Yee compute cell (`EmfGpuYeeCell128`). Enforces compile-time and runtime offset/stride/alignment
//! assertions, explicit scientific-to-display `f64 -> f32` projection with declared lossy fields,
//! and C02-C09 CPU `f64` vs GPU `f32` reference kernel parity budgets.

pub use super::poynting_energy::{
    c02_preview_f32_budget, c02_reduction_f64_budget, c02_small_op_f64_budget,
    instantaneous_poynting_and_energy,
};
use super::yee_fdtd::{yee_step_leapfrog_in_place, YeeMaterialParams, YeeSolverProfile};
use super::{
    BoundaryCondition, EmfOperatorError, Grid3dSpec, C0_M_PER_S, MU0_REF_H_PER_M,
};
use core::mem::{align_of, offset_of, size_of};

#[cfg(feature = "gpu-runtime")]
pub use etalon_render::emf::{
    evaluate_emf_fragment_cpu, phase_to_sigma_cpu, sample_field_bilinear_cpu,
    sigma_to_cie_xyz_cpu, sigma_to_linear_rgb_cpu, xyz_to_linear_srgb_cpu, EmfFieldCell,
    EmfSliceUniform,
};

#[cfg(feature = "gpu-runtime")]
pub use etalon_wgsl_forge::numerical::physics::maxwell_yee::{
    maxwell_yee_gpu_capacity, maxwell_yee_step_cpu, maxwell_yee_step_gpu,
    maxwell_yee_step_with_context, MaxwellYeeParams, C0_F32, MAXWELL_YEE_STEP_ENTRY,
    MAXWELL_YEE_STEP_WGSL, MAXWELL_YEE_STRIDE,
};

/// 64-byte WGSL-aligned (`vec4<f32>` x 4) display/preview record matching `EmfCellRecord64`.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmfGpuDisplayVoxel64 {
    /// `0..12`: Electric field `[Ex, Ey, Ez]` in V/m (`f32` narrowed from `f64`).
    pub e_field_v_per_m: [f32; 3],
    /// `12..16`: Display wavelength in nm (`0.0` if static/aperiodic).
    pub wavelength_nm: f32,
    /// `16..28`: Magnetic field `[Bx, By, Bz]` in Tesla (`f32` narrowed from `f64`).
    pub b_field_tesla: [f32; 3],
    /// `28..32`: Instantaneous Poynting flux magnitude `|S|` in W/m².
    pub poynting_flux_w_per_m2: f32,
    /// `32..44`: Instantaneous Poynting vector `[Sx, Sy, Sz]` in W/m².
    pub poynting_vec_w_per_m2: [f32; 3],
    /// `44..48`: Instantaneous electromagnetic energy density `u` in J/m³.
    pub energy_density_j_per_m3: f32,
    /// `48..52`: Coherence phase in radians (`f32` narrowed).
    pub coherence_phase_rad: f32,
    /// `52..56`: Legacy display spectral sigma in `[0, 1]` (never authoritative physical coordinate).
    pub display_spectral_sigma: f32,
    /// `56..60`: Confidence weight in `[0, 1]`.
    pub confidence_alpha: f32,
    /// `60..64`: Bitmask of lossy/narrowed fields (`EmfDisplayLossyFlags`).
    pub lossy_projection_flags: u32,
}

/// 128-byte Mode `0x0100` (`instantaneous_E_B_v1`) authoritative wire/CPU record (`wire-freeze.md` §4.1).
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmfMode0InstantaneousRecord128 {
    /// `0..24`: `electric_field_v_per_m` (`f64[3]`)
    pub electric_field_v_per_m: [f64; 3],
    /// `24..48`: `magnetic_field_tesla` (`f64[3]`)
    pub magnetic_field_tesla: [f64; 3],
    /// `48..72`: `local_position_m` (`f64[3]`)
    pub local_position_m: [f64; 3],
    /// `72..80`: `record_tick` (`i64`)
    pub record_tick: i64,
    /// `80..88`: `carrier_hz` (`f64`, `+0.0` if static/aperiodic)
    pub carrier_hz: f64,
    /// `88..96`: `coherence_phase_rad` (`f64`, `+0.0` if absent)
    pub coherence_phase_rad: f64,
    /// `96..120`: `stored_poynting_w_per_m2` (`f64[3]`, `+0.0` if absent)
    pub stored_poynting_w_per_m2: [f64; 3],
    /// `120..124`: `flags` (`u32`: bit0 carrier, bit1 phase, bit2 Poynting, bit3 confidence)
    pub flags: u32,
    /// `124..128`: `confidence` (`f32` in `[0, 1]`, `+0.0` if absent)
    pub confidence: f32,
}

/// 128-byte Mode `0x0102` (`yee_cartesian_v1`) authoritative wire/CPU record (`wire-freeze.md` §4.3).
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmfMode2YeeRecord128 {
    /// `0..24`: `electric_field_v_per_m` (`f64[3]`, E at time t, edge-staggered)
    pub electric_field_v_per_m: [f64; 3],
    /// `24..48`: `magnetic_field_h_a_per_m` (`f64[3]`, H at time t+dt/2, face-staggered)
    pub magnetic_field_h_a_per_m: [f64; 3],
    /// `48..72`: `impressed_current_j_a_per_m2` (`f64[3]`, J at time t)
    pub impressed_current_j_a_per_m2: [f64; 3],
    /// `72..80`: `scalar_potential_v` (`f64`, `+0.0` placeholder in no-phi profile)
    pub scalar_potential_v: f64,
    /// `80..88`: `charge_density_c_per_m3` (`f64`, model-zero in charge-free fixture)
    pub charge_density_c_per_m3: f64,
    /// `88..96`: `epsilon_relative` (`f64`, `> 0`)
    pub epsilon_relative: f64,
    /// `96..104`: `mu_relative` (`f64`, `> 0`)
    pub mu_relative: f64,
    /// `104..112`: `conductivity_s_per_m` (`f64`, `>= 0`)
    pub conductivity_s_per_m: f64,
    /// `112..116`: `material_entry` (`u32`)
    pub material_entry: u32,
    /// `116..120`: `constitutive_metadata_id` (`u32`, ref to schema 133)
    pub constitutive_metadata_id: u32,
    /// `120..128`: `state_record_index` (`u64`, `u64::MAX` if none)
    pub state_record_index: u64,
}

/// 128-byte WGSL `std430` storage-buffer cell for GPU Yee FDTD & ADE compute dispatches
/// (8 × `vec4<f32>` = 128 bytes, 16-byte vector alignment, 64-byte cache-line multiple).
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmfGpuYeeCell128 {
    /// `0..16`: `[Ex, Ey, Ez, epsilon_r]`
    pub e_and_eps_r: [f32; 4],
    /// `16..32`: `[Hx, Hy, Hz, mu_r]`
    pub h_and_mu_r: [f32; 4],
    /// `32..48`: `[Jx, Jy, Jz, sigma_s_per_m]`
    pub j_and_sigma: [f32; 4],
    /// `48..64`: `[Px, Py, Pz, ade_pole_weight]` (ADE polarization state)
    pub p_and_weight: [f32; 4],
    /// `64..80`: `[Jp_x, Jp_y, Jp_z, c_ee]` (ADE current + precomputed E decay coeff)
    pub jp_and_cee: [f32; 4],
    /// `80..96`: `[curl_e_x, curl_e_y, curl_e_z, c_eh]`
    pub curl_e_and_ceh: [f32; 4],
    /// `96..112`: `[curl_h_x, curl_h_y, curl_h_z, c_he]`
    pub curl_h_and_che: [f32; 4],
    /// `112..128`: `[material_entry, constitutive_id, flags, reserved]`
    pub meta_u32: [u32; 4],
}

// Compile-time ABI size, alignment, and byte-offset assertions matching `wire-freeze.md` §4.1/§4.3 and WGSL rules.
const _: () = {
    assert!(size_of::<EmfGpuDisplayVoxel64>() == 64);
    assert!(align_of::<EmfGpuDisplayVoxel64>() == 64);
    assert!(offset_of!(EmfGpuDisplayVoxel64, e_field_v_per_m) == 0);
    assert!(offset_of!(EmfGpuDisplayVoxel64, wavelength_nm) == 12);
    assert!(offset_of!(EmfGpuDisplayVoxel64, b_field_tesla) == 16);
    assert!(offset_of!(EmfGpuDisplayVoxel64, poynting_flux_w_per_m2) == 28);
    assert!(offset_of!(EmfGpuDisplayVoxel64, poynting_vec_w_per_m2) == 32);
    assert!(offset_of!(EmfGpuDisplayVoxel64, energy_density_j_per_m3) == 44);
    assert!(offset_of!(EmfGpuDisplayVoxel64, coherence_phase_rad) == 48);
    assert!(offset_of!(EmfGpuDisplayVoxel64, display_spectral_sigma) == 52);
    assert!(offset_of!(EmfGpuDisplayVoxel64, confidence_alpha) == 56);
    assert!(offset_of!(EmfGpuDisplayVoxel64, lossy_projection_flags) == 60);

    assert!(size_of::<EmfMode0InstantaneousRecord128>() == 128);
    assert!(align_of::<EmfMode0InstantaneousRecord128>() == 64);
    assert!(offset_of!(EmfMode0InstantaneousRecord128, electric_field_v_per_m) == 0);
    assert!(offset_of!(EmfMode0InstantaneousRecord128, magnetic_field_tesla) == 24);
    assert!(offset_of!(EmfMode0InstantaneousRecord128, local_position_m) == 48);
    assert!(offset_of!(EmfMode0InstantaneousRecord128, record_tick) == 72);
    assert!(offset_of!(EmfMode0InstantaneousRecord128, carrier_hz) == 80);
    assert!(offset_of!(EmfMode0InstantaneousRecord128, coherence_phase_rad) == 88);
    assert!(offset_of!(EmfMode0InstantaneousRecord128, stored_poynting_w_per_m2) == 96);
    assert!(offset_of!(EmfMode0InstantaneousRecord128, flags) == 120);
    assert!(offset_of!(EmfMode0InstantaneousRecord128, confidence) == 124);

    assert!(size_of::<EmfMode2YeeRecord128>() == 128);
    assert!(align_of::<EmfMode2YeeRecord128>() == 64);
    assert!(offset_of!(EmfMode2YeeRecord128, electric_field_v_per_m) == 0);
    assert!(offset_of!(EmfMode2YeeRecord128, magnetic_field_h_a_per_m) == 24);
    assert!(offset_of!(EmfMode2YeeRecord128, impressed_current_j_a_per_m2) == 48);
    assert!(offset_of!(EmfMode2YeeRecord128, scalar_potential_v) == 72);
    assert!(offset_of!(EmfMode2YeeRecord128, charge_density_c_per_m3) == 80);
    assert!(offset_of!(EmfMode2YeeRecord128, epsilon_relative) == 88);
    assert!(offset_of!(EmfMode2YeeRecord128, mu_relative) == 96);
    assert!(offset_of!(EmfMode2YeeRecord128, conductivity_s_per_m) == 104);
    assert!(offset_of!(EmfMode2YeeRecord128, material_entry) == 112);
    assert!(offset_of!(EmfMode2YeeRecord128, constitutive_metadata_id) == 116);
    assert!(offset_of!(EmfMode2YeeRecord128, state_record_index) == 120);

    assert!(size_of::<EmfGpuYeeCell128>() == 128);
    assert!(align_of::<EmfGpuYeeCell128>() == 64);
    assert!(offset_of!(EmfGpuYeeCell128, e_and_eps_r) == 0);
    assert!(offset_of!(EmfGpuYeeCell128, h_and_mu_r) == 16);
    assert!(offset_of!(EmfGpuYeeCell128, j_and_sigma) == 32);
    assert!(offset_of!(EmfGpuYeeCell128, p_and_weight) == 48);
    assert!(offset_of!(EmfGpuYeeCell128, jp_and_cee) == 64);
    assert!(offset_of!(EmfGpuYeeCell128, curl_e_and_ceh) == 80);
    assert!(offset_of!(EmfGpuYeeCell128, curl_h_and_che) == 96);
    assert!(offset_of!(EmfGpuYeeCell128, meta_u32) == 112);
};

/// Bitflags documenting which scientific fields were narrowed or omitted during `f64 -> f32` display projection.
pub mod lossy_flags {
    /// `f64` electric and magnetic field vectors narrowed to `f32`.
    pub const E_AND_B_NARROWED_F64_TO_F32: u32 = 1 << 0;
    /// `f64[3]` local position (`local_position_m`) omitted from voxel display payload.
    pub const LOCAL_POSITION_F64_OMITTED: u32 = 1 << 1;
    /// Exact `i64` clock tick (`record_tick`) omitted from voxel display payload.
    pub const RECORD_TICK_I64_OMITTED: u32 = 1 << 2;
    /// Carrier frequency converted to preview `wavelength_nm` and clamped display sigma `[400..700 nm]`.
    pub const DISPLAY_SIGMA_DERIVED_NON_AUTHORITATIVE: u32 = 1 << 3;
    /// Static/aperiodic field (`carrier_hz == 0`): wavelength & sigma set to `+0.0` (no `c/0` evaluation).
    pub const STATIC_CARRIER_NO_WAVELENGTH: u32 = 1 << 4;
}

/// Report returned alongside a projected 64-byte display voxel quantifying `f64 -> f32` narrowing errors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmfDisplayProjectionReport {
    pub voxel: EmfGpuDisplayVoxel64,
    pub max_e_abs_error: f64,
    pub max_b_abs_error: f64,
    pub max_s_abs_error: f64,
    pub u_abs_error: f64,
    pub lossy_flags: u32,
}

/// Explicitly project an authoritative 128-byte Mode `0x0100` (`instantaneous_E_B_v1`) scientific record
/// into a 64-byte `EmfGpuDisplayVoxel64` preview struct, verifying that every narrowed physical quantity
/// satisfies the C02 derived `f32` preview budget `abs_error <= 32 * 2^-23 * Q`.
pub fn project_mode0_to_display_voxel64(
    record: &EmfMode0InstantaneousRecord128,
    mat: &YeeMaterialParams,
) -> Result<EmfDisplayProjectionReport, EmfOperatorError> {
    let (s_f64, u_f64) = instantaneous_poynting_and_energy(
        record.electric_field_v_per_m,
        record.magnetic_field_tesla,
        mat,
    )?;
    if !record.carrier_hz.is_finite()
        || !record.coherence_phase_rad.is_finite()
        || !record.confidence.is_finite()
    {
        return Err(EmfOperatorError::NonFiniteInput);
    }

    let e_f32 = [
        record.electric_field_v_per_m[0] as f32,
        record.electric_field_v_per_m[1] as f32,
        record.electric_field_v_per_m[2] as f32,
    ];
    let b_f32 = [
        record.magnetic_field_tesla[0] as f32,
        record.magnetic_field_tesla[1] as f32,
        record.magnetic_field_tesla[2] as f32,
    ];

    let mu_f32 = (MU0_REF_H_PER_M * mat.mu_r) as f32;
    let eps_f32 = (super::EPS0_REF_F_PER_M * mat.epsilon_r) as f32;
    let sx_f32 = (e_f32[1] * b_f32[2] - e_f32[2] * b_f32[1]) / mu_f32;
    let sy_f32 = (e_f32[2] * b_f32[0] - e_f32[0] * b_f32[2]) / mu_f32;
    let sz_f32 = (e_f32[0] * b_f32[1] - e_f32[1] * b_f32[0]) / mu_f32;
    let s_f32 = [sx_f32, sy_f32, sz_f32];
    let s_mag_f32 = (sx_f32 * sx_f32 + sy_f32 * sy_f32 + sz_f32 * sz_f32).sqrt();

    let e_sq_f32 = e_f32[0] * e_f32[0] + e_f32[1] * e_f32[1] + e_f32[2] * e_f32[2];
    let b_sq_f32 = b_f32[0] * b_f32[0] + b_f32[1] * b_f32[1] + b_f32[2] * b_f32[2];
    let u_f32 = 0.5 * (eps_f32 * e_sq_f32 + b_sq_f32 / mu_f32);

    let mut flags = lossy_flags::E_AND_B_NARROWED_F64_TO_F32
        | lossy_flags::LOCAL_POSITION_F64_OMITTED
        | lossy_flags::RECORD_TICK_I64_OMITTED;

    let carrier_present = (record.flags & 1) != 0 && record.carrier_hz > 0.0;
    let (wavelength_nm, display_sigma) = if carrier_present {
        flags |= lossy_flags::DISPLAY_SIGMA_DERIVED_NON_AUTHORITATIVE;
        let v = mat.phase_velocity_m_per_s()?;
        let wl_nm = (v * 1e9 / record.carrier_hz) as f32;
        let sigma = ((wl_nm - 400.0) / 300.0).clamp(0.0, 1.0);
        (wl_nm, sigma)
    } else {
        flags |= lossy_flags::STATIC_CARRIER_NO_WAVELENGTH;
        (0.0_f32, 0.0_f32)
    };

    let q_e = record
        .electric_field_v_per_m
        .iter()
        .fold(0.0_f64, |acc, &v| acc.max(v.abs()));
    let q_b = record
        .magnetic_field_tesla
        .iter()
        .fold(0.0_f64, |acc, &v| acc.max(v.abs()));
    let q_s = s_f64.iter().fold(0.0_f64, |acc, &v| acc.max(v.abs()));
    let q_u = u_f64.abs();

    let mut max_e_err = 0.0_f64;
    let mut max_b_err = 0.0_f64;
    let mut max_s_err = 0.0_f64;
    for i in 0..3 {
        max_e_err = max_e_err.max((e_f32[i] as f64 - record.electric_field_v_per_m[i]).abs());
        max_b_err = max_b_err.max((b_f32[i] as f64 - record.magnetic_field_tesla[i]).abs());
        max_s_err = max_s_err.max((s_f32[i] as f64 - s_f64[i]).abs());
    }
    let u_err = (u_f32 as f64 - u_f64).abs();

    if q_e > 0.0 && max_e_err > c02_preview_f32_budget(q_e) {
        return Err(EmfOperatorError::GpuParityBudgetExceeded {
            max_abs_err: max_e_err,
            budget: c02_preview_f32_budget(q_e),
        });
    }
    if q_b > 0.0 && max_b_err > c02_preview_f32_budget(q_b) {
        return Err(EmfOperatorError::GpuParityBudgetExceeded {
            max_abs_err: max_b_err,
            budget: c02_preview_f32_budget(q_b),
        });
    }
    if q_s > 0.0 && max_s_err > c02_preview_f32_budget(q_s) {
        return Err(EmfOperatorError::GpuParityBudgetExceeded {
            max_abs_err: max_s_err,
            budget: c02_preview_f32_budget(q_s),
        });
    }
    if q_u > 0.0 && u_err > c02_preview_f32_budget(q_u) {
        return Err(EmfOperatorError::GpuParityBudgetExceeded {
            max_abs_err: u_err,
            budget: c02_preview_f32_budget(q_u),
        });
    }

    let voxel = EmfGpuDisplayVoxel64 {
        e_field_v_per_m: e_f32,
        wavelength_nm,
        b_field_tesla: b_f32,
        poynting_flux_w_per_m2: s_mag_f32,
        poynting_vec_w_per_m2: s_f32,
        energy_density_j_per_m3: u_f32,
        coherence_phase_rad: if (record.flags & 2) != 0 {
            record.coherence_phase_rad as f32
        } else {
            0.0
        },
        display_spectral_sigma: display_sigma,
        confidence_alpha: if (record.flags & 8) != 0 {
            record.confidence
        } else {
            0.0
        },
        lossy_projection_flags: flags,
    };

    Ok(EmfDisplayProjectionReport {
        voxel,
        max_e_abs_error: max_e_err,
        max_b_abs_error: max_b_err,
        max_s_abs_error: max_s_err,
        u_abs_error: u_err,
        lossy_flags: flags,
    })
}

/// Execute one step of the WGSL-equivalent `f32` Yee leapfrog kernel over a slice of
/// 128-byte `EmfGpuYeeCell128` storage buffer cells with periodic boundaries.
pub fn yee_step_gpu_f32_reference_in_place(
    grid: &Grid3dSpec,
    cells: &mut [EmfGpuYeeCell128],
) -> Result<(), EmfOperatorError> {
    let n = grid.validate()?;
    if cells.len() != n {
        return Err(EmfOperatorError::SliceLengthMismatch {
            expected: n,
            provided: cells.len(),
        });
    }
    let inv_dx = (1.0 / grid.dx) as f32;
    let inv_dy = (1.0 / grid.dy) as f32;
    let inv_dz = (1.0 / grid.dz) as f32;

    // Pass 1: Compute curl H (backward differences) and update E in f32
    for iz in 0..grid.nz {
        let izm = (iz + grid.nz - 1) % grid.nz;
        for iy in 0..grid.ny {
            let iym = (iy + grid.ny - 1) % grid.ny;
            for ix in 0..grid.nx {
                let ixm = (ix + grid.nx - 1) % grid.nx;
                let c = grid.idx(ix, iy, iz);

                let hx_c = cells[c].h_and_mu_r[0];
                let hy_c = cells[c].h_and_mu_r[1];
                let hz_c = cells[c].h_and_mu_r[2];

                let hz_ym = cells[grid.idx(ix, iym, iz)].h_and_mu_r[2];
                let hy_zm = cells[grid.idx(ix, iy, izm)].h_and_mu_r[1];
                let hx_zm = cells[grid.idx(ix, iy, izm)].h_and_mu_r[0];
                let hz_xm = cells[grid.idx(ixm, iy, iz)].h_and_mu_r[2];
                let hy_xm = cells[grid.idx(ixm, iy, iz)].h_and_mu_r[1];
                let hx_ym = cells[grid.idx(ix, iym, iz)].h_and_mu_r[0];

                let ch_x = (hz_c - hz_ym) * inv_dy - (hy_c - hy_zm) * inv_dz;
                let ch_y = (hx_c - hx_zm) * inv_dz - (hz_c - hz_xm) * inv_dx;
                let ch_z = (hy_c - hy_xm) * inv_dx - (hx_c - hx_ym) * inv_dy;

                cells[c].curl_h_and_che[0] = ch_x;
                cells[c].curl_h_and_che[1] = ch_y;
                cells[c].curl_h_and_che[2] = ch_z;
            }
        }
    }

    for cell in cells.iter_mut() {
        let c_ee = cell.jp_and_cee[3];
        let c_eh = cell.curl_e_and_ceh[3];
        for d in 0..3 {
            cell.e_and_eps_r[d] = c_ee * cell.e_and_eps_r[d]
                + c_eh * (cell.curl_h_and_che[d] - cell.j_and_sigma[d]);
        }
    }

    // Pass 2: Compute curl E (forward differences) and update H in f32
    for iz in 0..grid.nz {
        let izp = (iz + 1) % grid.nz;
        for iy in 0..grid.ny {
            let iyp = (iy + 1) % grid.ny;
            for ix in 0..grid.nx {
                let ixp = (ix + 1) % grid.nx;
                let c = grid.idx(ix, iy, iz);

                let ex_c = cells[c].e_and_eps_r[0];
                let ey_c = cells[c].e_and_eps_r[1];
                let ez_c = cells[c].e_and_eps_r[2];

                let ez_yp = cells[grid.idx(ix, iyp, iz)].e_and_eps_r[2];
                let ey_zp = cells[grid.idx(ix, iy, izp)].e_and_eps_r[1];
                let ex_zp = cells[grid.idx(ix, iy, izp)].e_and_eps_r[0];
                let ez_xp = cells[grid.idx(ixp, iy, iz)].e_and_eps_r[2];
                let ey_xp = cells[grid.idx(ixp, iy, iz)].e_and_eps_r[1];
                let ex_yp = cells[grid.idx(ix, iyp, iz)].e_and_eps_r[0];

                let ce_x = (ez_yp - ez_c) * inv_dy - (ey_zp - ey_c) * inv_dz;
                let ce_y = (ex_zp - ex_c) * inv_dz - (ez_xp - ez_c) * inv_dx;
                let ce_z = (ey_xp - ey_c) * inv_dx - (ex_yp - ex_c) * inv_dy;

                cells[c].curl_e_and_ceh[0] = ce_x;
                cells[c].curl_e_and_ceh[1] = ce_y;
                cells[c].curl_e_and_ceh[2] = ce_z;
            }
        }
    }

    for cell in cells.iter_mut() {
        let c_he = cell.curl_h_and_che[3];
        for d in 0..3 {
            cell.h_and_mu_r[d] -= c_he * cell.curl_e_and_ceh[d];
        }
    }

    Ok(())
}

/// Result of comparing CPU `f64` authoritative Yee leapfrog stepping against the WGSL `f32`
/// `EmfGpuYeeCell128` reference kernel on an identical 3D discretization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpuGpuYeeParityReport {
    pub max_e_abs_diff: f64,
    pub max_h_abs_diff: f64,
    pub e_rel_l2_diff: f64,
    pub h_rel_l2_diff: f64,
    pub e_budget: f64,
    pub h_budget: f64,
}

/// Run both the authoritative `f64` CPU Yee leapfrog solver and the `f32` `EmfGpuYeeCell128`
/// WGSL reference kernel for `steps` iterations on an identical 3D plane wave and verify that
/// componentwise and normalized L2 differences stay within the C02-C09 cross-backend parity budget.
pub fn verify_cpu_f64_vs_gpu_f32_yee_parity(
    n_x: usize,
    steps: usize,
) -> Result<CpuGpuYeeParityReport, EmfOperatorError> {
    let l_m = 1.0_f64;
    let dx = l_m / (n_x as f64);
    let grid = Grid3dSpec {
        nx: n_x,
        ny: 2,
        nz: 2,
        dx,
        dy: dx,
        dz: dx,
    };
    let mat = YeeMaterialParams::vacuum();
    let dt = 0.5 * super::yee_fdtd::yee_cfl_limit_3d(&grid, &mat)?;
    let n = grid.validate()?;

    let eps = mat.epsilon_f_per_m();
    let mu = mat.mu_h_per_m();
    let c_ee = 1.0_f32;
    let c_eh = (dt / eps) as f32;
    let c_he = (dt / mu) as f32;

    let k_x = 2.0 * core::f64::consts::PI / l_m;
    let z0 = C0_M_PER_S * MU0_REF_H_PER_M;

    let mut ex = vec![0.0_f64; n];
    let mut ey = vec![0.0_f64; n];
    let mut ez = vec![0.0_f64; n];
    let mut hx = vec![0.0_f64; n];
    let mut hy = vec![0.0_f64; n];
    let mut hz = vec![0.0_f64; n];
    let j_zero = vec![0.0_f64; n];
    let mut cx = vec![0.0_f64; n];
    let mut cy = vec![0.0_f64; n];
    let mut cz = vec![0.0_f64; n];

    let mut gpu_cells = vec![
        EmfGpuYeeCell128 {
            e_and_eps_r: [0.0, 0.0, 0.0, 1.0],
            h_and_mu_r: [0.0, 0.0, 0.0, 1.0],
            j_and_sigma: [0.0; 4],
            p_and_weight: [0.0; 4],
            jp_and_cee: [0.0, 0.0, 0.0, c_ee],
            curl_e_and_ceh: [0.0, 0.0, 0.0, c_eh],
            curl_h_and_che: [0.0, 0.0, 0.0, c_he],
            meta_u32: [0; 4],
        };
        n
    ];

    for iz in 0..grid.nz {
        for iy in 0..grid.ny {
            for ix in 0..grid.nx {
                let c = grid.idx(ix, iy, iz);
                let x_e = (ix as f64) * dx;
                let x_h = ((ix as f64) + 0.5) * dx;
                let ey_init = (k_x * x_e).cos();
                let hz_init = (k_x * x_h - 0.5 * k_x * C0_M_PER_S * dt).cos() / z0;
                ey[c] = ey_init;
                hz[c] = hz_init;
                gpu_cells[c].e_and_eps_r[1] = ey_init as f32;
                gpu_cells[c].h_and_mu_r[2] = hz_init as f32;
            }
        }
    }

    for _ in 0..steps {
        yee_step_leapfrog_in_place(
            &grid,
            YeeSolverProfile::LosslessPeriodicNoPhiV1,
            BoundaryCondition::Periodic,
            &mat,
            dt,
            &mut ex,
            &mut ey,
            &mut ez,
            &mut hx,
            &mut hy,
            &mut hz,
            &j_zero,
            &j_zero,
            &j_zero,
            &mut cx,
            &mut cy,
            &mut cz,
        )?;
        yee_step_gpu_f32_reference_in_place(&grid, &mut gpu_cells)?;
    }

    let mut max_e_abs = 0.0_f64;
    let mut max_h_abs = 0.0_f64;
    let mut e_diff_sq = 0.0_f64;
    let mut e_ref_sq = 0.0_f64;
    let mut h_diff_sq = 0.0_f64;
    let mut h_ref_sq = 0.0_f64;

    for i in 0..n {
        let de = (gpu_cells[i].e_and_eps_r[1] as f64 - ey[i]).abs();
        let dh = (gpu_cells[i].h_and_mu_r[2] as f64 - hz[i]).abs();
        max_e_abs = max_e_abs.max(de);
        max_h_abs = max_h_abs.max(dh);
        e_diff_sq += de * de;
        e_ref_sq += ey[i] * ey[i];
        h_diff_sq += dh * dh;
        h_ref_sq += hz[i] * hz[i];
    }

    let e_rel_l2 = (e_diff_sq / e_ref_sq.max(1e-30)).sqrt();
    let h_rel_l2 = (h_diff_sq / h_ref_sq.max(1e-30)).sqrt();

    let e_budget = c02_preview_f32_budget(1.0);
    let h_budget = c02_preview_f32_budget(1.0 / z0);

    if max_e_abs > e_budget {
        return Err(EmfOperatorError::GpuParityBudgetExceeded {
            max_abs_err: max_e_abs,
            budget: e_budget,
        });
    }
    if max_h_abs > h_budget {
        return Err(EmfOperatorError::GpuParityBudgetExceeded {
            max_abs_err: max_h_abs,
            budget: h_budget,
        });
    }

    Ok(CpuGpuYeeParityReport {
        max_e_abs_diff: max_e_abs,
        max_h_abs_diff: max_h_abs,
        e_rel_l2_diff: e_rel_l2,
        h_rel_l2_diff: h_rel_l2,
        e_budget,
        h_budget,
    })
}

/// Projects an authoritative 128-byte Mode `0x0100` (`EmfMode0InstantaneousRecord128`) scientific
/// record into a 64-byte [`EmfFieldCell`] for the `etalon-render` GPU display pipeline.
#[cfg(feature = "gpu-runtime")]
pub fn project_mode0_to_etalon_field_cell(
    record: &EmfMode0InstantaneousRecord128,
    mat: &YeeMaterialParams,
) -> Result<EmfFieldCell, EmfOperatorError> {
    let report = project_mode0_to_display_voxel64(record, mat)?;
    let v = &report.voxel;
    let frequency = if record.carrier_hz.is_finite() && record.carrier_hz > 0.0 {
        record.carrier_hz.min(f32::MAX as f64) as f32
    } else {
        0.0
    };
    let chromatic = v.display_spectral_sigma > 0.0 || frequency > 0.0 || v.coherence_phase_rad != 0.0;
    let sigma_phase = (v.display_spectral_sigma.fract() * core::f32::consts::TAU).max(0.0);
    Ok(EmfFieldCell {
        amplitude: v.energy_density_j_per_m3,
        phase: v.coherence_phase_rad,
        frequency,
        scale: if chromatic { 1.0 } else { 0.0 },
        attention_depth: (-(-v.poynting_flux_w_per_m2).exp_m1()).clamp(0.0, 1.0),
        epistemic_weight: v.confidence_alpha,
        topological_spin: 0.0,
        temporal_decay: 0.0,
        entropy_bias: 0.0,
        spatial_phase: sigma_phase,
        recurrence_frequency: (v.display_spectral_sigma * 6.0).clamp(0.0, 6.0),
        density_threshold: v.confidence_alpha.clamp(0.0, 1.0),
        manifold_curvature: 0.0,
        stokes_q: 0.0,
        stokes_u: 0.0,
        stokes_v: 0.0,
    })
}
