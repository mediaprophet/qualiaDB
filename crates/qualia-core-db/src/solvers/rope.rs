//! Rotary Position Embedding (RoPE) — the STEM definition: a 2-D rotation.
//!
//! RoPE encodes token position by **rotating** each adjacent dimension pair `(2i, 2i+1)` of a
//! head by a position- and frequency-dependent angle
//!
//! ```text
//! θ_i = (pos / scale) · base^(−2i / head_dim)
//! (x0, x1) ↦ (x0·cosθ − x1·sinθ,  x0·sinθ + x1·cosθ)
//! ```
//!
//! That is exactly a rotation in the `(2i, 2i+1)` plane — the same rotation a geometric-algebra
//! rotor (`super::geometric_algebra`) performs — applied per head. It is orthogonal, so it
//! preserves the norm of every pair. Nothing proprietary: it is trigonometry. The LLM runtime's
//! `rope_inplace` is an `f32` backend computing this same function.
//!
//! In place on a caller-owned slice; zero allocation.

/// Apply interleaved ("normal"/llama) RoPE to `vec`, treated as `n_heads` consecutive blocks of
/// `head_dim` elements. Each block's adjacent pairs `(2i, 2i+1)` are rotated by `θ_i`. `pos` is
/// the token position, `base` the RoPE frequency base (e.g. 10000), `scale` the position scaling
/// (≤ 0 or non-finite is treated as 1).
pub fn rope_interleaved(
    vec: &mut [f64],
    n_heads: usize,
    head_dim: usize,
    pos: f64,
    base: f64,
    scale: f64,
) {
    let half = head_dim / 2;
    if half == 0 {
        return;
    }
    let scale = if scale > 0.0 && scale.is_finite() { scale } else { 1.0 };
    let scaled_pos = pos / scale;
    for head in 0..n_heads {
        let off = head * head_dim;
        if off + head_dim > vec.len() {
            return;
        }
        for i in 0..half {
            let theta = scaled_pos * base.powf(-2.0 * i as f64 / head_dim as f64);
            let (s, c) = theta.sin_cos();
            let x0 = vec[off + 2 * i];
            let x1 = vec[off + 2 * i + 1];
            vec[off + 2 * i] = x0 * c - x1 * s;
            vec[off + 2 * i + 1] = x0 * s + x1 * c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_zero_is_identity() {
        // θ = 0 ⇒ no rotation.
        let orig = [1.0, 2.0, 3.0, 4.0];
        let mut v = orig;
        rope_interleaved(&mut v, 1, 4, 0.0, 10000.0, 1.0);
        for i in 0..4 {
            assert!((v[i] - orig[i]).abs() < 1e-12);
        }
    }

    #[test]
    fn rotation_preserves_pair_norm() {
        // Rotations are orthogonal: each (2i, 2i+1) pair keeps its magnitude.
        let orig = [0.3, -0.7, 1.1, 2.0, -1.5, 0.4];
        let mut v = orig;
        rope_interleaved(&mut v, 1, 6, 5.0, 10000.0, 1.0);
        for i in 0..3 {
            let n0 = orig[2 * i] * orig[2 * i] + orig[2 * i + 1] * orig[2 * i + 1];
            let n1 = v[2 * i] * v[2 * i] + v[2 * i + 1] * v[2 * i + 1];
            assert!((n0 - n1).abs() < 1e-9, "pair {i} norm changed: {n0} -> {n1}");
        }
    }

    #[test]
    fn quarter_turn_known_rotation() {
        // For i = 0, θ_0 = scaled_pos. Choose scaled_pos = π/2 ⇒ (x0, x1) -> (-x1, x0).
        let mut v = [2.0, 5.0];
        rope_interleaved(&mut v, 1, 2, std::f64::consts::FRAC_PI_2, 10000.0, 1.0);
        assert!((v[0] - (-5.0)).abs() < 1e-9, "x0 = {}", v[0]);
        assert!((v[1] - 2.0).abs() < 1e-9, "x1 = {}", v[1]);
    }

    #[test]
    fn per_head_blocks_independent() {
        // Two heads of dim 2; head 0 and head 1 each rotate their own pair by the same angle.
        let mut v = [1.0, 0.0, 0.0, 1.0];
        rope_interleaved(&mut v, 2, 2, std::f64::consts::FRAC_PI_2, 10000.0, 1.0);
        // head 0: (1,0) -> (0,1);  head 1: (0,1) -> (-1,0).
        assert!((v[0] - 0.0).abs() < 1e-9 && (v[1] - 1.0).abs() < 1e-9);
        assert!((v[2] - (-1.0)).abs() < 1e-9 && (v[3] - 0.0).abs() < 1e-9);
    }
}
