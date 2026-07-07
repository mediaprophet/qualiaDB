//! P8.3 — Statistical manifold: probability-simplex ops + Fisher metric +
//! KL as a Bregman divergence.
//!
//! ## The probability simplex
//!
//! A point on the (n-1)-simplex Δⁿ is a probability distribution p = (p₁,…,pₙ)
//! with pᵢ ≥ 0 and Σpᵢ = 1.
//!
//! ## Fisher information metric
//!
//! The Fisher metric on Δⁿ is the unique invariant Riemannian metric:
//! ```text
//! g_ij(p) = δ_ij / p_i
//! ```
//! This is the Fisher information matrix for the multinomial family.
//!
//! ## KL divergence as a Bregman divergence
//!
//! KL(p‖q) = Σ pᵢ log(pᵢ/qᵢ) is the Bregman divergence associated with
//! the negative entropy convex function:
//! ```text
//! ψ(p) = Σ pᵢ log(pᵢ)
//! ```
//! The Bregman divergence is:
//! ```text
//! D_ψ(p, q) = ψ(p) - ψ(q) - ⟨∇ψ(q), p - q⟩
//! ```
//! which expands to KL(p‖q).
//!
//! ## Bregman-Pythagorean identity
//!
//! For the KL divergence, the projection Π_S(p) of p onto a closed convex
//! set S satisfies:
//! ```text
//! KL(p‖q) = KL(p‖Π_S(p)) + KL(Π_S(p)‖q)
//! ```
//! for any q ∈ S.
//!
//! ## Determinism
//!
//! All operations are deterministic on canonical byte inputs.

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Statistical manifold error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatManifoldError {
    /// Negative probability mass.
    NegativeMass { index: usize, value_bits: u32 },
    /// Probabilities don't sum to 1 (outside tolerance).
    NotNormalised { sum_bits: u32 },
    /// Zero support (all probabilities zero).
    ZeroSupport,
    /// Dimension mismatch.
    DimMismatch { expected: usize, got: usize },
}

impl core::fmt::Display for StatManifoldError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NegativeMass { index, value_bits } => {
                let v = f32::from_bits(*value_bits);
                write!(f, "stat_manifold: negative mass at index {index}: {v}")
            }
            Self::NotNormalised { sum_bits } => {
                let s = f32::from_bits(*sum_bits);
                write!(f, "stat_manifold: not normalised, sum = {s}")
            }
            Self::ZeroSupport => write!(f, "stat_manifold: zero support"),
            Self::DimMismatch { expected, got } => {
                write!(
                    f,
                    "stat_manifold: dim mismatch, expected {expected}, got {got}"
                )
            }
        }
    }
}

impl std::error::Error for StatManifoldError {}

// ───────────────────────────────────────────────────────────────────────────
//  Probability simplex operations
// ───────────────────────────────────────────────────────────────────────────

/// Normalisation tolerance.
const NORM_TOL: f32 = 1e-6;

/// Validate that a vector is a probability distribution (non-negative,
/// sums to 1).
pub fn validate_probability(p: &[f32]) -> Result<(), StatManifoldError> {
    if p.is_empty() {
        return Err(StatManifoldError::ZeroSupport);
    }
    let mut sum = 0.0f32;
    for (i, &pi) in p.iter().enumerate() {
        if pi < 0.0 {
            return Err(StatManifoldError::NegativeMass {
                index: i,
                value_bits: pi.to_bits(),
            });
        }
        sum += pi;
    }
    if sum == 0.0 {
        return Err(StatManifoldError::ZeroSupport);
    }
    if (sum - 1.0).abs() > NORM_TOL {
        return Err(StatManifoldError::NotNormalised {
            sum_bits: sum.to_bits(),
        });
    }
    Ok(())
}

/// Project a vector onto the probability simplex (normalise to sum = 1).
///
/// This is the Euclidean projection: clip negatives to 0, then normalise.
/// For the information-theoretic projection (KL), use `kl_projection`.
///
/// The Euclidean projection is idempotent: projecting an already-valid
/// distribution returns it unchanged.
pub fn simplex_project(p: &[f32], out: &mut [f32]) -> Result<usize, StatManifoldError> {
    if p.len() != out.len() {
        return Err(StatManifoldError::DimMismatch {
            expected: p.len(),
            got: out.len(),
        });
    }
    let n = p.len();
    if n == 0 {
        return Err(StatManifoldError::ZeroSupport);
    }

    // Clip negatives, sum, normalise.
    let mut sum = 0.0f32;
    for i in 0..n {
        out[i] = p[i].max(0.0);
        sum += out[i];
    }
    if sum == 0.0 {
        return Err(StatManifoldError::ZeroSupport);
    }
    for i in 0..n {
        out[i] /= sum;
    }
    Ok(n)
}

/// Check if the projection is idempotent: project(project(p)) == project(p).
pub fn simplex_project_idempotent(p: &[f32]) -> bool {
    let n = p.len();
    let mut proj1 = vec![0.0f32; n];
    let mut proj2 = vec![0.0f32; n];
    if simplex_project(p, &mut proj1).is_err() {
        return false;
    }
    if simplex_project(&proj1, &mut proj2).is_err() {
        return false;
    }
    for i in 0..n {
        if (proj1[i] - proj2[i]).abs() > 1e-7 {
            return false;
        }
    }
    true
}

// ───────────────────────────────────────────────────────────────────────────
//  Fisher information metric
// ───────────────────────────────────────────────────────────────────────────

/// Fisher information metric: `g_ij(p) = δ_ij / p_i`.
///
/// Computes the Fisher inner product of two tangent vectors u, v at point p:
/// ```text
/// ⟨u, v⟩_p = Σ u_i * v_i / p_i
/// ```
pub fn fisher_inner_product(p: &[f32], u: &[f32], v: &[f32]) -> Result<f64, StatManifoldError> {
    if p.len() != u.len() || p.len() != v.len() {
        return Err(StatManifoldError::DimMismatch {
            expected: p.len(),
            got: u.len(),
        });
    }
    let mut sum = 0.0f64;
    for i in 0..p.len() {
        if p[i] <= 0.0 {
            // Zero-support entry — tangent vector must be zero here.
            continue;
        }
        sum += (u[i] as f64) * (v[i] as f64) / (p[i] as f64);
    }
    Ok(sum)
}

/// Fisher distance between two distributions p, q:
/// `d_F(p, q) = arccos(⟨√p, √q⟩)` where √p = (√p₁,…,√pₙ).
///
/// This is the geodesic distance on the probability simplex under the
/// Fisher metric (the simplex is isometric to the positive orthant of
/// the unit sphere under the √ map).
pub fn fisher_distance(p: &[f32], q: &[f32]) -> Result<f64, StatManifoldError> {
    if p.len() != q.len() {
        return Err(StatManifoldError::DimMismatch {
            expected: p.len(),
            got: q.len(),
        });
    }
    let mut dot = 0.0f64;
    for i in 0..p.len() {
        let sp = (p[i] as f64).max(0.0).sqrt();
        let sq = (q[i] as f64).max(0.0).sqrt();
        dot += sp * sq;
    }
    // Clamp to [-1, 1] for numerical stability.
    let cos_theta = dot.clamp(-1.0, 1.0);
    Ok(cos_theta.acos())
}

// ───────────────────────────────────────────────────────────────────────────
//  KL divergence (Bregman divergence of negative entropy)
// ───────────────────────────────────────────────────────────────────────────

/// Negative entropy: `ψ(p) = Σ pᵢ log(pᵢ)`.
///
/// This is the convex generator for the KL Bregman divergence.
/// Note: `ψ(p) = Σ pᵢ log(pᵢ)` is convex (it's the negative of Shannon
/// entropy, up to sign: actually `Σ pᵢ log(pᵢ) = -H(p)` where H is entropy,
/// and -H is convex since H is concave).
pub fn neg_entropy(p: &[f32]) -> f64 {
    let mut sum = 0.0f64;
    for &pi in p {
        if pi > 0.0 {
            sum += (pi as f64) * (pi as f64).ln();
        }
    }
    sum
}

/// Gradient of negative entropy: `∇ψ(p)_i = log(pᵢ) + 1`.
pub fn neg_entropy_grad(p: &[f32], out: &mut [f64]) {
    for (i, &pi) in p.iter().enumerate() {
        out[i] = if pi > 0.0 {
            (pi as f64).ln() + 1.0
        } else {
            f64::NEG_INFINITY
        };
    }
}

/// KL divergence: `KL(p‖q) = Σ pᵢ log(pᵢ/qᵢ)`.
///
/// This is the Bregman divergence of the negative entropy:
/// `D_ψ(p, q) = ψ(p) - ψ(q) - ⟨∇ψ(q), p - q⟩`
pub fn kl_divergence(p: &[f32], q: &[f32]) -> Result<f64, StatManifoldError> {
    if p.len() != q.len() {
        return Err(StatManifoldError::DimMismatch {
            expected: p.len(),
            got: q.len(),
        });
    }
    let mut sum = 0.0f64;
    for i in 0..p.len() {
        if p[i] > 0.0 {
            if q[i] <= 0.0 {
                // KL is infinite when p_i > 0 and q_i = 0.
                return Ok(f64::INFINITY);
            }
            sum += (p[i] as f64) * ((p[i] as f64).ln() - (q[i] as f64).ln());
        }
    }
    Ok(sum)
}

/// KL divergence computed via the Bregman divergence formula:
/// `D_ψ(p, q) = ψ(p) - ψ(q) - ⟨∇ψ(q), p - q⟩`
///
/// This should match `kl_divergence` to within floating-point tolerance.
pub fn kl_bregman_form(p: &[f32], q: &[f32]) -> Result<f64, StatManifoldError> {
    if p.len() != q.len() {
        return Err(StatManifoldError::DimMismatch {
            expected: p.len(),
            got: q.len(),
        });
    }
    let psi_p = neg_entropy(p);
    let psi_q = neg_entropy(q);

    let mut grad_q = vec![0.0f64; q.len()];
    neg_entropy_grad(q, &mut grad_q);

    // ⟨∇ψ(q), p - q⟩ = Σ (log(q_i) + 1) * (p_i - q_i)
    let mut inner = 0.0f64;
    for i in 0..p.len() {
        if q[i] > 0.0 {
            inner += grad_q[i] * ((p[i] as f64) - (q[i] as f64));
        }
    }

    Ok(psi_p - psi_q - inner)
}

// ───────────────────────────────────────────────────────────────────────────
//  Bregman-Pythagorean identity
// ───────────────────────────────────────────────────────────────────────────

/// Verify the Bregman-Pythagorean identity for KL divergence:
///
/// Given a point p, its KL-projection q* onto a linear family S = {q : Σqᵢ = 1}
/// (which is just the normalisation of p), and any q ∈ S:
///
/// `KL(p‖q) = KL(p‖q*) + KL(q*‖q)`
///
/// For the specific case where S is the set of uniform distributions,
/// q* = (1/n,…,1/n) and the identity becomes:
/// `KL(p‖u) = KL(p‖q*) + KL(q*‖u)`
///
/// This function constructs a test case and returns the three KL values.
pub fn bregman_pythagorean_test(
    p: &[f32],
    q_star: &[f32],
    q: &[f32],
) -> Result<(f64, f64, f64), StatManifoldError> {
    let kl_p_q = kl_divergence(p, q)?;
    let kl_p_qstar = kl_divergence(p, q_star)?;
    let kl_qstar_q = kl_divergence(q_star, q)?;
    Ok((kl_p_q, kl_p_qstar, kl_qstar_q))
}

// ───────────────────────────────────────────────────────────────────────────
//  Determinism hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over a probability vector for determinism verification.
pub fn probability_hash(p: &[f32]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &pi in p {
        hash ^= pi.to_bits() as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kl_self_is_zero() {
        let p = [0.3f32, 0.3, 0.4];
        let kl = kl_divergence(&p, &p).unwrap();
        assert!(kl.abs() < 1e-12, "KL(p||p) must be 0, got {}", kl);
    }

    #[test]
    fn kl_matches_bregman_form() {
        let p = [0.2f32, 0.5, 0.3];
        let q = [0.4f32, 0.4, 0.2];
        let kl_direct = kl_divergence(&p, &q).unwrap();
        let kl_bregman = kl_bregman_form(&p, &q).unwrap();
        assert!(
            (kl_direct - kl_bregman).abs() < 1e-10,
            "KL direct {} must match Bregman form {}",
            kl_direct,
            kl_bregman
        );
    }

    #[test]
    fn kl_matches_bregman_form_uniform() {
        let p = [0.25f32, 0.25, 0.25, 0.25];
        let q = [0.1f32, 0.2, 0.3, 0.4];
        let kl_direct = kl_divergence(&p, &q).unwrap();
        let kl_bregman = kl_bregman_form(&p, &q).unwrap();
        assert!(
            (kl_direct - kl_bregman).abs() < 1e-6,
            "KL direct {} must match Bregman form {}",
            kl_direct,
            kl_bregman
        );
    }

    #[test]
    fn fisher_metric_delta1_closed_form() {
        // On Δ¹ (Bernoulli: p = (t, 1-t)), the Fisher metric is:
        // g(t) = 1/t + 1/(1-t) = 1/(t(1-t))
        // The geodesic distance is:
        // d = 2 * arccos(√(p·q)) where p·q = √(t_p*t_q) + √((1-t_p)*(1-t_q))
        let p = [0.5f32, 0.5];
        let _q = [0.3f32, 0.7];

        // Fisher inner product of tangent vector u = (1, -1) with itself at p.
        let u = [1.0f32, -1.0];
        let g_uu = fisher_inner_product(&p, &u, &u).unwrap();
        // g_uu = 1/0.5 + 1/0.5 = 4.
        assert!(
            (g_uu - 4.0).abs() < 1e-10,
            "Fisher g(u,u) at p=(0.5,0.5) should be 4, got {}",
            g_uu
        );
    }

    #[test]
    fn fisher_distance_self_zero() {
        let p = [0.3f32, 0.3, 0.4];
        let d = fisher_distance(&p, &p).unwrap();
        assert!(d.abs() < 1e-10, "Fisher distance to self must be 0");
    }

    #[test]
    fn fisher_distance_orthogonal_max() {
        // Disjoint supports → cos(θ) = 0 → θ = π/2.
        let p = [1.0f32, 0.0];
        let q = [0.0f32, 1.0];
        let d = fisher_distance(&p, &q).unwrap();
        assert!(
            (d - core::f64::consts::FRAC_PI_2).abs() < 1e-10,
            "Fisher distance between disjoint supports should be π/2, got {}",
            d
        );
    }

    #[test]
    fn simplex_projection_is_idempotent() {
        let p = [0.3f32, 0.7, 0.0];
        assert!(
            simplex_project_idempotent(&p),
            "projection must be idempotent on valid distribution"
        );

        let p2 = [0.5f32, 0.5];
        assert!(
            simplex_project_idempotent(&p2),
            "projection must be idempotent on valid distribution"
        );
    }

    #[test]
    fn simplex_project_normalises() {
        let p = [1.0f32, 2.0, 3.0]; // sums to 6
        let mut out = [0.0f32; 3];
        simplex_project(&p, &mut out).unwrap();
        let sum: f32 = out.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "projected must sum to 1");
        for &v in &out {
            assert!(v >= 0.0, "projected must be non-negative");
        }
    }

    #[test]
    fn simplex_project_clips_negatives() {
        let p = [-0.5f32, 2.0, 1.0]; // has negative
        let mut out = [0.0f32; 3];
        simplex_project(&p, &mut out).unwrap();
        for &v in &out {
            assert!(v >= 0.0, "projected must clip negatives");
        }
    }

    #[test]
    fn bregman_pythagorean_identity() {
        // S = {q ∈ Δ² : q₁ = q₂} (affine set on the simplex).
        // p = (0.3, 0.1, 0.6), I-projection onto S is q* = (0.2, 0.2, 0.6).
        // q = (0.15, 0.15, 0.7) is another point in S.
        // Identity: KL(p||q) = KL(p||q*) + KL(q*||q)
        let p = [0.3f32, 0.1, 0.6];
        let q_star = [0.2f32, 0.2, 0.6];
        let q = [0.15f32, 0.15, 0.7];

        let (kl_p_q, kl_p_qstar, kl_qstar_q) = bregman_pythagorean_test(&p, &q_star, &q).unwrap();

        let lhs = kl_p_q;
        let rhs = kl_p_qstar + kl_qstar_q;
        assert!(
            (lhs - rhs).abs() < 1e-6,
            "Bregman-Pythagorean: KL(p||q)={} should equal KL(p||q*)+KL(q*||q)={}",
            lhs,
            rhs
        );
    }

    #[test]
    fn validate_rejects_negative() {
        let p = [0.5f32, -0.3, 0.8];
        let err = validate_probability(&p).unwrap_err();
        assert!(matches!(
            err,
            StatManifoldError::NegativeMass { index: 1, .. }
        ));
    }

    #[test]
    fn validate_rejects_zero_support() {
        let p = [0.0f32, 0.0, 0.0];
        let err = validate_probability(&p).unwrap_err();
        assert!(matches!(err, StatManifoldError::ZeroSupport));
    }

    #[test]
    fn validate_rejects_not_normalised() {
        let p = [0.5f32, 0.3, 0.3]; // sums to 1.1
        let err = validate_probability(&p).unwrap_err();
        assert!(matches!(err, StatManifoldError::NotNormalised { .. }));
    }

    #[test]
    fn validate_accepts_valid() {
        let p = [0.3f32, 0.3, 0.4];
        assert!(validate_probability(&p).is_ok());
    }

    #[test]
    fn kl_infinite_when_q_zero_and_p_positive() {
        let p = [0.5f32, 0.5];
        let q = [1.0f32, 0.0];
        let kl = kl_divergence(&p, &q).unwrap();
        assert!(
            kl.is_infinite(),
            "KL must be infinite when q has zero support"
        );
    }

    #[test]
    fn kl_positive_for_different_distributions() {
        let p = [0.5f32, 0.5];
        let q = [0.9f32, 0.1];
        let kl = kl_divergence(&p, &q).unwrap();
        assert!(kl > 0.0, "KL must be positive for different distributions");
    }

    #[test]
    fn neg_entropy_is_negative() {
        // ψ(p) = Σ pᵢ log(pᵢ) = -H(p), and H(p) ≥ 0, so ψ(p) ≤ 0.
        let p = [0.3f32, 0.3, 0.4];
        let psi = neg_entropy(&p);
        assert!(psi <= 0.0, "negative entropy must be ≤ 0, got {}", psi);
    }

    #[test]
    fn neg_entropy_uniform_is_log_n() {
        // For uniform p = (1/n,…,1/n): ψ = Σ (1/n) log(1/n) = log(1/n) = -log(n).
        let p = [0.25f32, 0.25, 0.25, 0.25];
        let psi = neg_entropy(&p);
        // ψ(uniform) = Σ (1/n) log(1/n) = log(1/n) = -log(n)
        let expected = (0.25f64).ln(); // = -log(4) = -1.386...
        assert!(
            (psi - expected).abs() < 1e-10,
            "neg entropy of uniform should be -log(n) = {}, got {}",
            expected,
            psi
        );
    }

    #[test]
    fn determinism_probability_hash() {
        let p = [0.3f32, 0.3, 0.4];
        let h1 = probability_hash(&p);
        let h2 = probability_hash(&p);
        assert_eq!(h1, h2, "hash must be deterministic");
    }

    #[test]
    fn determinism_kl() {
        let p = [0.2f32, 0.5, 0.3];
        let q = [0.4f32, 0.4, 0.2];
        let kl1 = kl_divergence(&p, &q).unwrap();
        let kl2 = kl_divergence(&p, &q).unwrap();
        assert_eq!(kl1.to_bits(), kl2.to_bits(), "KL must be bit-identical");
    }

    #[test]
    fn determinism_fisher_distance() {
        let p = [0.3f32, 0.7];
        let q = [0.5f32, 0.5];
        let d1 = fisher_distance(&p, &q).unwrap();
        let d2 = fisher_distance(&p, &q).unwrap();
        assert_eq!(
            d1.to_bits(),
            d2.to_bits(),
            "Fisher distance must be bit-identical"
        );
    }

    #[test]
    fn fisher_distance_delta1_known_value() {
        // On Δ¹, Fisher distance between (1,0) and (0.5,0.5):
        // cos(θ) = √(1*0.5) + √(0*0.5) = √0.5
        // θ = arccos(√0.5) = π/4
        let p = [1.0f32, 0.0];
        let q = [0.5f32, 0.5];
        let d = fisher_distance(&p, &q).unwrap();
        assert!(
            (d - core::f64::consts::FRAC_PI_4).abs() < 1e-10,
            "Fisher distance (1,0)→(0.5,0.5) should be π/4, got {}",
            d
        );
    }

    #[test]
    fn fisher_inner_product_known_value() {
        // At p = (0.5, 0.5), u = (1, 0):
        // g(u,u) = 1²/0.5 + 0²/0.5 = 2
        let p = [0.5f32, 0.5];
        let u = [1.0f32, 0.0];
        let g = fisher_inner_product(&p, &u, &u).unwrap();
        assert!(
            (g - 2.0).abs() < 1e-10,
            "Fisher g(u,u) should be 2, got {}",
            g
        );
    }
}
