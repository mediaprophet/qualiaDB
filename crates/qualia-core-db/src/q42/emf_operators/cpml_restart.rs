//! 3D Roden-Gedney CPML + Multi-Pole Debye/Drude/Lorentz ADE Coupled Restart & BLAKE3 Sealing.
//!
//! Re-exports the continuum 3D CPML stretched-coordinate and multi-pole ADE coupled stepper
//! and `Schema 103` key `26`/`29` restart serializer from `etalon_sim::maxwell_emf::cpml_restart`
//! (`C01` ownership), and adds the `qualia-core-db` BLAKE3 (`QUALIA-YEE-CHECKPOINT-v1\0`)
//! cryptographic state commitment (`SealedCpmlAdeCheckpoint`).

pub use etalon_sim::maxwell_emf::cpml_restart::*;

use super::EmfOperatorError;

/// BLAKE3-sealed `Schema 103` key `26` (`ade_auxiliary_state`) + key `29` (`pml_auxiliary_state`)
/// checkpoint envelope for deterministic 3D dispersive CPML FDTD continuation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedCpmlAdeCheckpoint {
    pub restart_bytes: Vec<u8>,
    pub state_blake3: [u8; 32],
}

impl SealedCpmlAdeCheckpoint {
    /// Compute the canonical `QUALIA-YEE-CHECKPOINT-v1\0` BLAKE3-256 digest over `restart_bytes`.
    pub fn compute_blake3_for_bytes(restart_bytes: &[u8]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"QUALIA-YEE-CHECKPOINT-v1\0");
        hasher.update(&SCHEMA_103_KEY_ADE_AUXILIARY_STATE.to_le_bytes());
        hasher.update(&SCHEMA_103_KEY_PML_AUXILIARY_STATE.to_le_bytes());
        hasher.update(restart_bytes);
        *hasher.finalize().as_bytes()
    }

    /// Serialize and BLAKE3-seal a `CpmlAdeState` checkpoint.
    pub fn seal_from_state(state: &CpmlAdeState) -> Result<Self, EmfOperatorError> {
        let mut restart_bytes = Vec::new();
        serialize_cpml_ade_restart_into(state, &mut restart_bytes)?;
        let state_blake3 = Self::compute_blake3_for_bytes(&restart_bytes);
        Ok(Self {
            restart_bytes,
            state_blake3,
        })
    }

    /// Verify the BLAKE3 commitment and restore the `CpmlAdeState` for deterministic continuation.
    pub fn verify_and_restore(&self) -> Result<CpmlAdeState, EmfOperatorError> {
        let expected = Self::compute_blake3_for_bytes(&self.restart_bytes);
        if expected != self.state_blake3 {
            return Err(EmfOperatorError::RestartDigestMismatch);
        }
        restore_cpml_ade_restart_from(&self.restart_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42::emf_operators::{
        BoundaryCondition, DebyePole, DrudePole, Grid3dSpec, LorentzPole, YeeMaterialParams,
    };

    #[test]
    fn blake3_sealed_cpml_ade_checkpoint_roundtrip_and_tamper_rejection() {
        let grid = Grid3dSpec {
            nx: 6,
            ny: 6,
            nz: 6,
            dx: 1e-3,
            dy: 1e-3,
            dz: 1e-3,
        };
        let mat = YeeMaterialParams {
            epsilon_r: 2.0,
            mu_r: 1.0,
            sigma_s_per_m: 0.005,
        };
        let cpml = CpmlGradingSpec {
            thickness_cells: 2,
            m_order: 3.0,
            sigma_max_s_per_m: 1.0,
            kappa_max: 4.0,
            alpha_max_s_per_m: 0.04,
        };
        let mut st = CpmlAdeState::new(
            grid,
            mat,
            BoundaryCondition::DirichletPec,
            cpml,
            1e-12,
            vec![DebyePole {
                delta_epsilon: 0.4,
                tau_s: 4e-12,
            }],
            vec![DrudePole {
                omega_p_rad_s: 1.5e10,
                gamma_rad_s: 1e9,
            }],
            vec![LorentzPole {
                delta_epsilon: 0.15,
                omega_0_rad_s: 4e10,
                gamma_rad_s: 1.5e9,
            }],
        )
        .unwrap();
        st.e[2][grid.idx(3, 3, 3)] = 1.0;
        st.step_in_place().unwrap();

        let mut sealed = SealedCpmlAdeCheckpoint::seal_from_state(&st).unwrap();
        let restored = sealed.verify_and_restore().unwrap();
        assert_eq!(restored, st);

        sealed.state_blake3[0] ^= 0xFF;
        assert_eq!(
            sealed.verify_and_restore(),
            Err(EmfOperatorError::RestartDigestMismatch)
        );
    }
}
