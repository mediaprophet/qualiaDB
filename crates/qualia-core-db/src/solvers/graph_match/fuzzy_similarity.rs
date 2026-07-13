//! Fuzzy RDF graph similarity (Ma, Li & Ma ch 3.4) — a degree-aware similarity
//! between two fuzzy RDF graphs (triples carrying a membership degree). The fuzzy
//! Jaccard generalizes set overlap to graded membership: shared structure counts in
//! proportion to *how strongly* both graphs assert it. Kernel-class `Reduction`.

use std::collections::HashMap;

/// An RDF triple `(s, p, o)` with a membership degree in `[0,1]`. Terms are term
/// ids (interned URIs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuzzyTriple {
    pub s: usize,
    pub p: usize,
    pub o: usize,
    pub degree: f64,
}

fn to_map(g: &[FuzzyTriple]) -> HashMap<(usize, usize, usize), f64> {
    let mut m = HashMap::new();
    for t in g {
        // Keep the strongest degree if a triple repeats.
        let e = m.entry((t.s, t.p, t.o)).or_insert(0.0);
        if t.degree > *e {
            *e = t.degree.clamp(0.0, 1.0);
        }
    }
    m
}

/// Fuzzy Jaccard similarity `Σ min(d₁,d₂) / Σ max(d₁,d₂)` over the union of triples
/// (a missing triple has degree 0). Returns `[0,1]`; `1.0` for identical graphs,
/// `0.0` for disjoint ones. Two empty graphs are defined as similarity `1.0`.
pub fn fuzzy_jaccard(g1: &[FuzzyTriple], g2: &[FuzzyTriple]) -> f64 {
    let m1 = to_map(g1);
    let m2 = to_map(g2);
    let mut keys: std::collections::HashSet<(usize, usize, usize)> = m1.keys().copied().collect();
    keys.extend(m2.keys().copied());
    if keys.is_empty() {
        return 1.0;
    }
    let mut num = 0.0;
    let mut den = 0.0;
    for k in keys {
        let a = *m1.get(&k).unwrap_or(&0.0);
        let b = *m2.get(&k).unwrap_or(&0.0);
        num += a.min(b);
        den += a.max(b);
    }
    if den > 0.0 {
        num / den
    } else {
        1.0
    }
}

/// Degree-weighted overlap (Dice-style): `2·Σ min / (Σd₁ + Σd₂)`. An alternative
/// emphasizing shared mass. Returns `[0,1]`.
pub fn fuzzy_dice(g1: &[FuzzyTriple], g2: &[FuzzyTriple]) -> f64 {
    let m1 = to_map(g1);
    let m2 = to_map(g2);
    let sum1: f64 = m1.values().sum();
    let sum2: f64 = m2.values().sum();
    if sum1 + sum2 == 0.0 {
        return 1.0;
    }
    let mut inter = 0.0;
    for (k, &a) in &m1 {
        if let Some(&b) = m2.get(k) {
            inter += a.min(b);
        }
    }
    2.0 * inter / (sum1 + sum2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: usize, p: usize, o: usize, d: f64) -> FuzzyTriple {
        FuzzyTriple { s, p, o, degree: d }
    }

    #[test]
    fn identical_graphs_are_one() {
        let g = [t(0, 1, 2, 0.8), t(2, 1, 3, 0.5)];
        assert!((fuzzy_jaccard(&g, &g) - 1.0).abs() < 1e-12);
        assert!((fuzzy_dice(&g, &g) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn disjoint_graphs_are_zero() {
        let g1 = [t(0, 1, 2, 0.9)];
        let g2 = [t(5, 6, 7, 0.9)];
        assert!(fuzzy_jaccard(&g1, &g2).abs() < 1e-12);
    }

    #[test]
    fn partial_overlap_with_degrees() {
        // Shared triple at degrees 0.8 vs 0.4; one extra in each.
        let g1 = [t(0, 1, 2, 0.8), t(3, 1, 4, 0.6)];
        let g2 = [t(0, 1, 2, 0.4), t(5, 1, 6, 0.7)];
        let j = fuzzy_jaccard(&g1, &g2);
        // num = min(.8,.4)=0.4 ; den = max(.8,.4)+0.6+0.7 = 0.8+0.6+0.7 = 2.1 → 0.19.
        assert!((j - 0.4 / 2.1).abs() < 1e-9, "jaccard {j}");
        assert!(j > 0.0 && j < 1.0);
    }

    #[test]
    fn empty_graphs_are_similar() {
        assert!((fuzzy_jaccard(&[], &[]) - 1.0).abs() < 1e-12);
    }
}
