//! Fuzzy RDF schema — graded entailment (Ma, Li & Ma ch 3.3). RDFS reasoning with
//! a degree: `subClassOf`/`type` hold to a degree in `[0,1]`, and degrees compose
//! along the class hierarchy by a t-norm.
//!
//! Mission fit: "this guardianship relation is 0.6 a `MedicalProxy`" is a *degree of
//! role-holding*; propagating it through the hierarchy with a t-norm reasons about
//! **partial agency** without faking a crisp claim. Reuses the existing fuzzy
//! operators ([`crate::modalities::fuzzy`]); kernel-class `Reduction`.

use crate::modalities::fuzzy::{t_conorm_godel, t_norm_product};

/// Graded transitive closure of `subClassOf`. Input edges `(sub, super, degree)`
/// over `n` classes. The closure degree that `a ⊑ c` is the **best (t-conorm/max)
/// over all paths** of the **t-norm (product) along each path**. Diagonal is `1.0`
/// (every class is a subclass of itself with full degree). Returns the `n×n`
/// row-major degree matrix.
pub fn subclass_closure(n: usize, edges: &[(usize, usize, f64)]) -> Vec<f64> {
    let mut m = vec![0.0f32; n * n];
    for i in 0..n {
        m[i * n + i] = 1.0;
    }
    for &(a, b, d) in edges {
        if a < n && b < n {
            let dd = d.clamp(0.0, 1.0) as f32;
            // Keep the strongest direct assertion.
            if dd > m[a * n + b] {
                m[a * n + b] = dd;
            }
        }
    }
    // Fuzzy transitive closure (Floyd-Warshall with product t-norm + max t-conorm).
    for k in 0..n {
        for i in 0..n {
            let ik = m[i * n + k];
            if ik == 0.0 {
                continue;
            }
            for j in 0..n {
                let via = t_norm_product(ik, m[k * n + j]);
                let cur = m[i * n + j];
                m[i * n + j] = t_conorm_godel(cur, via);
            }
        }
    }
    m.iter().map(|&v| v as f64).collect()
}

/// Degree to which an instance is of class `c`, given it is of class `a` with degree
/// `type_degree` and `a ⊑ c` with `subclass_degree`: `t-norm(type, subclass)`.
pub fn type_entailment(type_degree: f64, subclass_degree: f64) -> f64 {
    t_norm_product(
        type_degree.clamp(0.0, 1.0) as f32,
        subclass_degree.clamp(0.0, 1.0) as f32,
    ) as f64
}

/// Convenience: given the closure matrix and an instance's direct `type` degrees per
/// class (`type_of[c]`), the entailed degree of membership in class `c` =
/// `max_a t-norm(type_of[a], closure[a][c])`. Returns the entailed type degrees.
pub fn entailed_types(n: usize, closure: &[f64], type_of: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; n];
    for c in 0..n {
        let mut best = 0.0f64;
        for a in 0..n {
            if type_of[a] > 0.0 {
                let d = type_entailment(type_of[a], closure[a * n + c]);
                if d > best {
                    best = d;
                }
            }
        }
        out[c] = best;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-6;

    #[test]
    fn transitive_degree_composes_by_product() {
        // A ⊑ B (0.8), B ⊑ C (0.7) → A ⊑ C = 0.8·0.7 = 0.56.
        let m = subclass_closure(3, &[(0, 1, 0.8), (1, 2, 0.7)]);
        assert!((m[0 * 3 + 1] - 0.8).abs() < EPS);
        assert!((m[1 * 3 + 2] - 0.7).abs() < EPS);
        assert!((m[0 * 3 + 2] - 0.56).abs() < EPS, "A⊑C {}", m[0 * 3 + 2]);
        // Reflexive.
        assert!((m[0 * 3 + 0] - 1.0).abs() < EPS);
    }

    #[test]
    fn best_path_wins() {
        // Two paths A→C: A→B→C (0.9·0.5=0.45) and A→D→C (0.6·0.9=0.54). Max = 0.54.
        let edges = [(0, 1, 0.9), (1, 2, 0.5), (0, 3, 0.6), (3, 2, 0.9)];
        let m = subclass_closure(4, &edges);
        assert!(
            (m[0 * 4 + 2] - 0.54).abs() < EPS,
            "best path {}",
            m[0 * 4 + 2]
        );
    }

    #[test]
    fn type_entailment_propagates() {
        // type(x, A)=0.6, A ⊑ MedicalProxy(=class 1) with degree 0.8 → 0.48.
        let m = subclass_closure(2, &[(0, 1, 0.8)]);
        let types = entailed_types(2, &m, &[0.6, 0.0]);
        assert!((types[1] - 0.48).abs() < EPS, "entailed {}", types[1]);
        // x is still fully of its own class A.
        assert!((types[0] - 0.6).abs() < EPS);
    }

    #[test]
    fn type_entailment_pairwise() {
        assert!((type_entailment(0.5, 0.4) - 0.2).abs() < EPS);
    }
}
