use crate::sim::LocalSimulator;
use core::f64::consts::FRAC_1_SQRT_2;
use qualia_core_db::NQuin;

use qualia_core_db::specialized_libs::category_theory::{Endomorphism, Morphism, Object};
use qualia_core_db::specialized_libs::shared::ZeroHeapMatrix;

/// Quantum State representation (The Object in our Category)
pub struct HilbertSpace<'a> {
    pub amplitudes: &'a mut [(f64, f64)],
}

impl<'a> Object for HilbertSpace<'a> {
    type Properties = usize; // number of qubits
    fn properties(&self) -> Self::Properties {
        self.amplitudes.len().trailing_zeros() as usize
    }
}

/// A specific gate application to a subset of the Hilbert space
pub struct GateApplication {
    pub target: usize,
    pub control: Option<usize>,
    pub matrix_2x2: Option<ZeroHeapMatrix<(f64, f64), 2, 2>>,
}

impl<'a> Morphism<HilbertSpace<'a>, HilbertSpace<'a>> for GateApplication {
    fn apply(&self, state: &mut HilbertSpace<'a>) -> Result<(), &'static str> {
        let bit = 1 << self.target;

        if let Some(control) = self.control {
            // CNOT logic
            let control_bit = 1 << control;
            for i in 0..state.amplitudes.len() {
                if (i & control_bit) != 0 && (i & bit) == 0 {
                    let pair_idx = i | bit;
                    state.amplitudes.swap(i, pair_idx);
                }
            }
        } else if let Some(matrix) = self.matrix_2x2.as_ref() {
            // General 1-qubit logic using the defined morphism matrix
            for i in 0..state.amplitudes.len() {
                if (i & bit) == 0 {
                    let pair_idx = i | bit;
                    let a = state.amplitudes[i];
                    let b = state.amplitudes[pair_idx];

                    // Multiply by matrix:
                    // [m00 m01] * [a]
                    // [m10 m11]   [b]
                    let m00 = matrix.data[0][0];
                    let m01 = matrix.data[0][1];
                    let m10 = matrix.data[1][0];
                    let m11 = matrix.data[1][1];

                    // Complex multiplication: (r1+i1)(r2+i2) = (r1r2 - i1i2) + i(r1i2 + r2i1)
                    let c_mul = |c1: (f64, f64), c2: (f64, f64)| -> (f64, f64) {
                        (c1.0 * c2.0 - c1.1 * c2.1, c1.0 * c2.1 + c1.1 * c2.0)
                    };
                    let c_add = |c1: (f64, f64), c2: (f64, f64)| -> (f64, f64) {
                        (c1.0 + c2.0, c1.1 + c2.1)
                    };

                    state.amplitudes[i] = c_add(c_mul(m00, a), c_mul(m01, b));
                    state.amplitudes[pair_idx] = c_add(c_mul(m10, a), c_mul(m11, b));
                }
            }
        }

        Ok(())
    }
}

/// A local zero-allocation state vector simulator acting via Endomorphisms.
pub struct StateVectorSimulator {
    num_qubits: u32,
}

impl StateVectorSimulator {
    /// Creates a new instance.
    pub fn new(num_qubits: u32) -> Self {
        Self { num_qubits }
    }
}

impl LocalSimulator for StateVectorSimulator {
    fn execute_circuit(
        &self,
        state_vector: &mut [(f64, f64)],
        instructions: &[NQuin],
    ) -> Result<(), &'static str> {
        let expected_len = 1 << self.num_qubits;
        if state_vector.len() < expected_len {
            return Err("State vector buffer too small");
        }

        for quin in instructions {
            // NQuin layout for quantum instructions:
            // predicate = q_hash("q42:quantum_op")
            // object contains opcode and operands
            // For now, we will assume a simple encoding in `object` for demonstration
            let opcode = (quin.object >> 56) & 0xFF;
            let operand1 = ((quin.object >> 40) & 0xFFFF) as usize;
            let operand2 = ((quin.object >> 24) & 0xFFFF) as usize;

            let mut space = HilbertSpace {
                amplitudes: state_vector,
            };
            let mut morphism = GateApplication {
                target: operand1,
                control: None,
                matrix_2x2: None,
            };

            match opcode {
                // OP_X
                0x01 => {
                    morphism.matrix_2x2 = Some(ZeroHeapMatrix::new([
                        [(0.0, 0.0), (1.0, 0.0)],
                        [(1.0, 0.0), (0.0, 0.0)],
                    ]));
                    morphism.apply(&mut space)?;
                }
                // OP_Z
                0x02 => {
                    morphism.matrix_2x2 = Some(ZeroHeapMatrix::new([
                        [(1.0, 0.0), (0.0, 0.0)],
                        [(0.0, 0.0), (-1.0, 0.0)],
                    ]));
                    morphism.apply(&mut space)?;
                }
                // OP_H
                0x03 => {
                    morphism.matrix_2x2 = Some(ZeroHeapMatrix::new([
                        [(FRAC_1_SQRT_2, 0.0), (FRAC_1_SQRT_2, 0.0)],
                        [(FRAC_1_SQRT_2, 0.0), (-FRAC_1_SQRT_2, 0.0)],
                    ]));
                    morphism.apply(&mut space)?;
                }
                // OP_CX
                0x04 => {
                    morphism.control = Some(operand1);
                    morphism.target = operand2;
                    morphism.apply(&mut space)?;
                }
                // OP_Y
                0x05 => {
                    morphism.matrix_2x2 = Some(ZeroHeapMatrix::new([
                        [(0.0, 0.0), (0.0, -1.0)],
                        [(0.0, 1.0), (0.0, 0.0)],
                    ]));
                    morphism.apply(&mut space)?;
                }
                // OP_S
                0x06 => {
                    morphism.matrix_2x2 = Some(ZeroHeapMatrix::new([
                        [(1.0, 0.0), (0.0, 0.0)],
                        [(0.0, 0.0), (0.0, 1.0)],
                    ]));
                    morphism.apply(&mut space)?;
                }
                // OP_T
                0x07 => {
                    morphism.matrix_2x2 = Some(ZeroHeapMatrix::new([
                        [(1.0, 0.0), (0.0, 0.0)],
                        [(0.0, 0.0), (FRAC_1_SQRT_2, FRAC_1_SQRT_2)],
                    ]));
                    morphism.apply(&mut space)?;
                }
                // OP_SWAP
                0x08 => {
                    let bit1 = 1 << operand1;
                    let bit2 = 1 << operand2;
                    for i in 0..space.amplitudes.len() {
                        let b1 = (i & bit1) != 0;
                        let b2 = (i & bit2) != 0;
                        if b1 && !b2 {
                            let pair_idx = (i & !bit1) | bit2;
                            space.amplitudes.swap(i, pair_idx);
                        }
                    }
                }
                _ => return Err("Unknown opcode"),
            }
        }

        Ok(())
    }

    fn sample(
        &self,
        state_vector: &[(f64, f64)],
        shots: u32,
        out_samples: &mut [u64],
    ) -> Result<usize, &'static str> {
        if out_samples.len() < shots as usize {
            return Err("Output buffer too small for shots");
        }
        if state_vector.is_empty() {
            return Err("Empty state vector");
        }

        // Streaming Born-rule sampling across the full state vector
        let mut total_norm = 0.0;
        for amp in state_vector.iter() {
            total_norm += amp.0 * amp.0 + amp.1 * amp.1;
        }
        if total_norm <= 0.0 {
            total_norm = 1.0;
        }

        let mut seed: u64 = 0x2545f4914f6cdd1d;
        for i in 0..shots as usize {
            // SplitMix64 pseudo-random generator
            seed = seed.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = seed;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^= z >> 31;
            let u = ((z >> 11) as f64) / ((1u64 << 53) as f64);
            let target = u * total_norm;

            let mut cumulative = 0.0;
            let mut outcome = 0u64;
            for (idx, amp) in state_vector.iter().enumerate() {
                let p = amp.0 * amp.0 + amp.1 * amp.1;
                cumulative += p;
                if cumulative >= target || idx == state_vector.len() - 1 {
                    outcome = idx as u64;
                    break;
                }
            }
            out_samples[i] = outcome;
        }

        Ok(shots as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bell_state_and_full_sampling() {
        let sim = StateVectorSimulator::new(2);
        let mut sv = [(0.0, 0.0); 4];
        sv[0] = (1.0, 0.0); // |00>

        // Bell circuit: H(0), CX(0, 1)
        let quin_h = NQuin {
            subject: 0,
            predicate: 0,
            object: (0x03u64 << 56) | (0u64 << 40),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let quin_cx = NQuin {
            subject: 0,
            predicate: 0,
            object: (0x04u64 << 56) | (0u64 << 40) | (1u64 << 24),
            context: 0,
            metadata: 0,
            parity: 0,
        };

        sim.execute_circuit(&mut sv, &[quin_h, quin_cx]).unwrap();
        // Bell state: (|00> + |11>) / sqrt(2)
        assert!((sv[0].0 - FRAC_1_SQRT_2).abs() < 1e-9);
        assert!((sv[1].0).abs() < 1e-9);
        assert!((sv[2].0).abs() < 1e-9);
        assert!((sv[3].0 - FRAC_1_SQRT_2).abs() < 1e-9);

        // Sample 100 shots: outcomes must only be 0 (|00>) or 3 (|11>)
        let mut samples = [0u64; 100];
        let count = sim.sample(&sv, 100, &mut samples).unwrap();
        assert_eq!(count, 100);
        for &s in samples.iter() {
            assert!(s == 0 || s == 3, "Outcome must be 0 or 3 in Bell state, got {s}");
        }
    }

    #[test]
    fn test_sampling_beyond_32_amplitudes() {
        let sim = StateVectorSimulator::new(6); // 64 basis states
        let mut sv = [(0.0, 0.0); 64];
        sv[42] = (1.0, 0.0); // |42>

        let mut samples = [0u64; 10];
        sim.sample(&sv, 10, &mut samples).unwrap();
        for &s in samples.iter() {
            assert_eq!(s, 42, "Basis state 42 must be sampled with certainty");
        }
    }
}
