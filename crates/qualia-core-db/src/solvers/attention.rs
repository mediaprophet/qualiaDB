//! Scaled dot-product attention — the STEM definition of the transformer's attention
//! operation, as the composition it actually is:
//!
//! ```text
//! Attention(Q, K, V) = softmax( (Q·Kᵀ) · scale ) · V
//! ```
//!
//! There is nothing proprietary here: it is two matrix multiplies
//! ([`super::linear_algebra::gemm`]) with a row-wise normalized exponential
//! ([`super::activation::softmax`]) between them. This module is the inspectable home for
//! that math. The LLM runtime's `cpu_attention_pass` / GPU attention shaders are *backends*
//! that compute this same function (plus the integrated KV-cache, RoPE and projection
//! plumbing); `gguf` is only the weight file format.
//!
//! Caller-owned, zero internal allocation: the `n_q × n_k` score matrix and the `n_q × d_v`
//! output are caller-supplied buffers.

use crate::solvers::activation::softmax;
use crate::solvers::linear_algebra::gemm::{gemm, Transpose};
use crate::solvers::SolversError;

/// Compute `O = softmax((Q·Kᵀ)·scale) · V`, row-major, caller-owned.
///
/// - `q`: `n_q × d`  (queries)
/// - `k`: `n_k × d`  (keys)
/// - `v`: `n_k × d_v` (values)
/// - `scale`: the dot-product scaling (transformers use `1/√d`)
/// - `causal`: if `true`, query `i` may attend only to keys at position `≤ (n_k − n_q + i)`
///   (autoregressive masking; for full self-attention `n_q == n_k` this is `j ≤ i`)
/// - `scores`: scratch + attention weights, length `n_q * n_k` (overwritten)
/// - `out`: result, length `n_q * d_v` (overwritten)
///
/// On return, `scores` holds the row-stochastic attention weights and `out` the context.
/// Fails closed ([`SolversError::InvalidDimension`]) on any length/shape mismatch.
#[allow(clippy::too_many_arguments)]
pub fn scaled_dot_product_attention(
    n_q: usize,
    n_k: usize,
    d: usize,
    d_v: usize,
    q: &[f64],
    k: &[f64],
    v: &[f64],
    scale: f64,
    causal: bool,
    scores: &mut [f64],
    out: &mut [f64],
) -> Result<(), SolversError> {
    if q.len() != n_q * d
        || k.len() != n_k * d
        || v.len() != n_k * d_v
        || scores.len() != n_q * n_k
        || out.len() != n_q * d_v
    {
        return Err(SolversError::InvalidDimension);
    }
    if n_q > n_k {
        // Causal alignment assumes the queries are the last n_q positions of the n_k keys.
        return Err(SolversError::InvalidDimension);
    }

    // 1) scores = (Q · Kᵀ) · scale   — op(K)=Kᵀ since K is stored n_k×d.
    gemm(
        Transpose::No,
        Transpose::Yes,
        n_q,
        n_k,
        d,
        scale,
        q,
        k,
        0.0,
        scores,
    )?;

    // 2) optional causal mask, then row-wise softmax (each query's weights over the keys).
    for i in 0..n_q {
        let row = &mut scores[i * n_k..(i + 1) * n_k];
        if causal {
            let last_allowed = n_k - n_q + i; // position of query i within the key sequence
            for j in (last_allowed + 1)..n_k {
                row[j] = f64::NEG_INFINITY;
            }
        }
        softmax(row);
    }

    // 3) out = scores · V   — the value-weighted context.
    gemm(
        Transpose::No,
        Transpose::No,
        n_q,
        d_v,
        n_k,
        1.0,
        scores,
        v,
        0.0,
        out,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: &[f64], b: &[f64], tol: f64) {
        assert_eq!(a.len(), b.len());
        for i in 0..a.len() {
            assert!(
                (a[i] - b[i]).abs() < tol,
                "idx {i}: {} != {} (tol {tol})",
                a[i],
                b[i]
            );
        }
    }

    #[test]
    fn single_key_returns_that_value() {
        // One key/value ⇒ softmax over a single score = 1 ⇒ output = V.
        let q = [1.0, 2.0]; // 1×2
        let k = [0.5, 0.5]; // 1×2
        let v = [7.0, -3.0, 9.0]; // 1×3
        let mut scores = [0.0; 1];
        let mut out = [0.0; 3];
        scaled_dot_product_attention(1, 1, 2, 3, &q, &k, &v, 1.0, false, &mut scores, &mut out)
            .unwrap();
        approx(&scores, &[1.0], 1e-12);
        approx(&out, &v, 1e-12);
    }

    #[test]
    fn uniform_scores_average_the_values() {
        // Q=0 ⇒ all scores 0 ⇒ softmax uniform ⇒ output = mean of the value rows.
        let q = [0.0, 0.0]; // 1×2
        let k = [1.0, 0.0, 0.0, 1.0]; // 2×2
        let v = [2.0, 4.0, 6.0, 8.0]; // 2×2  (rows [2,4],[6,8])
        let mut scores = [0.0; 2];
        let mut out = [0.0; 2];
        scaled_dot_product_attention(1, 2, 2, 2, &q, &k, &v, 1.0, false, &mut scores, &mut out)
            .unwrap();
        approx(&scores, &[0.5, 0.5], 1e-12);
        approx(&out, &[4.0, 6.0], 1e-12); // ([2,4]+[6,8])/2
    }

    #[test]
    fn matches_hand_computed_softmax_qkt_v() {
        // Q (1×2), K (2×2), V (2×1); scale = 1. Verify against an independent hand computation.
        let q = [1.0, 0.0];
        let k = [1.0, 0.0, 0.0, 1.0]; // rows k0=[1,0], k1=[0,1]
        let v = [10.0, 20.0]; // v0=10, v1=20
        let scale = 1.0;
        let mut scores = [0.0; 2];
        let mut out = [0.0; 1];
        scaled_dot_product_attention(1, 2, 2, 1, &q, &k, &v, scale, false, &mut scores, &mut out)
            .unwrap();
        // s0 = q·k0 = 1, s1 = q·k1 = 0. softmax([1,0]) = [e/(e+1), 1/(e+1)].
        let e = std::f64::consts::E;
        let w0 = e / (e + 1.0);
        let w1 = 1.0 / (e + 1.0);
        approx(&scores, &[w0, w1], 1e-12);
        approx(&out, &[w0 * 10.0 + w1 * 20.0], 1e-12);
    }

    #[test]
    fn causal_mask_blocks_future_keys() {
        // n_q = n_k = 2, causal: query 0 sees only key 0; query 1 sees both.
        let q = [1.0, 1.0]; // 2×1 queries (each scalar)
        let k = [1.0, 1.0]; // 2×1 keys
        let v = [5.0, 9.0]; // 2×1 values
        let mut scores = [0.0; 4];
        let mut out = [0.0; 2];
        scaled_dot_product_attention(2, 2, 1, 1, &q, &k, &v, 1.0, true, &mut scores, &mut out)
            .unwrap();
        // Row 0 (query 0): only key 0 allowed ⇒ weight [1, 0] ⇒ out = v0 = 5.
        approx(&scores[0..2], &[1.0, 0.0], 1e-12);
        approx(&out[0..1], &[5.0], 1e-12);
        // Row 1 (query 1): both keys, equal scores ⇒ [0.5, 0.5] ⇒ out = (5+9)/2 = 7.
        approx(&scores[2..4], &[0.5, 0.5], 1e-12);
        approx(&out[1..2], &[7.0], 1e-12);
    }

    #[test]
    fn rejects_bad_dims() {
        let q = [1.0];
        let k = [1.0];
        let v = [1.0];
        let mut scores = [0.0; 2]; // wrong: should be 1
        let mut out = [0.0; 1];
        assert!(matches!(
            scaled_dot_product_attention(1, 1, 1, 1, &q, &k, &v, 1.0, false, &mut scores, &mut out),
            Err(SolversError::InvalidDimension)
        ));
    }
}
