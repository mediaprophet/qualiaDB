//! Wave-physics substrate **logic** (§20, legal_logic.md) — the continuous→discrete bridge.
//!
//! SCOPE (honest): this is the CPU reference *logic* that bridges a continuous physical signal
//! to a discrete factual quin the epistemic layer can reason over — `∫Ψ > τ → Fact(p)`. The
//! full GPU-enumerated 10D-tensor manifold *renderer* (`compute_universe.rs`) is a separate,
//! larger effort (STELLAR tasks #11–13); this module deliberately does NOT implement that.
//!
//! Qualitative/quantitative realities (EMF, acoustic, visual evidence) are evaluated as
//! continuous math here, then thresholded into discrete facts — so e.g. a measured signal
//! exceeding a legal limit instantiates a factual quin that the deontic/epistemic engines use.

use core::f64::consts::PI;

/// The named physical coordinates of a wave sample Ψ(x,y,z,t,f,a,φ) — the axes of the manifold.
#[derive(Debug, Clone, Copy)]
pub struct WaveCoord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub t: f64,
    pub f: f64,
    pub a: f64,
    pub phi: f64,
}

/// Evaluate the wave field Ψ at a coordinate: amplitude `a` oscillating at frequency `f` with
/// phase `φ` at time `t`, attenuated by an inverse-square spatial envelope `1/(1+r²)`.
/// Deterministic CPU reference.
pub fn wave_eval(c: &WaveCoord) -> f64 {
    let r2 = c.x * c.x + c.y * c.y + c.z * c.z;
    let envelope = 1.0 / (1.0 + r2);
    c.a * (2.0 * PI * c.f * c.t + c.phi).sin() * envelope
}

/// Trapezoidal integral of |Ψ| over an ordered sample series — the accumulated signal energy.
pub fn integrate_abs(samples: &[f64]) -> f64 {
    if samples.len() < 2 {
        return samples.first().map(|v| v.abs()).unwrap_or(0.0);
    }
    let mut acc = 0.0;
    for w in samples.windows(2) {
        acc += (w[0].abs() + w[1].abs()) * 0.5;
    }
    acc
}

/// **Continuous → discrete**: if the integrated signal exceeds `threshold`, instantiate the
/// discrete fact `fact_id` (`∫Ψ > τ → Fact(p)`); otherwise `None`. The bridge from the manifold
/// substrate to `epistemic.rs`.
pub fn continuous_to_fact(samples: &[f64], threshold: f64, fact_id: u64) -> Option<u64> {
    if integrate_abs(samples) > threshold {
        Some(fact_id)
    } else {
        None
    }
}

// ─── Topological data analysis: Vietoris-Rips + persistent H0 ─────────────────────
//
// Detect topological features (connected components / clusters) in the continuous signal by
// building a Vietoris-Rips complex at a scale ε and tracking how components are born and die as ε
// grows (0-dimensional persistent homology). Bounded + zero-heap (fixed union-find arrays).

/// Bound on points in one topological query.
pub const MAX_MANIFOLD_POINTS: usize = 64;

#[inline]
fn uf_find(parent: &mut [usize; MAX_MANIFOLD_POINTS], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}
#[inline]
fn uf_union(parent: &mut [usize; MAX_MANIFOLD_POINTS], a: usize, b: usize) {
    let (ra, rb) = (uf_find(parent, a), uf_find(parent, b));
    if ra != rb {
        parent[ra] = rb;
    }
}

/// **Betti-0** (number of connected components) of the **Vietoris-Rips** complex at scale
/// `epsilon`: connect points `i,j` whenever `dist[i*n+j] <= epsilon` (`dist` is a flattened
/// `n×n` distance matrix). `0` for invalid input. Bounded + zero-heap.
pub fn vietoris_rips_b0(dist: &[f64], n: usize, epsilon: f64) -> usize {
    if n == 0 || n > MAX_MANIFOLD_POINTS || dist.len() < n * n {
        return 0;
    }
    let mut parent = [0usize; MAX_MANIFOLD_POINTS];
    for (i, p) in parent.iter_mut().enumerate().take(n) {
        *p = i;
    }
    for i in 0..n {
        for j in (i + 1)..n {
            if dist[i * n + j] <= epsilon {
                uf_union(&mut parent, i, j);
            }
        }
    }
    let mut components = 0usize;
    for i in 0..n {
        if uf_find(&mut parent, i) == i {
            components += 1;
        }
    }
    components
}

/// **Persistent 0-dim homology**: write the Betti-0 (component count) at each scale in `epsilons`
/// (assumed increasing) into `out_b0` — the barcode of connected features (born at ε=0, dying as
/// they merge; b0 is monotonically non-increasing). Returns the count written. Zero-heap.
pub fn persistent_h0(dist: &[f64], n: usize, epsilons: &[f64], out_b0: &mut [usize]) -> usize {
    let m = epsilons.len().min(out_b0.len());
    for k in 0..m {
        out_b0[k] = vietoris_rips_b0(dist, n, epsilons[k]);
    }
    m
}

/// **Topological dimension-bridging**: lift a lower-dimensional sample `low` (e.g. a 1-D audio
/// sample, or a 7-axis `WaveCoord`) into a higher `out`-dimensional manifold coordinate, zero-
/// padding the new axes. Returns `false` if `out` is too small. The 1D→10D bridge.
pub fn bridge_dimensions(low: &[f64], out: &mut [f64]) -> bool {
    if out.len() < low.len() {
        return false;
    }
    for (i, o) in out.iter_mut().enumerate() {
        *o = if i < low.len() { low[i] } else { 0.0 };
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    #[test]
    fn vietoris_rips_and_persistent_h0() {
        // 4 points on a line at 0,1,2,10 → pairwise |Δ| distance matrix.
        let pts = [0.0f64, 1.0, 2.0, 10.0];
        let n = 4;
        let mut dist = [0.0f64; 16];
        for i in 0..n {
            for j in 0..n {
                dist[i * n + j] = (pts[i] - pts[j]).abs();
            }
        }
        // ε=0.5: nothing connects → 4 components.
        assert_eq!(vietoris_rips_b0(&dist, n, 0.5), 4);
        // ε=1.0: the 0-1-2 cluster connects (dist 1 each); 10 stays apart → 2 components.
        assert_eq!(vietoris_rips_b0(&dist, n, 1.0), 2);
        // ε=10: everything connects → 1 component.
        assert_eq!(vietoris_rips_b0(&dist, n, 10.0), 1);
        // The persistence barcode across an increasing filtration.
        let mut b0 = [0usize; 3];
        let m = persistent_h0(&dist, n, &[0.5, 1.0, 10.0], &mut b0);
        assert_eq!(m, 3);
        assert_eq!(b0, [4, 2, 1], "b0 is monotonically non-increasing as ε grows");
    }

    #[test]
    fn dimension_bridging_zero_pads() {
        let low = [1.0f64, 2.0, 3.0]; // a 3-D sample
        let mut out = [9.0f64; 10]; // into the 10-D manifold
        assert!(bridge_dimensions(&low, &mut out));
        assert_eq!(&out[..3], &[1.0, 2.0, 3.0]);
        assert!(out[3..].iter().all(|&v| v == 0.0), "new axes zero-padded");
        // Too-small target refuses.
        assert!(!bridge_dimensions(&low, &mut [0.0; 2]));
    }

    #[test]
    fn wave_eval_at_origin_peak() {
        // origin (envelope=1), f=0, phi=π/2, t=0 → sin(π/2)=1 → Ψ = a.
        let c = WaveCoord { x: 0.0, y: 0.0, z: 0.0, t: 0.0, f: 0.0, a: 2.0, phi: PI / 2.0 };
        assert!((wave_eval(&c) - 2.0).abs() < 1e-9);
        // Off-origin attenuates: same wave at r²=3 (x=y=z=1) → 2 * 1/(1+3) = 0.5.
        let c2 = WaveCoord { x: 1.0, y: 1.0, z: 1.0, ..c };
        assert!((wave_eval(&c2) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn continuous_signal_crosses_into_a_fact() {
        let samples = [1.0, 1.0, 1.0]; // trapezoid: 1 + 1 = 2
        assert!((integrate_abs(&samples) - 2.0).abs() < 1e-9);
        let fact = q_hash("fact:emfLimitExceeded");
        // Over threshold → the fact is instantiated.
        assert_eq!(continuous_to_fact(&samples, 1.5, fact), Some(fact));
        // Under threshold → no fact.
        assert_eq!(continuous_to_fact(&samples, 5.0, fact), None);
    }
}
