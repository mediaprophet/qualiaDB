//! 3D Cartesian Yee FDTD Solver, Discrete Dispersion Oracle, and BLAKE3 Checkpoint Sealing.
//!
//! Re-exports the continuum 3D Cartesian Yee FDTD leapfrog solver, 3D CFL admission gate,
//! and analytical discrete dispersion relation from `etalon_sim::maxwell_emf::yee_fdtd`
//! per C01 ownership, and adds the `qualia-core-db` BLAKE3 (`QUALIA-YEE-CHECKPOINT-v1\0`)
//! deterministic checkpoint state commitment (`YeeCheckpointState`).

pub use etalon_sim::maxwell_emf::yee_fdtd::*;

use super::{BoundaryCondition, EmfOperatorError, Grid3dSpec};

/// Complete deterministic checkpoint state for a 3D Yee FDTD simulation.
///
/// Per C02 Mode 2 (`EMF-A02`), a valid restart requires complete field slices, complete
/// boundary state, complete auxiliary dispersive state (or explicit lossless declaration),
/// explicit no-phi availability metadata, and a verified `BLAKE3` (`QUALIA-YEE-CHECKPOINT-v1\0`)
/// digest commitment.
#[derive(Clone, Debug, PartialEq)]
pub struct YeeCheckpointState {
    pub profile: YeeSolverProfile,
    pub boundary: BoundaryCondition,
    pub grid: Grid3dSpec,
    pub material: YeeMaterialParams,
    pub dt: f64,
    pub step_index: u64,
    pub has_complete_boundary_state: bool,
    pub has_complete_auxiliary_state: bool,
    pub phi_unavailable_declared: bool,
    pub ex: Vec<f64>,
    pub ey: Vec<f64>,
    pub ez: Vec<f64>,
    pub hx: Vec<f64>,
    pub hy: Vec<f64>,
    pub hz: Vec<f64>,
    pub jx: Vec<f64>,
    pub jy: Vec<f64>,
    pub jz: Vec<f64>,
    pub state_blake3: [u8; 32],
}

impl YeeCheckpointState {
    /// Compute the deterministic BLAKE3-256 commitment over all checkpoint parameters and IEEE-754 field bits.
    pub fn compute_blake3(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"QUALIA-YEE-CHECKPOINT-v1\0");
        hasher.update(&[self.profile as u8, self.boundary as u8]);
        hasher.update(&(self.grid.nx as u64).to_le_bytes());
        hasher.update(&(self.grid.ny as u64).to_le_bytes());
        hasher.update(&(self.grid.nz as u64).to_le_bytes());
        hasher.update(&self.grid.dx.to_bits().to_le_bytes());
        hasher.update(&self.grid.dy.to_bits().to_le_bytes());
        hasher.update(&self.grid.dz.to_bits().to_le_bytes());
        hasher.update(&self.material.epsilon_r.to_bits().to_le_bytes());
        hasher.update(&self.material.mu_r.to_bits().to_le_bytes());
        hasher.update(&self.material.sigma_s_per_m.to_bits().to_le_bytes());
        hasher.update(&self.dt.to_bits().to_le_bytes());
        hasher.update(&self.step_index.to_le_bytes());
        for slice in [
            &self.ex, &self.ey, &self.ez, &self.hx, &self.hy, &self.hz, &self.jx, &self.jy,
            &self.jz,
        ] {
            for &v in slice.iter() {
                hasher.update(&v.to_bits().to_le_bytes());
            }
        }
        *hasher.finalize().as_bytes()
    }

    /// Seal `state_blake3` with the computed digest.
    pub fn seal(&mut self) {
        self.state_blake3 = self.compute_blake3();
    }

    /// Admit a checkpoint for deterministic solver continuation.
    ///
    /// Fails closed if boundary state is incomplete, auxiliary state is incomplete,
    /// `phi_unavailable_declared` is false in the no-phi profile, CFL is violated,
    /// or the BLAKE3 digest does not match.
    pub fn admit_for_restart(&self) -> Result<(), EmfOperatorError> {
        if !self.has_complete_boundary_state {
            return Err(EmfOperatorError::IncompleteRestartState {
                reason: "missing complete boundary state",
            });
        }
        if !self.has_complete_auxiliary_state {
            return Err(EmfOperatorError::IncompleteRestartState {
                reason: "missing complete auxiliary state",
            });
        }
        if !self.phi_unavailable_declared {
            return Err(EmfOperatorError::IncompleteRestartState {
                reason: "no-phi profile requires explicit unavailable scalar potential declaration",
            });
        }
        let n = self.grid.validate()?;
        self.material.validate(self.profile)?;
        admit_yee_timestep(&self.grid, &self.material, self.dt)?;
        for slice in [
            &self.ex, &self.ey, &self.ez, &self.hx, &self.hy, &self.hz, &self.jx, &self.jy,
            &self.jz,
        ] {
            if slice.len() != n {
                return Err(EmfOperatorError::SliceLengthMismatch {
                    expected: n,
                    provided: slice.len(),
                });
            }
            if slice.iter().any(|v| !v.is_finite()) {
                return Err(EmfOperatorError::NonFiniteInput);
            }
        }
        if self.compute_blake3() != self.state_blake3 {
            return Err(EmfOperatorError::RestartDigestMismatch);
        }
        Ok(())
    }

    /// Advance this checkpoint state by `steps` leapfrog iterations.
    pub fn advance_steps(&mut self, steps: usize) -> Result<(), EmfOperatorError> {
        self.admit_for_restart()?;
        let n = self.grid.validate()?;
        let mut cx = vec![0.0_f64; n];
        let mut cy = vec![0.0_f64; n];
        let mut cz = vec![0.0_f64; n];
        for _ in 0..steps {
            yee_step_leapfrog_in_place(
                &self.grid,
                self.profile,
                self.boundary,
                &self.material,
                self.dt,
                &mut self.ex,
                &mut self.ey,
                &mut self.ez,
                &mut self.hx,
                &mut self.hy,
                &mut self.hz,
                &self.jx,
                &self.jy,
                &self.jz,
                &mut cx,
                &mut cy,
                &mut cz,
            )?;
            self.step_index += 1;
        }
        self.seal();
        Ok(())
    }

    /// Bitwise equality check (`to_bits()`) across all six electromagnetic field arrays.
    pub fn is_bitwise_identical_to(&self, other: &Self) -> bool {
        if self.step_index != other.step_index || self.state_blake3 != other.state_blake3 {
            return false;
        }
        let eq_bits = |a: &[f64], b: &[f64]| -> bool {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.to_bits() == y.to_bits())
        };
        eq_bits(&self.ex, &other.ex)
            && eq_bits(&self.ey, &other.ey)
            && eq_bits(&self.ez, &other.ez)
            && eq_bits(&self.hx, &other.hx)
            && eq_bits(&self.hy, &other.hy)
            && eq_bits(&self.hz, &other.hz)
    }
}
