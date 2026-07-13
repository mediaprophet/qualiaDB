//! Score functions and their analytic gradients for the four embedding families.
//!
//! Vector layout conventions (rank `k`):
//! * **TransE / DistMult** — entity and relation are length `k` real vectors.
//! * **ComplEx** — entity and relation are length `2k`: `[re(0..k), im(k..2k)]`.
//! * **RotatE** — entity is length `2k` `[re, im]`; relation is length `k` of phase
//!   angles `θ` (the rotation `e^{iθ}` has unit modulus by construction).
//!
//! Every score returns a value where **higher = more plausible** (translational
//! models return the negative distance). The gradient helpers return `∂score/∂·` for
//! each of head, relation and tail, used by [`super::train`].

use super::KgEmbeddingError;

/// The embedding score family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreModel {
    /// `score = -‖h + r − t‖_p` (Bordes et al. 2013). `p` is the norm order.
    TransE { p: u8 },
    /// `score = ⟨h, r, t⟩ = Σ h_i r_i t_i` (Yang et al. 2014). Symmetric.
    DistMult,
    /// `score = Re(⟨h, r, t̄⟩)` over complex embeddings (Trouillon et al. 2016).
    ComplEx,
    /// `score = −Σ_i |h_i ∘ e^{iθ_i} − t_i|` (Sun et al. 2019). Models composition.
    RotatE,
}

impl ScoreModel {
    /// Storage length `(ent_dim, rel_dim)` per vector at rank `k`.
    pub fn dims(self, k: usize) -> (usize, usize) {
        match self {
            ScoreModel::TransE { .. } | ScoreModel::DistMult => (k, k),
            ScoreModel::ComplEx => (2 * k, 2 * k),
            ScoreModel::RotatE => (2 * k, k),
        }
    }

    /// Validate slice lengths against the model at rank `k`.
    fn check(self, h: &[f64], r: &[f64], t: &[f64], k: usize) -> Result<(), KgEmbeddingError> {
        let (ed, rd) = self.dims(k);
        if h.len() == ed && t.len() == ed && r.len() == rd {
            Ok(())
        } else {
            Err(KgEmbeddingError::InvalidDimension)
        }
    }

    /// Plausibility score; higher is better. Fails closed on a length mismatch.
    pub fn score(self, h: &[f64], r: &[f64], t: &[f64], k: usize) -> Result<f64, KgEmbeddingError> {
        self.check(h, r, t, k)?;
        Ok(match self {
            ScoreModel::TransE { p } => transe_score(h, r, t, p),
            ScoreModel::DistMult => distmult_score(h, r, t),
            ScoreModel::ComplEx => complex_score(h, r, t, k),
            ScoreModel::RotatE => rotate_score(h, r, t, k),
        })
    }

    /// `∂score/∂h`, `∂score/∂r`, `∂score/∂t` into the caller's buffers (lengths must
    /// match the model dims). Used by the trainer.
    pub fn gradient(
        self,
        h: &[f64],
        r: &[f64],
        t: &[f64],
        k: usize,
        gh: &mut [f64],
        gr: &mut [f64],
        gt: &mut [f64],
    ) -> Result<(), KgEmbeddingError> {
        self.check(h, r, t, k)?;
        if gh.len() != h.len() || gr.len() != r.len() || gt.len() != t.len() {
            return Err(KgEmbeddingError::InvalidDimension);
        }
        match self {
            ScoreModel::TransE { p } => transe_grad(h, r, t, p, gh, gr, gt),
            ScoreModel::DistMult => distmult_grad(h, r, t, gh, gr, gt),
            ScoreModel::ComplEx => complex_grad(h, r, t, k, gh, gr, gt),
            ScoreModel::RotatE => rotate_grad(h, r, t, k, gh, gr, gt),
        }
        Ok(())
    }
}

// ── TransE: score = -‖h + r - t‖_p ───────────────────────────────────────────

fn transe_distance(h: &[f64], r: &[f64], t: &[f64], p: u8) -> f64 {
    if p == 1 {
        h.iter()
            .zip(r)
            .zip(t)
            .map(|((&a, &b), &c)| (a + b - c).abs())
            .sum()
    } else {
        h.iter()
            .zip(r)
            .zip(t)
            .map(|((&a, &b), &c)| (a + b - c).powi(2))
            .sum::<f64>()
            .sqrt()
    }
}

pub fn transe_score(h: &[f64], r: &[f64], t: &[f64], p: u8) -> f64 {
    -transe_distance(h, r, t, p)
}

fn transe_grad(
    h: &[f64],
    r: &[f64],
    t: &[f64],
    p: u8,
    gh: &mut [f64],
    gr: &mut [f64],
    gt: &mut [f64],
) {
    // score = -d. For L2, ∂d/∂h_i = u_i/d where u = h+r-t; ∂(-d)/∂h = -u/d.
    // For L1, ∂d/∂h_i = sign(u_i).
    if p == 1 {
        for i in 0..h.len() {
            let u = h[i] + r[i] - t[i];
            let s = if u > 0.0 {
                1.0
            } else if u < 0.0 {
                -1.0
            } else {
                0.0
            };
            gh[i] = -s;
            gr[i] = -s;
            gt[i] = s;
        }
    } else {
        let d = transe_distance(h, r, t, 2).max(1e-12);
        for i in 0..h.len() {
            let u = h[i] + r[i] - t[i];
            let g = u / d;
            gh[i] = -g;
            gr[i] = -g;
            gt[i] = g;
        }
    }
}

// ── DistMult: score = Σ h_i r_i t_i ──────────────────────────────────────────

pub fn distmult_score(h: &[f64], r: &[f64], t: &[f64]) -> f64 {
    h.iter().zip(r).zip(t).map(|((&a, &b), &c)| a * b * c).sum()
}

fn distmult_grad(h: &[f64], r: &[f64], t: &[f64], gh: &mut [f64], gr: &mut [f64], gt: &mut [f64]) {
    for i in 0..h.len() {
        gh[i] = r[i] * t[i];
        gr[i] = h[i] * t[i];
        gt[i] = h[i] * r[i];
    }
}

// ── ComplEx: score = Re(Σ h_i r_i conj(t_i)) ─────────────────────────────────
// Layout [re(0..k), im(k..2k)]. With h=a+bi, r=c+di, t=e+fi:
//   Re = Σ (a c - b d) e + (a d + b c) f

pub fn complex_score(h: &[f64], r: &[f64], t: &[f64], k: usize) -> f64 {
    let mut s = 0.0;
    for i in 0..k {
        let (a, b) = (h[i], h[k + i]);
        let (c, d) = (r[i], r[k + i]);
        let (e, f) = (t[i], t[k + i]);
        s += (a * c - b * d) * e + (a * d + b * c) * f;
    }
    s
}

fn complex_grad(
    h: &[f64],
    r: &[f64],
    t: &[f64],
    k: usize,
    gh: &mut [f64],
    gr: &mut [f64],
    gt: &mut [f64],
) {
    for i in 0..k {
        let (a, b) = (h[i], h[k + i]);
        let (c, d) = (r[i], r[k + i]);
        let (e, f) = (t[i], t[k + i]);
        // ∂s/∂a, ∂s/∂b
        gh[i] = c * e + d * f;
        gh[k + i] = -d * e + c * f;
        // ∂s/∂c, ∂s/∂d
        gr[i] = a * e + b * f;
        gr[k + i] = -b * e + a * f;
        // ∂s/∂e, ∂s/∂f
        gt[i] = a * c - b * d;
        gt[k + i] = a * d + b * c;
    }
}

// ── RotatE: score = -Σ_i |h_i ∘ e^{iθ_i} - t_i| ──────────────────────────────
// Entity [re, im] length 2k; relation θ length k.

pub fn rotate_score(h: &[f64], r: &[f64], t: &[f64], k: usize) -> f64 {
    let mut d = 0.0;
    for i in 0..k {
        let (a, b) = (h[i], h[k + i]);
        let (cs, sn) = (r[i].cos(), r[i].sin());
        let (e, f) = (t[i], t[k + i]);
        let p = a * cs - b * sn; // rotated real
        let q = a * sn + b * cs; // rotated imag
        d += ((p - e).powi(2) + (q - f).powi(2)).sqrt();
    }
    -d
}

fn rotate_grad(
    h: &[f64],
    r: &[f64],
    t: &[f64],
    k: usize,
    gh: &mut [f64],
    gr: &mut [f64],
    gt: &mut [f64],
) {
    // score = -Σ m_i ; gradients of -m_i.
    for i in 0..k {
        let (a, b) = (h[i], h[k + i]);
        let theta = r[i];
        let (cs, sn) = (theta.cos(), theta.sin());
        let (e, f) = (t[i], t[k + i]);
        let p = a * cs - b * sn;
        let q = a * sn + b * cs;
        let dx = p - e;
        let dy = q - f;
        let m = (dx * dx + dy * dy).sqrt().max(1e-12);
        // ∂m/∂a = (dx*cs + dy*sn)/m ; ∂m/∂b = (-dx*sn + dy*cs)/m
        gh[i] = -(dx * cs + dy * sn) / m;
        gh[k + i] = -(-dx * sn + dy * cs) / m;
        // ∂m/∂θ = (dx*(-q) + dy*p)/m
        gr[i] = -(dx * (-q) + dy * p) / m;
        // ∂m/∂e = -dx/m ; ∂m/∂f = -dy/m → grad of -m flips sign
        gt[i] = dx / m;
        gt[k + i] = dy / m;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    #[test]
    fn transe_perfect_translation_scores_zero_distance() {
        // h + r = t exactly → distance 0 → score 0 (the maximum).
        let h = [1.0, 2.0];
        let r = [0.5, -1.0];
        let t = [1.5, 1.0];
        assert!((transe_score(&h, &r, &t, 2) - 0.0).abs() < EPS);
        // A wrong tail scores strictly lower (more negative).
        let bad = [9.0, 9.0];
        assert!(transe_score(&h, &r, &bad, 2) < transe_score(&h, &r, &t, 2));
    }

    #[test]
    fn distmult_matches_hand_value() {
        let h = [1.0, 2.0, 3.0];
        let r = [0.5, 0.5, 0.5];
        let t = [2.0, 1.0, 1.0];
        // Σ = 1*.5*2 + 2*.5*1 + 3*.5*1 = 1 + 1 + 1.5 = 3.5
        assert!((distmult_score(&h, &r, &t) - 3.5).abs() < EPS);
    }

    #[test]
    fn complex_reduces_to_distmult_when_imaginary_zero() {
        // With zero imaginary parts ComplEx ≡ DistMult.
        let k = 2;
        let h = [1.0, 2.0, 0.0, 0.0];
        let r = [0.5, 0.5, 0.0, 0.0];
        let t = [2.0, 1.0, 0.0, 0.0];
        let cx = complex_score(&h, &r, &t, k);
        let dm = distmult_score(&[1.0, 2.0], &[0.5, 0.5], &[2.0, 1.0]);
        assert!((cx - dm).abs() < EPS, "complex {cx} vs distmult {dm}");
    }

    #[test]
    fn rotate_zero_phase_is_translation_to_self() {
        // θ = 0 → rotation is identity → |h - t|. h == t → distance 0.
        let k = 2;
        let h = [1.0, 2.0, 0.5, -0.5];
        let r = [0.0, 0.0];
        let t = [1.0, 2.0, 0.5, -0.5];
        assert!((rotate_score(&h, &r, &t, k) - 0.0).abs() < EPS);
    }

    #[test]
    fn rotate_half_pi_rotates_real_to_imag() {
        // θ = π/2 rotates (a, b) → (-b, a). h=(1,0) → rotated (0,1); t=(0,1) → 0 dist.
        let k = 1;
        let h = [1.0, 0.0];
        let r = [std::f64::consts::FRAC_PI_2];
        let t = [0.0, 1.0];
        assert!((rotate_score(&h, &r, &t, k) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn gradients_match_finite_difference() {
        // Verify each model's analytic gradient against a numerical one.
        let cases: &[(ScoreModel, usize, &[f64], &[f64], &[f64])] = &[
            (
                ScoreModel::TransE { p: 2 },
                3,
                &[0.3, -0.4, 0.1],
                &[0.2, 0.5, -0.1],
                &[0.6, 0.0, 0.2],
            ),
            (
                ScoreModel::DistMult,
                3,
                &[0.3, -0.4, 0.1],
                &[0.2, 0.5, -0.1],
                &[0.6, 0.0, 0.2],
            ),
            (
                ScoreModel::ComplEx,
                2,
                &[0.3, -0.4, 0.1, 0.2],
                &[0.2, 0.5, -0.1, 0.3],
                &[0.6, 0.0, 0.2, -0.2],
            ),
            (
                ScoreModel::RotatE,
                2,
                &[0.3, -0.4, 0.1, 0.2],
                &[0.7, -0.3],
                &[0.6, 0.1, 0.2, -0.2],
            ),
        ];
        let eps = 1e-6;
        for &(model, k, h, r, t) in cases {
            let (ed, rd) = model.dims(k);
            let mut gh = vec![0.0; ed];
            let mut gr = vec![0.0; rd];
            let mut gt = vec![0.0; ed];
            model
                .gradient(h, r, t, k, &mut gh, &mut gr, &mut gt)
                .unwrap();
            // Numerical ∂score/∂h
            for i in 0..ed {
                let mut hp = h.to_vec();
                hp[i] += eps;
                let mut hm = h.to_vec();
                hm[i] -= eps;
                let num = (model.score(&hp, r, t, k).unwrap() - model.score(&hm, r, t, k).unwrap())
                    / (2.0 * eps);
                assert!(
                    (gh[i] - num).abs() < 1e-4,
                    "{model:?} gh[{i}] {} vs {num}",
                    gh[i]
                );
            }
            for i in 0..rd {
                let mut rp = r.to_vec();
                rp[i] += eps;
                let mut rm = r.to_vec();
                rm[i] -= eps;
                let num = (model.score(h, &rp, t, k).unwrap() - model.score(h, &rm, t, k).unwrap())
                    / (2.0 * eps);
                assert!(
                    (gr[i] - num).abs() < 1e-4,
                    "{model:?} gr[{i}] {} vs {num}",
                    gr[i]
                );
            }
        }
    }

    #[test]
    fn score_fails_closed_on_dim_mismatch() {
        let m = ScoreModel::DistMult;
        assert_eq!(
            m.score(&[1.0, 2.0], &[1.0], &[1.0, 2.0], 2).unwrap_err(),
            KgEmbeddingError::InvalidDimension
        );
    }
}
