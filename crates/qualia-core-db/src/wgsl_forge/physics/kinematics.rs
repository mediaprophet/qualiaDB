//! Softened inverse-square N-body step as a certified forge kernel.
//!
//! Embeds [`shaders/kinematics.wgsl`](../../../shaders/kinematics.wgsl) via
//! `include_str!` (single source of truth), grades it against the exact CPU oracle
//! [`nbody_step_cpu`], and runs it on any wgpu adapter via [`nbody_step_gpu`].
//!
//! State is a flat `f32` buffer, 8 scalars per particle:
//! `[px, py, pz, vx, vy, vz, mass, charge]`. The kernel is **double-buffered**: forces
//! are read from the input state only and written to a separate output, so the result is
//! independent of invocation order (no read/write race). The pairwise force on `i` is
//! `F_i = coupling · Σ_{j≠i} q_i q_j (x_i−x_j) / (|x_i−x_j|² + soft)^{3/2}` (Plummer
//! softening, no singular skip branch), then a symplectic-Euler update
//! `v ← v + (F/m)dt`, `x ← x + v·dt`. `coupling` selects the law: `+k` electrostatic
//! (repulsive like-charges), `−G` gravitational (put mass in the charge slot).

use crate::wgsl_forge::ForgeError;

/// The N-body step kernel source (embedded from the canonical `.wgsl`).
pub const KIN_STEP_WGSL: &str = include_str!("../../shaders/kinematics.wgsl");
/// Entry-point name of [`KIN_STEP_WGSL`].
pub const KIN_STEP_ENTRY: &str = "nbody_step";
/// Scalars per particle in the flat state buffer.
pub const KIN_STRIDE: usize = 8;

/// Exact CPU oracle for one N-body step. Reads `state_in`, returns the new state (same
/// length), mirroring the WGSL scalar-for-scalar: forces accumulated in increasing-`j`
/// order (skipping self), `1/(r²+soft)^{3/2}` via `r2 * sqrt(r2)`, then `v` then `x`
/// using the new `v`. Particles with `mass == 0` take a zero inverse mass.
pub fn nbody_step_cpu(state_in: &[f32], dt: f32, soft: f32, coupling: f32) -> Vec<f32> {
    let count = state_in.len() / KIN_STRIDE;
    let mut out = vec![0.0f32; state_in.len()];
    for i in 0..count {
        let bi = i * KIN_STRIDE;
        let pix = state_in[bi];
        let piy = state_in[bi + 1];
        let piz = state_in[bi + 2];
        let qi = state_in[bi + 7];

        let mut fx = 0.0f32;
        let mut fy = 0.0f32;
        let mut fz = 0.0f32;
        for j in 0..count {
            if j == i {
                continue;
            }
            let bj = j * KIN_STRIDE;
            let rx = pix - state_in[bj];
            let ry = piy - state_in[bj + 1];
            let rz = piz - state_in[bj + 2];
            let r2 = rx * rx + ry * ry + rz * rz + soft;
            let inv = coupling * qi * state_in[bj + 7] / (r2 * r2.sqrt());
            fx += rx * inv;
            fy += ry * inv;
            fz += rz * inv;
        }

        let mass = state_in[bi + 6];
        let inv_m = if mass != 0.0 { 1.0 / mass } else { 0.0 };
        let vx = state_in[bi + 3] + fx * inv_m * dt;
        let vy = state_in[bi + 4] + fy * inv_m * dt;
        let vz = state_in[bi + 5] + fz * inv_m * dt;

        out[bi] = pix + vx * dt;
        out[bi + 1] = piy + vy * dt;
        out[bi + 2] = piz + vz * dt;
        out[bi + 3] = vx;
        out[bi + 4] = vy;
        out[bi + 5] = vz;
        out[bi + 6] = mass;
        out[bi + 7] = qi;
    }
    out
}

/// Run one N-body step on the GPU and read back the new flat state. Builds a transient
/// wgpu context, uploads `state_in` (binding 0, read), a zeroed output (binding 1,
/// read_write) and `params = [dt, soft, coupling]` (binding 2, read), dispatches one
/// invocation per particle, and reads the output back. Returns the same length as
/// `state_in`.
pub fn nbody_step_gpu(
    state_in: &[f32],
    dt: f32,
    soft: f32,
    coupling: f32,
) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::execute::{
        BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
    };
    use crate::wgsl_forge::Schedule;

    if state_in.is_empty() || state_in.len() % KIN_STRIDE != 0 {
        return Err(ForgeError::GpuValidation(format!(
            "nbody_step_gpu: state length {} is not a non-zero multiple of {KIN_STRIDE}",
            state_in.len()
        )));
    }
    let count = state_in.len() / KIN_STRIDE;
    let capacity = (state_in.len() * 8).max(4 << 20);
    let mut ctx = WgpuComputeContext::new(capacity)?;

    let view_in = ctx.allocate_and_write(
        bytemuck::cast_slice(state_in),
        0,
        0,
        BindingUsage::StorageRead,
    )?;
    let zeros = vec![0.0f32; state_in.len()];
    let view_out = ctx.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        1,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let params = [dt, soft, coupling];
    let view_params = ctx.allocate_and_write(
        bytemuck::cast_slice(&params),
        2,
        0,
        BindingUsage::StorageRead,
    )?;

    let buffers = vec![view_in, view_out, view_params];
    let pipeline = WgpuPipeline::compile(&ctx, KIN_STEP_WGSL, KIN_STEP_ENTRY)?;
    let schedule = Schedule {
        workgroup_size: 64,
        ..Default::default()
    };
    pipeline.dispatch(&buffers, &schedule, count)?;
    let mut out = ctx.read_buffer_f32(&view_out)?;
    out.truncate(state_in.len());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::validate::validate_wgsl;

    /// The N-body kernel must naga-validate and expose the `nbody_step` entry point.
    #[test]
    fn kinematics_wgsl_validates() {
        let report = validate_wgsl(KIN_STEP_WGSL).expect("kinematics WGSL must naga-validate");
        assert!(
            report.entry_points.iter().any(|e| e == KIN_STEP_ENTRY),
            "validated module must expose {KIN_STEP_ENTRY}; got {:?}",
            report.entry_points
        );
    }

    /// Two equal positive charges on the x-axis must repel: with `coupling = +1`, the
    /// left particle ends up moving in `−x` and the right in `+x`, so they separate.
    #[test]
    fn nbody_oracle_like_charges_repel() {
        // p0 at x=0, p1 at x=1; both q=+1, m=1, at rest.
        let state = vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, // particle 0
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, // particle 1
        ];
        let out = nbody_step_cpu(&state, 0.1, 1e-4, 1.0);
        // Particle 0 (left) pushed −x → new x < 0; particle 1 (right) pushed +x → x > 1.
        assert!(out[0] < 0.0, "left particle should move −x, got {}", out[0]);
        assert!(
            out[8] > 1.0,
            "right particle should move +x, got {}",
            out[8]
        );
        // x-velocities are equal and opposite (symmetric two-body).
        assert!((out[3] + out[11]).abs() < 1e-5, "momentum not conserved");
        // Mass/charge slots carried through unchanged.
        assert_eq!(out[6], 1.0);
        assert_eq!(out[7], 1.0);
    }

    /// Opposite charges attract: `q0=+1, q1=−1, coupling=+1` → they move together.
    #[test]
    fn nbody_oracle_opposite_charges_attract() {
        let state = vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, //
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, -1.0, //
        ];
        let out = nbody_step_cpu(&state, 0.1, 1e-4, 1.0);
        assert!(out[0] > 0.0, "left particle should move +x, got {}", out[0]);
        assert!(
            out[8] < 1.0,
            "right particle should move −x, got {}",
            out[8]
        );
    }

    /// GPU certify: the kernel on a real adapter must match the CPU oracle within f32
    /// tolerance over a deterministic multi-particle scene. Run by the orchestrator.
    #[test]
    #[serial_test::serial(gpu)]
    fn nbody_gpu_matches_oracle() {
        if !crate::wgsl_forge::test_gpu_available() { return; }
        let count = 128usize;
        let mut state = Vec::with_capacity(count * KIN_STRIDE);
        for i in 0..count {
            let f = i as f32;
            state.extend_from_slice(&[
                (f * 0.21) - 12.0,
                (f * 0.13) - 8.0,
                (f * 0.07) - 4.0,
                0.0,
                0.0,
                0.0,
                1.0 + (f % 3.0),
                if i % 2 == 0 { 1.0 } else { -1.0 },
            ]);
        }
        let (dt, soft, coupling) = (0.005f32, 1e-2, 1.0);
        let expected = nbody_step_cpu(&state, dt, soft, coupling);
        let gpu = nbody_step_gpu(&state, dt, soft, coupling).expect("nbody_step_gpu");
        assert_eq!(gpu.len(), expected.len());
        for (g, e) in gpu.iter().zip(expected.iter()) {
            let tol = 1e-3 * e.abs().max(1.0);
            assert!((g - e).abs() <= tol, "GPU/CPU mismatch: {g} vs {e}");
        }
    }
}
