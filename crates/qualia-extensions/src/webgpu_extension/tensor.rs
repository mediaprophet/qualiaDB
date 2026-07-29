//! Dense matrix multiply `C[m×n] = A[m×k] · B[k×n]` (row-major).
//!
//! `A` and `B` come from `input_data["matrix_a"]` / `["matrix_b"]`, with
//! dimensions in `uniform_data` (`m`, `k`, `n`). This is the one direct (exact)
//! solve — the residual is `0` and the test asserts the exact product.

use super::{uniform, SolverReport, WebGpuJobParams};
use std::collections::HashMap;

pub fn solve(params: &WebGpuJobParams) -> SolverReport {
    let m = uniform(params, "m", 4.0).max(1.0) as usize;
    let k = uniform(params, "k", 4.0).max(1.0) as usize;
    let n = uniform(params, "n", 4.0).max(1.0) as usize;

    // Defaults (used when no input is supplied): A = identity(m×k bounded to a
    // square), B = a deterministic ramp, so C is well-defined and non-trivial.
    let sq = m.min(k).min(n);
    let a = params
        .input_data
        .get("matrix_a")
        .cloned()
        .unwrap_or_else(|| {
            let mut v = vec![0.0f32; m * k];
            for i in 0..sq {
                v[i * k + i] = 1.0;
            }
            v
        });
    let b = params
        .input_data
        .get("matrix_b")
        .cloned()
        .unwrap_or_else(|| (0..k * n).map(|i| i as f32).collect());

    // Guard against malformed inputs by clamping to the declared shape.
    let a_ok = a.len() >= m * k;
    let b_ok = b.len() >= k * n;

    let mut c = vec![0.0f32; m * n];
    if a_ok && b_ok {
        for i in 0..m {
            for p in 0..k {
                let aip = a[i * k + p];
                if aip == 0.0 {
                    continue;
                }
                let arow = &b[p * n..p * n + n];
                let crow = &mut c[i * n..i * n + n];
                for j in 0..n {
                    crow[j] += aip * arow[j];
                }
            }
        }
    }

    let mut output = HashMap::new();
    output.insert("result_out".to_string(), c);

    let flops = (2 * m * n * k).max(1) as u64;
    SolverReport {
        output,
        iterations_used: 1,
        final_residual: if a_ok && b_ok { 0.0 } else { 1.0 },
        converged: a_ok && b_ok,
        flops,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webgpu_extension::DispatchParams;

    fn gemm_params(a: Vec<f32>, b: Vec<f32>, m: u32, k: u32, n: u32) -> WebGpuJobParams {
        let mut input = HashMap::new();
        input.insert("matrix_a".to_string(), a);
        input.insert("matrix_b".to_string(), b);
        let mut uniforms = HashMap::new();
        uniforms.insert("m".to_string(), m as f32);
        uniforms.insert("k".to_string(), k as f32);
        uniforms.insert("n".to_string(), n as f32);
        WebGpuJobParams {
            shader_name: "tensor_gemm".to_string(),
            grid_size: (0, 0, 0),
            input_data: input,
            uniform_data: uniforms,
            dispatch_params: DispatchParams::default(),
        }
    }

    #[test]
    fn computes_exact_2x2_product() {
        // [[1,2],[3,4]] · [[5,6],[7,8]] = [[19,22],[43,50]].
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![5.0, 6.0, 7.0, 8.0];
        let r = solve(&gemm_params(a, b, 2, 2, 2));
        assert_eq!(r.output["result_out"], vec![19.0, 22.0, 43.0, 50.0]);
        assert!(r.converged);
        assert_eq!(r.final_residual, 0.0);
    }

    #[test]
    fn identity_is_a_left_unit() {
        let id = vec![1.0, 0.0, 0.0, 1.0];
        let b = vec![2.0, 3.0, 4.0, 5.0];
        let r = solve(&gemm_params(id, b.clone(), 2, 2, 2));
        assert_eq!(r.output["result_out"], b);
    }

    #[test]
    fn rectangular_shapes_multiply_correctly() {
        // A is 2×3, B is 3×2 ⇒ C is 2×2.
        // A = [[1,2,3],[4,5,6]], B = [[1,0],[0,1],[1,1]]
        // C = [[1+0+3, 0+2+3],[4+0+6, 0+5+6]] = [[4,5],[10,11]]
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = vec![1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let r = solve(&gemm_params(a, b, 2, 3, 2));
        assert_eq!(r.output["result_out"], vec![4.0, 5.0, 10.0, 11.0]);
    }
}
