//! Quantum context (q) for epistemic superposition and wavefunction collapse

use serde::{Deserialize, Serialize};

/// Quantum context state
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum QuantumState {
    /// Collapsed Ground Truth (q = 0)
    GroundTruth = 0,
    /// Parallel epistemic context (q > 0)
    ParallelContext = 1,
    /// Pending GSR resolution ("In Escrow")
    InEscrow = 2,
    /// Sandbox evaluation (e.g., isolated q=999)
    Sandbox = 3,
}

impl Default for QuantumState {
    fn default() -> Self {
        QuantumState::GroundTruth
    }
}

impl QuantumState {
    /// Builds `QuantumState` from `q_value`.
    pub fn from_q_value(q: f32) -> Self {
        if q == 0.0 {
            QuantumState::GroundTruth
        } else if q >= 999.0 {
            QuantumState::Sandbox
        } else {
            QuantumState::ParallelContext
        }
    }

    /// Converts to `q_value`.
    pub fn to_q_value(&self) -> f32 {
        match self {
            QuantumState::GroundTruth => 0.0,
            QuantumState::ParallelContext => 1.0,
            QuantumState::InEscrow => 2.0,
            QuantumState::Sandbox => 999.0,
        }
    }
}

/// Five-band quantum context range (§2.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantumContextRange {
    /// Empirical Ground Truth (q == 0)
    EmpiricalGroundTruth,
    /// Theoretical Hypothesis (q in 1..=998, branch ID)
    TheoreticalHypothesis(u16),
    /// Epistemic Escrow (q == 999, isolated quarantine / unverified LLM output)
    EpistemicEscrow,
    /// Fictional Construct (q in 1000..=65535, universe / world ID)
    FictionalConstruct(u16),
    /// Quantum Eigenstate (q >= 65536, packed (n, l, m) or Fock state ID)
    QuantumEigenstate(u32),
}

impl Default for QuantumContextRange {
    fn default() -> Self {
        Self::EmpiricalGroundTruth
    }
}

impl QuantumContextRange {
    /// Builds `QuantumContextRange` from continuous `q` value.
    pub fn from_q_value(q: f32) -> Self {
        if q <= 0.0 || q.abs() < 1e-6 {
            Self::EmpiricalGroundTruth
        } else if q < 998.5 {
            let branch = q.round().clamp(1.0, 998.0) as u16;
            Self::TheoreticalHypothesis(branch)
        } else if q < 999.5 {
            Self::EpistemicEscrow
        } else if q < 65535.5 {
            let world = q.round().clamp(1000.0, 65535.0) as u16;
            Self::FictionalConstruct(world)
        } else {
            Self::QuantumEigenstate(q.round().max(65536.0) as u32)
        }
    }

    /// Converts to canonical continuous `q` value.
    pub fn to_q_value(&self) -> f32 {
        match self {
            Self::EmpiricalGroundTruth => 0.0,
            Self::TheoreticalHypothesis(b) => *b as f32,
            Self::EpistemicEscrow => 999.0,
            Self::FictionalConstruct(u) => *u as f32,
            Self::QuantumEigenstate(id) => {
                if *id >= 65536 {
                    *id as f32
                } else {
                    (*id + 65536) as f32
                }
            }
        }
    }

    /// Pack hydrogenic quantum numbers `(n, l, m)` into `QuantumEigenstate`.
    ///
    /// Principal quantum number `n >= 1` (bits 16..23).
    /// Orbital angular momentum `0 <= l < n` (bits 8..15).
    /// Magnetic quantum number `-l <= m <= l` (bits 0..7 as two's-complement `i8`).
    /// With `n >= 1`, the packed value is naturally `>= 65536`.
    pub fn pack_hydrogenic_nlm(n: u8, l: u8, m: i8) -> Self {
        let n_eff = n.max(1) as u32;
        let l_eff = (l as u32) & 0xFF;
        let m_eff = (m as u8 as u32) & 0xFF;
        let packed = (n_eff << 16) | (l_eff << 8) | m_eff;
        Self::QuantumEigenstate(packed)
    }

    /// Unpack hydrogenic quantum numbers `(n, l, m)` if this is a valid hydrogenic eigenstate.
    pub fn unpack_hydrogenic_nlm(&self) -> Option<(u8, u8, i8)> {
        match self {
            Self::QuantumEigenstate(packed) => {
                let n = ((packed >> 16) & 0xFF) as u8;
                let l = ((packed >> 8) & 0xFF) as u8;
                let m = (packed & 0xFF) as u8 as i8;
                if n >= 1 && l < n && m >= -(l as i8) && m <= (l as i8) {
                    Some((n, l, m))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_five_bands() {
        assert_eq!(
            QuantumContextRange::from_q_value(0.0),
            QuantumContextRange::EmpiricalGroundTruth
        );
        assert_eq!(
            QuantumContextRange::from_q_value(1.0),
            QuantumContextRange::TheoreticalHypothesis(1)
        );
        assert_eq!(
            QuantumContextRange::from_q_value(500.0),
            QuantumContextRange::TheoreticalHypothesis(500)
        );
        assert_eq!(
            QuantumContextRange::from_q_value(998.0),
            QuantumContextRange::TheoreticalHypothesis(998)
        );
        assert_eq!(
            QuantumContextRange::from_q_value(999.0),
            QuantumContextRange::EpistemicEscrow
        );
        assert_eq!(
            QuantumContextRange::from_q_value(1000.0),
            QuantumContextRange::FictionalConstruct(1000)
        );
        assert_eq!(
            QuantumContextRange::from_q_value(65535.0),
            QuantumContextRange::FictionalConstruct(65535)
        );
        assert_eq!(
            QuantumContextRange::from_q_value(65536.0),
            QuantumContextRange::QuantumEigenstate(65536)
        );
    }

    #[test]
    fn test_to_q_value_round_trips() {
        let cases = [
            QuantumContextRange::EmpiricalGroundTruth,
            QuantumContextRange::TheoreticalHypothesis(12),
            QuantumContextRange::EpistemicEscrow,
            QuantumContextRange::FictionalConstruct(4200),
            QuantumContextRange::QuantumEigenstate(70000),
        ];
        for case in cases {
            let q = case.to_q_value();
            let recovered = QuantumContextRange::from_q_value(q);
            assert_eq!(case, recovered);
        }
    }

    #[test]
    fn test_hydrogenic_nlm_packing_round_trips() {
        let test_orbitals = [
            (1u8, 0u8, 0i8),    // 1s
            (2u8, 0u8, 0i8),    // 2s
            (2u8, 1u8, -1i8),   // 2p_-1
            (2u8, 1u8, 0i8),    // 2p_0
            (2u8, 1u8, 1i8),    // 2p_+1
            (3u8, 2u8, -2i8),   // 3d_-2
            (3u8, 2u8, 2i8),    // 3d_+2
            (4u8, 3u8, -3i8),   // 4f_-3
        ];
        for (n, l, m) in test_orbitals {
            let state = QuantumContextRange::pack_hydrogenic_nlm(n, l, m);
            let q = state.to_q_value();
            assert!(q >= 65536.0, "Eigenstate q must be >= 65536");
            let recovered_range = QuantumContextRange::from_q_value(q);
            let unpacked = recovered_range.unpack_hydrogenic_nlm();
            assert_eq!(unpacked, Some((n, l, m)));
        }
    }

    #[test]
    fn test_invalid_nlm_unpacks_none() {
        assert_eq!(
            QuantumContextRange::EmpiricalGroundTruth.unpack_hydrogenic_nlm(),
            None
        );
        // l >= n is invalid
        let invalid = QuantumContextRange::QuantumEigenstate((1 << 16) | (1 << 8));
        assert_eq!(invalid.unpack_hydrogenic_nlm(), None);
    }
}
