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

        // Extremely rudimentary sampling: we need random numbers,
        // but since we are `#![no_std]` and pure, we expect a provided PRNG or simple hashing.
        // For demonstration, we simply deterministically pick based on probabilities.
        // In a real simulator, we'd pass in a PRNG state or similar.

        // This is a stub for deterministic output to fit zero-alloc requirements
        let mut sum = 0.0;
        let mut cdf = [0.0; 32]; // Small static buffer for small qubits only
        let limit = state_vector.len().min(32);

        for i in 0..limit {
            let p = state_vector[i].0 * state_vector[i].0 + state_vector[i].1 * state_vector[i].1;
            sum += p;
            cdf[i] = sum;
        }

        let mut seed: u64 = 0x1234567890abcdef;
        for i in 0..shots as usize {
            // Mock random number [0.0, 1.0) using simple xorshift
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let pseudo_rand = (seed % 1000) as f64 / 1000.0;

            let mut outcome = 0;
            for j in 0..limit {
                if pseudo_rand <= cdf[j] {
                    outcome = j as u64;
                    break;
                }
            }
            out_samples[i] = outcome;
        }

        Ok(shots as usize)
    }
}
