//! Activation & normalization functions — the STEM definitions of the element-wise and
//! reduction operations a transformer forward pass is built from.
//!
//! These are not a proprietary "AI engine"; they are standard mathematics:
//! - **activations** (ReLU, sigmoid, tanh, SiLU/Swish, GELU) — element-wise nonlinear maps;
//! - **softmax** — the normalized exponential, a projection onto the probability simplex;
//! - **RMS / layer normalization** — statistical rescaling (variance / mean-and-variance).
//!
//! This module is the canonical, inspectable home for that math. The LLM runtime
//! (`gguf_bridge`) holds inline `f32` hot-path versions and promoted GPU kernels; those are
//! *backends* of these definitions and are checked against them (the same arrangement proved
//! for GEMM in `solvers::linear_algebra::gemm`). `gguf` itself is only a weight *file format* —
//! the mathematics lives here.
//!
//! All functions operate **in place on caller-owned `f64` slices** — zero allocation.

/// ReLU: `max(0, x)`, element-wise.
pub fn relu(x: &mut [f64]) {
    for v in x.iter_mut() {
        if *v < 0.0 {
            *v = 0.0;
        }
    }
}

/// Logistic sigmoid: `σ(x) = 1 / (1 + e^{-x})`, element-wise.
pub fn sigmoid(x: &mut [f64]) {
    for v in x.iter_mut() {
        *v = 1.0 / (1.0 + (-*v).exp());
    }
}

/// Hyperbolic tangent, element-wise.
pub fn tanh(x: &mut [f64]) {
    for v in x.iter_mut() {
        *v = v.tanh();
    }
}

/// SiLU / Swish: `x · σ(x) = x / (1 + e^{-x})`, element-wise (Llama/SmolLM2 gate activation).
pub fn silu(x: &mut [f64]) {
    for v in x.iter_mut() {
        *v = *v / (1.0 + (-*v).exp());
    }
}

/// GELU (Gaussian Error Linear Unit), tanh approximation:
/// `0.5·x·(1 + tanh(√(2/π)·(x + 0.044715·x³)))`. The standard GPT-2/transformer GELU.
pub fn gelu(x: &mut [f64]) {
    const C: f64 = 0.797_884_560_802_865_4; // sqrt(2/π)
    for v in x.iter_mut() {
        let x3 = *v * *v * *v;
        *v = 0.5 * *v * (1.0 + (C * (*v + 0.044_715 * x3)).tanh());
    }
}

/// Softmax in place: `softmax(x)_i = e^{x_i} / Σ_j e^{x_j}`, computed in the numerically
/// stable shifted form `e^{x_i − max} / Σ e^{x_j − max}`. After the call `x` sums to 1
/// (a probability distribution). A length-0 slice is left unchanged.
pub fn softmax(x: &mut [f64]) {
    if x.is_empty() {
        return;
    }
    let mut max = f64::NEG_INFINITY;
    for &v in x.iter() {
        if v > max {
            max = v;
        }
    }
    let mut sum = 0.0;
    for v in x.iter_mut() {
        *v = (*v - max).exp();
        sum += *v;
    }
    if sum > 0.0 {
        let inv = 1.0 / sum;
        for v in x.iter_mut() {
            *v *= inv;
        }
    }
}

/// RMS normalization in place: `x_i ← (x_i / sqrt(mean(x²) + eps)) · weight_i`.
/// No mean subtraction (the Llama/transformer RMSNorm). `weight` must match `x` in length;
/// shorter is honoured up to the common length.
pub fn rms_norm(x: &mut [f64], weight: &[f64], eps: f64) {
    let n = x.len().min(weight.len());
    if n == 0 {
        return;
    }
    let mut ss = 0.0;
    for i in 0..n {
        ss += x[i] * x[i];
    }
    let inv_rms = 1.0 / (ss / n as f64 + eps).sqrt();
    for i in 0..n {
        x[i] = x[i] * inv_rms * weight[i];
    }
}

/// Layer normalization in place: `x_i ← ((x_i − μ) / sqrt(σ² + eps)) · weight_i + bias_i`,
/// with `μ`, `σ²` the mean and (population) variance over `x`. `weight`/`bias` match `x`.
pub fn layer_norm(x: &mut [f64], weight: &[f64], bias: &[f64], eps: f64) {
    let n = x.len().min(weight.len()).min(bias.len());
    if n == 0 {
        return;
    }
    let mut mean = 0.0;
    for i in 0..n {
        mean += x[i];
    }
    mean /= n as f64;
    let mut var = 0.0;
    for i in 0..n {
        let d = x[i] - mean;
        var += d * d;
    }
    var /= n as f64;
    let inv_std = 1.0 / (var + eps).sqrt();
    for i in 0..n {
        x[i] = (x[i] - mean) * inv_std * weight[i] + bias[i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "{a} != {b} (tol {tol})");
    }

    #[test]
    fn relu_clamps_negatives() {
        let mut x = [-2.0, -0.1, 0.0, 0.5, 3.0];
        relu(&mut x);
        assert_eq!(x, [0.0, 0.0, 0.0, 0.5, 3.0]);
    }

    #[test]
    fn sigmoid_known_values() {
        let mut x = [0.0, 1.0, -1.0];
        sigmoid(&mut x);
        approx(x[0], 0.5, 1e-12);
        approx(x[1], 1.0 / (1.0 + (-1.0f64).exp()), 1e-12);
        approx(x[2], 1.0 / (1.0 + 1.0f64.exp()), 1e-12);
    }

    #[test]
    fn silu_equals_x_times_sigmoid() {
        let mut x = [1.5, -0.7, 2.0];
        let orig = x;
        silu(&mut x);
        for i in 0..3 {
            let s = 1.0 / (1.0 + (-orig[i]).exp());
            approx(x[i], orig[i] * s, 1e-12);
        }
        // SiLU(0) = 0.
        let mut z = [0.0];
        silu(&mut z);
        approx(z[0], 0.0, 1e-12);
    }

    #[test]
    fn gelu_zero_and_monotone() {
        let mut x = [0.0];
        gelu(&mut x);
        approx(x[0], 0.0, 1e-12); // GELU(0) = 0
        // Large positive ≈ identity, large negative ≈ 0.
        let mut big = [10.0, -10.0];
        gelu(&mut big);
        approx(big[0], 10.0, 1e-3);
        approx(big[1], 0.0, 1e-3);
    }

    #[test]
    fn softmax_sums_to_one_and_orders() {
        let mut x = [1.0, 2.0, 3.0];
        softmax(&mut x);
        approx(x.iter().sum::<f64>(), 1.0, 1e-12);
        assert!(x[2] > x[1] && x[1] > x[0]); // monotone in the inputs
        // Uniform inputs ⇒ uniform distribution.
        let mut u = [5.0, 5.0, 5.0, 5.0];
        softmax(&mut u);
        for &v in &u {
            approx(v, 0.25, 1e-12);
        }
    }

    #[test]
    fn softmax_is_shift_invariant_and_stable() {
        let mut a = [1.0, 2.0, 3.0];
        let mut b = [1.0 + 1000.0, 2.0 + 1000.0, 3.0 + 1000.0];
        softmax(&mut a);
        softmax(&mut b); // would overflow without the max-shift
        for i in 0..3 {
            approx(a[i], b[i], 1e-12);
        }
    }

    #[test]
    fn rms_norm_scales_to_unit_rms() {
        // With unit weights, the output RMS is 1 (for eps→0).
        let mut x = [3.0, -4.0, 0.0, 5.0];
        let w = [1.0, 1.0, 1.0, 1.0];
        rms_norm(&mut x, &w, 0.0);
        let ms = x.iter().map(|v| v * v).sum::<f64>() / 4.0;
        approx(ms.sqrt(), 1.0, 1e-12);
    }

    #[test]
    fn layer_norm_zero_mean_unit_var() {
        let mut x = [1.0, 2.0, 3.0, 4.0];
        let w = [1.0, 1.0, 1.0, 1.0];
        let b = [0.0, 0.0, 0.0, 0.0];
        layer_norm(&mut x, &w, &b, 0.0);
        let mean = x.iter().sum::<f64>() / 4.0;
        approx(mean, 0.0, 1e-12);
        let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / 4.0;
        approx(var, 1.0, 1e-12);
    }
}
