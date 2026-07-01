//! Zero-allocation local StateVector Simulator trait and utilities.
//!
//! Enforces that all execution happens within caller-supplied buffers
//! to guarantee zero heap allocations.

use qualia_core_db::NQuin;

pub mod statevector;
pub use statevector::*;

/// StateVector simulation backend trait.
pub trait LocalSimulator {
    /// Applies a circuit execution path onto the provided state vector buffer.
    ///
    /// # Arguments
    ///
    /// * `state_vector` - Fixed-size complex amplitude array (real, imaginary) interleaved.
    /// * `instructions` - Sequence of circuit opcodes/instructions.
    fn execute_circuit(
        &self,
        state_vector: &mut [(f64, f64)],
        instructions: &[NQuin],
    ) -> Result<(), &'static str>;

    /// Samples the state vector to produce measurement results in a caller-supplied buffer.
    fn sample(
        &self,
        state_vector: &[(f64, f64)],
        shots: u32,
        out_samples: &mut [u64],
    ) -> Result<usize, &'static str>;
}
