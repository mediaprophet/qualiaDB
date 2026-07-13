//! Interval type-2 fuzzy sets (CI-SKM ch 6) — uncertainty *about* the membership
//! degree itself. A type-1 fuzzy degree is a single number "0.7"; an interval
//! type-2 degree is `[0.55, 0.85]` — the **footprint of uncertainty** — so the
//! system can honestly say "about 0.7, but I'm not even sure how sure."
//!
//! Mission fit: this is the native encoding of confidence-about-confidence and the
//! out-of-band remainder — a false-precise single degree pretends to a certainty
//! the system does not have. Reuses the type-1 t-norm/t-conorm operators
//! ([`crate::modalities::fuzzy`]) as the degenerate (`lower == upper`) case.
//!
//! Type-reduction (interval → crisp) uses the **Karnik-Mendel** procedure, the
//! standard IT2 centroid algorithm. Kernel-class `ElementwiseMap`.

use crate::modalities::fuzzy::{t_conorm_godel, t_norm_godel};

/// An interval type-2 membership degree `[lower, upper] ⊆ [0,1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntervalType2 {
    pub lower: f32,
    pub upper: f32,
}

impl IntervalType2 {
    /// Construct, clamping to `[0,1]` and ordering so `lower ≤ upper`.
    pub fn new(a: f32, b: f32) -> Self {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        Self {
            lower: lo.clamp(0.0, 1.0),
            upper: hi.clamp(0.0, 1.0),
        }
    }

    /// A crisp (type-1) degree as the degenerate interval `[d, d]`.
    pub fn crisp(d: f32) -> Self {
        let d = d.clamp(0.0, 1.0);
        Self { lower: d, upper: d }
    }

    /// The footprint of uncertainty — how uncertain the degree itself is.
    pub fn footprint(self) -> f32 {
        self.upper - self.lower
    }

    /// Meet (type-2 conjunction): apply a t-norm bound-wise. Default Gödel (min).
    pub fn meet(self, other: Self) -> Self {
        Self {
            lower: t_norm_godel(self.lower, other.lower),
            upper: t_norm_godel(self.upper, other.upper),
        }
    }

    /// Join (type-2 disjunction): t-conorm bound-wise. Default Gödel (max).
    pub fn join(self, other: Self) -> Self {
        Self {
            lower: t_conorm_godel(self.lower, other.lower),
            upper: t_conorm_godel(self.upper, other.upper),
        }
    }

    /// Complement `[1−upper, 1−lower]`.
    pub fn complement(self) -> Self {
        Self {
            lower: 1.0 - self.upper,
            upper: 1.0 - self.lower,
        }
    }

    /// Type-reduce a single interval to a crisp degree (interval midpoint).
    pub fn type_reduce(self) -> f32 {
        0.5 * (self.lower + self.upper)
    }
}

/// Karnik-Mendel centroid of a discretized interval type-2 fuzzy set: points `x`
/// with lower/upper membership grades `lmf`/`umf` (all same length, `x` sorted
/// ascending). Returns the centroid interval `[c_left, c_right]`. `None` on a
/// length mismatch / empty / zero total membership.
pub fn karnik_mendel(x: &[f32], lmf: &[f32], umf: &[f32]) -> Option<(f32, f32)> {
    let n = x.len();
    if n == 0 || lmf.len() != n || umf.len() != n {
        return None;
    }
    let c_left = km_endpoint(x, lmf, umf, true)?;
    let c_right = km_endpoint(x, lmf, umf, false)?;
    Some((c_left, c_right))
}

/// One Karnik-Mendel endpoint. `left = true` computes the smallest centroid
/// (upper MF below the switch, lower MF above); `left = false` the largest.
fn km_endpoint(x: &[f32], lmf: &[f32], umf: &[f32], left: bool) -> Option<f32> {
    let n = x.len();
    // Initial θ = average MF.
    let mut theta: Vec<f32> = (0..n).map(|i| 0.5 * (lmf[i] + umf[i])).collect();
    let weighted = |theta: &[f32]| -> Option<f32> {
        let den: f32 = theta.iter().sum();
        if den <= 0.0 {
            return None;
        }
        Some(x.iter().zip(theta).map(|(xi, ti)| xi * ti).sum::<f32>() / den)
    };
    let mut y = weighted(&theta)?;
    for _ in 0..100 {
        // Switch point k: x[k] ≤ y ≤ x[k+1].
        let mut k = 0;
        while k + 1 < n && x[k + 1] < y {
            k += 1;
        }
        // Reassign θ around the switch.
        for i in 0..n {
            let below = i <= k;
            theta[i] = if left == below { umf[i] } else { lmf[i] };
        }
        let y_new = weighted(&theta)?;
        if (y_new - y).abs() < 1e-7 {
            return Some(y_new);
        }
        y = y_new;
    }
    Some(y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footprint_and_crisp_degenerate() {
        let it2 = IntervalType2::new(0.55, 0.85);
        assert!((it2.footprint() - 0.30).abs() < 1e-6);
        // A crisp degree has zero footprint and type-reduces to itself.
        let c = IntervalType2::crisp(0.7);
        assert_eq!(c.footprint(), 0.0);
        assert!((c.type_reduce() - 0.7).abs() < 1e-6);
        // Construction orders the bounds.
        assert_eq!(
            IntervalType2::new(0.9, 0.2),
            IntervalType2 {
                lower: 0.2,
                upper: 0.9
            }
        );
    }

    #[test]
    fn meet_join_reduce_to_type1_on_crisp() {
        // With zero footprint, meet/join match the type-1 min/max.
        let a = IntervalType2::crisp(0.3);
        let b = IntervalType2::crisp(0.8);
        assert!((a.meet(b).type_reduce() - 0.3).abs() < 1e-6); // min
        assert!((a.join(b).type_reduce() - 0.8).abs() < 1e-6); // max
    }

    #[test]
    fn meet_widens_or_preserves_uncertainty_sensibly() {
        let a = IntervalType2::new(0.4, 0.7);
        let b = IntervalType2::new(0.5, 0.9);
        let m = a.meet(b); // [min(.4,.5), min(.7,.9)] = [.4,.7]
        assert!((m.lower - 0.4).abs() < 1e-6 && (m.upper - 0.7).abs() < 1e-6);
        let j = a.join(b); // [max,max] = [.5,.9]
        assert!((j.lower - 0.5).abs() < 1e-6 && (j.upper - 0.9).abs() < 1e-6);
    }

    #[test]
    fn karnik_mendel_brackets_the_centroid() {
        // Symmetric set centered at x=2 → centroid interval brackets 2.
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        let lmf = [0.1, 0.4, 0.6, 0.4, 0.1];
        let umf = [0.3, 0.7, 0.9, 0.7, 0.3];
        let (cl, cr) = karnik_mendel(&x, &lmf, &umf).unwrap();
        assert!(
            cl <= 2.0 + 1e-4 && cr >= 2.0 - 1e-4,
            "centroid [{cl},{cr}] should bracket 2"
        );
        assert!(cl <= cr);
    }

    #[test]
    fn guards() {
        assert!(karnik_mendel(&[], &[], &[]).is_none());
        assert!(karnik_mendel(&[1.0], &[0.5], &[0.5, 0.6]).is_none());
    }
}
