use crate::NQuin;

/// Many-valued / fuzzy logic over truth degrees in `[0, 1]`. Distinct from the
/// Bayesian `probabilistic` modality: fuzzy conjunction uses a t-norm (not a
/// product), modelling DEGREES of (partial) satisfaction — e.g. a right that is
/// partially fulfilled. Each proposition carries its truth degree as an f32 in the
/// quin `metadata`. Zero-heap throughout.

/// Gödel t-norm (fuzzy AND) — the minimum.
#[inline]
pub fn t_norm_godel(a: f32, b: f32) -> f32 {
    a.min(b)
}

/// Łukasiewicz t-norm (fuzzy AND) — `max(0, a + b - 1)`.
#[inline]
pub fn t_norm_lukasiewicz(a: f32, b: f32) -> f32 {
    (a + b - 1.0).max(0.0)
}

/// Gödel t-conorm (fuzzy OR) — the maximum.
#[inline]
pub fn t_conorm_godel(a: f32, b: f32) -> f32 {
    a.max(b)
}

/// Read a proposition's fuzzy truth degree (canonical f32 in `metadata`, via the
/// FrameLayout ABI), clamped to [0,1].
#[inline]
pub fn degree(quin: &NQuin) -> f32 {
    crate::frame_layout::truth_degree(quin.metadata).clamp(0.0, 1.0)
}

/// Fuzzy conjunction (Gödel t-norm = min) of the truth degrees carried by `quins`.
/// Empty input → 1.0 (the t-norm identity). Zero-heap.
pub fn conjunction(quins: &[NQuin]) -> f32 {
    let mut acc = 1.0f32;
    for q in quins {
        acc = t_norm_godel(acc, degree(q));
    }
    acc
}

// ─── T-norm / T-conorm families (Gödel, Łukasiewicz, Product, Drastic) ──────────────

/// Łukasiewicz t-conorm (fuzzy OR) — `min(1, a + b)`.
#[inline]
pub fn t_conorm_lukasiewicz(a: f32, b: f32) -> f32 {
    (a + b).min(1.0)
}

/// Product t-norm (fuzzy AND) — `a · b` (the probabilistic/algebraic conjunction).
#[inline]
pub fn t_norm_product(a: f32, b: f32) -> f32 {
    a * b
}

/// Product t-conorm (fuzzy OR) — `a + b - a·b` (the probabilistic sum).
#[inline]
pub fn t_conorm_product(a: f32, b: f32) -> f32 {
    a + b - a * b
}

/// Drastic t-norm — the smallest t-norm: `b` if `a==1`, `a` if `b==1`, else `0`.
#[inline]
pub fn t_norm_drastic(a: f32, b: f32) -> f32 {
    if a >= 1.0 { b } else if b >= 1.0 { a } else { 0.0 }
}

/// Drastic t-conorm — the largest t-conorm: `b` if `a==0`, `a` if `b==0`, else `1`.
#[inline]
pub fn t_conorm_drastic(a: f32, b: f32) -> f32 {
    if a <= 0.0 { b } else if b <= 0.0 { a } else { 1.0 }
}

/// Standard fuzzy negation (complement) — `1 - a`, clamped to [0,1].
#[inline]
pub fn fuzzy_not(a: f32) -> f32 {
    (1.0 - a).clamp(0.0, 1.0)
}

/// Fuzzy disjunction (Gödel t-conorm = max) of the truth degrees carried by `quins`.
/// Empty input → 0.0 (the t-conorm identity). Zero-heap.
pub fn disjunction(quins: &[NQuin]) -> f32 {
    let mut acc = 0.0f32;
    for q in quins {
        acc = t_conorm_godel(acc, degree(q));
    }
    acc
}

// ─── Linguistic hedges (Zadeh) ──────────────────────────────────────────────────────

/// Concentration hedge "very" — `μ²` (sharpens, lowers partial memberships).
#[inline]
pub fn hedge_very(mu: f32) -> f32 {
    mu * mu
}

/// Concentration hedge "extremely" — `μ³`.
#[inline]
pub fn hedge_extremely(mu: f32) -> f32 {
    mu * mu * mu
}

/// Dilation hedge "more or less" / "somewhat" — `√μ` (broadens, raises partial memberships).
#[inline]
pub fn hedge_more_or_less(mu: f32) -> f32 {
    mu.max(0.0).sqrt()
}

// ─── Defuzzification ────────────────────────────────────────────────────────────────
//
// A fuzzy output set is given as a discretised universe `u` (assumed monotonically
// increasing) with parallel membership `mu`. Each method collapses it to a crisp value.
// `None` when the slices mismatch / are empty / carry no mass. Zero-heap (slice scans).

/// Centroid / centre-of-gravity (COG): `Σ(uᵢ·μᵢ) / Σ μᵢ`.
pub fn defuzz_centroid(u: &[f32], mu: &[f32]) -> Option<f32> {
    if u.len() != mu.len() || u.is_empty() {
        return None;
    }
    let mut num = 0.0f32;
    let mut den = 0.0f32;
    for i in 0..u.len() {
        num += u[i] * mu[i];
        den += mu[i];
    }
    if den.abs() < 1e-9 { None } else { Some(num / den) }
}

/// Mean-of-Maximum (MOM): the mean of the universe points attaining maximum membership.
pub fn defuzz_mean_of_max(u: &[f32], mu: &[f32]) -> Option<f32> {
    if u.len() != mu.len() || u.is_empty() {
        return None;
    }
    let mut max = f32::MIN;
    for &m in mu {
        if m > max {
            max = m;
        }
    }
    let mut sum = 0.0f32;
    let mut n = 0u32;
    for i in 0..u.len() {
        if (mu[i] - max).abs() < 1e-6 {
            sum += u[i];
            n += 1;
        }
    }
    if n == 0 { None } else { Some(sum / n as f32) }
}

/// Smallest-of-Maximum (SOM): the smallest universe point attaining maximum membership.
pub fn defuzz_smallest_of_max(u: &[f32], mu: &[f32]) -> Option<f32> {
    if u.len() != mu.len() || u.is_empty() {
        return None;
    }
    let mut max = f32::MIN;
    for &m in mu {
        if m > max {
            max = m;
        }
    }
    for i in 0..u.len() {
        if (mu[i] - max).abs() < 1e-6 {
            return Some(u[i]); // u is increasing → first match is smallest
        }
    }
    None
}

/// Bisector: the universe point that splits the area under `μ` into two equal halves.
pub fn defuzz_bisector(u: &[f32], mu: &[f32]) -> Option<f32> {
    if u.len() != mu.len() || u.is_empty() {
        return None;
    }
    let total: f32 = mu.iter().sum();
    if total.abs() < 1e-9 {
        return None;
    }
    let half = total / 2.0;
    let mut cum = 0.0f32;
    for i in 0..u.len() {
        cum += mu[i];
        if cum >= half {
            return Some(u[i]);
        }
    }
    Some(u[u.len() - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_norms_and_conjunction() {
        assert!((t_norm_godel(0.7, 0.4) - 0.4).abs() < 1e-6);
        assert!((t_norm_lukasiewicz(0.7, 0.4) - 0.1).abs() < 1e-6);
        assert!((t_norm_lukasiewicz(0.3, 0.4) - 0.0).abs() < 1e-6);
        assert!((t_conorm_godel(0.7, 0.4) - 0.7).abs() < 1e-6);

        let mk = |d: f32| {
            let mut q = NQuin::default();
            q.metadata = d.to_bits() as u64;
            q
        };
        // min(0.9, 0.6, 0.8) = 0.6
        assert!((conjunction(&[mk(0.9), mk(0.6), mk(0.8)]) - 0.6).abs() < 1e-6);
        assert!((conjunction(&[]) - 1.0).abs() < 1e-6);
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn t_norm_conorm_families() {
        // Product family.
        assert!(close(t_norm_product(0.5, 0.4), 0.2));
        assert!(close(t_conorm_product(0.5, 0.4), 0.7)); // 0.5+0.4-0.2
        // Łukasiewicz t-conorm.
        assert!(close(t_conorm_lukasiewicz(0.7, 0.4), 1.0)); // min(1, 1.1)
        assert!(close(t_conorm_lukasiewicz(0.3, 0.4), 0.7));
        // Drastic: identity only when one operand is the unit/zero, else collapses.
        assert!(close(t_norm_drastic(1.0, 0.4), 0.4));
        assert!(close(t_norm_drastic(0.6, 0.4), 0.0));
        assert!(close(t_conorm_drastic(0.0, 0.4), 0.4));
        assert!(close(t_conorm_drastic(0.6, 0.4), 1.0));
        // Complement.
        assert!(close(fuzzy_not(0.3), 0.7));
        // Ordering: drastic ≤ product ≤ Gödel (t-norms); reverse for t-conorms.
        let (a, b) = (0.6f32, 0.4f32);
        assert!(t_norm_drastic(a, b) <= t_norm_product(a, b));
        assert!(t_norm_product(a, b) <= t_norm_godel(a, b));
    }

    #[test]
    fn hedges_concentrate_and_dilate() {
        assert!(close(hedge_very(0.5), 0.25));
        assert!(close(hedge_extremely(0.5), 0.125));
        assert!(close(hedge_more_or_less(0.25), 0.5));
        // "very" lowers a partial membership; "more or less" raises it.
        assert!(hedge_very(0.6) < 0.6);
        assert!(hedge_more_or_less(0.6) > 0.6);
    }

    #[test]
    fn defuzzification_methods() {
        // Symmetric triangle centred at 2.0 → centroid/bisector/MOM all 2.0.
        let u = [0.0, 1.0, 2.0, 3.0, 4.0];
        let mu = [0.0, 0.5, 1.0, 0.5, 0.0];
        assert!(close(defuzz_centroid(&u, &mu).unwrap(), 2.0));
        assert!(close(defuzz_mean_of_max(&u, &mu).unwrap(), 2.0));
        assert!(close(defuzz_smallest_of_max(&u, &mu).unwrap(), 2.0));
        assert!(close(defuzz_bisector(&u, &mu).unwrap(), 2.0));

        // Plateau at the max over {1.0, 2.0}: MOM = 1.5, SOM = 1.0.
        let mu2 = [0.0, 1.0, 1.0, 0.2, 0.0];
        assert!(close(defuzz_mean_of_max(&u, &mu2).unwrap(), 1.5));
        assert!(close(defuzz_smallest_of_max(&u, &mu2).unwrap(), 1.0));

        // Degenerate inputs refuse rather than divide by zero.
        assert!(defuzz_centroid(&u, &[0.0; 5]).is_none());
        assert!(defuzz_centroid(&[1.0, 2.0], &[0.1]).is_none());
        assert!(defuzz_centroid(&[], &[]).is_none());
    }
}
