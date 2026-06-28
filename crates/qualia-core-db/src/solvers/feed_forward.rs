//! Feed-forward network (SwiGLU) — the STEM definition of the transformer FFN block as the
//! composition it is:
//!
//! ```text
//! FFN(x) = W_down · ( SiLU(W_gate · x) ⊙ (W_up · x) )
//! ```
//!
//! That is three matrix–vector products ([`super::linear_algebra::gemm::matvec`]), one
//! activation ([`super::activation::silu`]), and one Hadamard product
//! ([`super::linear_algebra::vector::hadamard_assign`]). Nothing proprietary — the LLM "FFN
//! block" is exactly this. The runtime's `dispatch_ffn_block_pre_norm` is a backend computing
//! this same function over quantized weights on the GPU.
//!
//! Caller-owned, zero internal allocation (gate/up scratch buffers are supplied).

use crate::solvers::activation::silu;
use crate::solvers::linear_algebra::gemm::{matvec, Transpose};
use crate::solvers::linear_algebra::vector::hadamard_assign;
use crate::solvers::SolversError;

/// Compute `out = W_down · ( SiLU(W_gate · x) ⊙ (W_up · x) )`, row-major, caller-owned.
///
/// - `x`: input, length `d_model`
/// - `w_gate`, `w_up`: `d_ff × d_model` each
/// - `w_down`: `d_model × d_ff`
/// - `gate_buf`, `up_buf`: scratch, length `d_ff` each (overwritten)
/// - `out`: result, length `d_model` (overwritten)
///
/// Fails closed ([`SolversError::InvalidDimension`]) on any shape mismatch.
#[allow(clippy::too_many_arguments)]
pub fn swiglu_ffn(
    d_model: usize,
    d_ff: usize,
    x: &[f64],
    w_gate: &[f64],
    w_up: &[f64],
    w_down: &[f64],
    gate_buf: &mut [f64],
    up_buf: &mut [f64],
    out: &mut [f64],
) -> Result<(), SolversError> {
    if x.len() != d_model
        || w_gate.len() != d_ff * d_model
        || w_up.len() != d_ff * d_model
        || w_down.len() != d_model * d_ff
        || gate_buf.len() != d_ff
        || up_buf.len() != d_ff
        || out.len() != d_model
    {
        return Err(SolversError::InvalidDimension);
    }
    // gate = W_gate · x   ;   up = W_up · x
    matvec(Transpose::No, d_ff, d_model, w_gate, x, gate_buf)?;
    matvec(Transpose::No, d_ff, d_model, w_up, x, up_buf)?;
    // gate = SiLU(gate) ⊙ up
    silu(gate_buf);
    hadamard_assign(gate_buf, up_buf)?;
    // out = W_down · gate
    matvec(Transpose::No, d_model, d_ff, w_down, gate_buf, out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silu_scalar(z: f64) -> f64 {
        z / (1.0 + (-z).exp())
    }

    #[test]
    fn matches_hand_computed_swiglu() {
        // d_model = 2, d_ff = 2. Known weights; verify against an independent hand computation.
        let x = [1.0, 2.0];
        // W_gate (2×2): rows [1,0],[0,1]  ⇒ gate = [x0, x1] = [1, 2]
        let w_gate = [1.0, 0.0, 0.0, 1.0];
        // W_up (2×2): rows [1,1],[1,-1]   ⇒ up = [x0+x1, x0-x1] = [3, -1]
        let w_up = [1.0, 1.0, 1.0, -1.0];
        // W_down (2×2): rows [1,0],[0,1]  ⇒ out = h (identity)
        let w_down = [1.0, 0.0, 0.0, 1.0];

        let mut gate = [0.0; 2];
        let mut up = [0.0; 2];
        let mut out = [0.0; 2];
        swiglu_ffn(
            2, 2, &x, &w_gate, &w_up, &w_down, &mut gate, &mut up, &mut out,
        )
        .unwrap();

        // Expected: h = silu([1,2]) ⊙ [3,-1]; out = h.
        let h0 = silu_scalar(1.0) * 3.0;
        let h1 = silu_scalar(2.0) * -1.0;
        assert!((out[0] - h0).abs() < 1e-12, "out0 = {} != {}", out[0], h0);
        assert!((out[1] - h1).abs() < 1e-12, "out1 = {} != {}", out[1], h1);
    }

    #[test]
    fn zero_input_gives_zero_output() {
        // SiLU(0)=0 ⇒ gate=0 ⇒ Hadamard 0 ⇒ out 0, for any weights.
        let x = [0.0, 0.0, 0.0];
        let w_gate = [1.0; 9];
        let w_up = [2.0; 9];
        let w_down = [3.0; 9];
        let mut gate = [0.0; 3];
        let mut up = [0.0; 3];
        let mut out = [0.0; 3];
        swiglu_ffn(
            3, 3, &x, &w_gate, &w_up, &w_down, &mut gate, &mut up, &mut out,
        )
        .unwrap();
        for &v in &out {
            assert!(v.abs() < 1e-12);
        }
    }

    #[test]
    fn rejects_bad_dims() {
        let x = [1.0, 2.0];
        let w = [1.0, 0.0, 0.0, 1.0];
        let mut gate = [0.0; 2];
        let mut up = [0.0; 2];
        let mut out = [0.0; 1]; // wrong: should be d_model = 2
        assert!(matches!(
            swiglu_ffn(2, 2, &x, &w, &w, &w, &mut gate, &mut up, &mut out),
            Err(SolversError::InvalidDimension)
        ));
    }
}
