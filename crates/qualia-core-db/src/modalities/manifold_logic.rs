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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

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
