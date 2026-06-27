//! Degree-annotated solutions and the algebra that composes them.
//!
//! A [`FuzzySolution`] is one of the engine's [`BindingRow`]s plus a truth degree. A
//! [`FuzzyResultSet`] is a sequence of them, with the relational-algebra-over-degrees
//! operations f-SPARQL needs: conjunctive **join** (BGP / `AND`, degree via t-norm),
//! **union** (`UNION`, t-conorm), **projection** (existential, max degree per distinct
//! projection), **negation**, **α-cut** threshold, and **ranking** by degree.

use super::DegreeNorm;
use crate::sparql_ast::{BindingRow, VariableId, MAX_BINDINGS};

/// One solution with its truth degree in `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuzzySolution {
    pub row: BindingRow,
    pub degree: f64,
}

impl FuzzySolution {
    pub fn new(row: BindingRow, degree: f64) -> Self {
        Self { row, degree: degree.clamp(0.0, 1.0) }
    }
}

/// Two rows are *compatible* iff every variable bound in both holds the same value.
fn compatible(a: &BindingRow, b: &BindingRow) -> bool {
    for i in 0..MAX_BINDINGS {
        if let (Some(x), Some(y)) = (a.slots[i], b.slots[i]) {
            if x != y {
                return false;
            }
        }
    }
    true
}

/// Merge two compatible rows (union of bindings).
fn merge(a: &BindingRow, b: &BindingRow) -> BindingRow {
    let mut out = *a;
    for i in 0..MAX_BINDINGS {
        if out.slots[i].is_none() {
            out.slots[i] = b.slots[i];
        }
    }
    out
}

/// A bag of degree-annotated solutions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FuzzyResultSet {
    pub solutions: Vec<FuzzySolution>,
}

impl FuzzyResultSet {
    pub fn new() -> Self {
        Self { solutions: Vec::new() }
    }

    pub fn from_solutions(solutions: Vec<FuzzySolution>) -> Self {
        Self { solutions }
    }

    pub fn push(&mut self, sol: FuzzySolution) {
        self.solutions.push(sol);
    }

    pub fn len(&self) -> usize {
        self.solutions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.solutions.is_empty()
    }

    /// α-cut: keep only solutions with degree `>= alpha`.
    pub fn threshold(mut self, alpha: f64) -> Self {
        self.solutions.retain(|s| s.degree >= alpha);
        self
    }

    /// Sort by degree, highest confidence first (stable).
    pub fn order_by_degree_desc(mut self) -> Self {
        self.solutions
            .sort_by(|a, b| b.degree.partial_cmp(&a.degree).unwrap_or(core::cmp::Ordering::Equal));
        self
    }

    /// Keep the `k` highest-degree solutions.
    pub fn top_k(mut self, k: usize) -> Self {
        self = self.order_by_degree_desc();
        self.solutions.truncate(k);
        self
    }

    /// Fuzzy negation: replace each degree with its complement under `norm`. (Used for
    /// `NOT`/`MINUS`-style scoring of how *un*-matched a solution is.)
    pub fn negate(mut self, norm: DegreeNorm) -> Self {
        for s in &mut self.solutions {
            s.degree = norm.not(s.degree);
        }
        self
    }

    /// Conjunctive **join** (basic graph pattern / `AND`): every compatible pair of
    /// solutions across the two sets yields a merged solution whose degree is the
    /// t-norm of the two input degrees.
    pub fn join(&self, other: &FuzzyResultSet, norm: DegreeNorm) -> FuzzyResultSet {
        let mut out = Vec::new();
        for a in &self.solutions {
            for b in &other.solutions {
                if compatible(&a.row, &b.row) {
                    out.push(FuzzySolution::new(merge(&a.row, &b.row), norm.and(a.degree, b.degree)));
                }
            }
        }
        FuzzyResultSet { solutions: out }
    }

    /// **Union** (`UNION`): solutions from both sets. Where the *same* row appears in
    /// both, its degrees combine via the t-conorm (the most confident wins under
    /// Gödel) rather than duplicating.
    pub fn union(&self, other: &FuzzyResultSet, norm: DegreeNorm) -> FuzzyResultSet {
        let mut out: Vec<FuzzySolution> = self.solutions.clone();
        for b in &other.solutions {
            if let Some(existing) = out.iter_mut().find(|a| a.row == b.row) {
                existing.degree = norm.or(existing.degree, b.degree);
            } else {
                out.push(*b);
            }
        }
        FuzzyResultSet { solutions: out }
    }

    /// **Projection** onto `vars` (existential): drop all other variable bindings; rows
    /// that become identical are merged, keeping the maximum degree (∃ over the
    /// projected-away variables — the t-conorm under Gödel).
    pub fn project(&self, vars: &[VariableId], norm: DegreeNorm) -> FuzzyResultSet {
        let keep = |row: &BindingRow| {
            let mut r = BindingRow::new();
            for &v in vars {
                if let Some(val) = row.get(v) {
                    r.set(v, val);
                }
            }
            r
        };
        let mut out: Vec<FuzzySolution> = Vec::new();
        for s in &self.solutions {
            let pr = keep(&s.row);
            if let Some(existing) = out.iter_mut().find(|e| e.row == pr) {
                existing.degree = norm.or(existing.degree, s.degree);
            } else {
                out.push(FuzzySolution::new(pr, s.degree));
            }
        }
        FuzzyResultSet { solutions: out }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pairs: &[(VariableId, u64)]) -> BindingRow {
        let mut r = BindingRow::new();
        for &(v, val) in pairs {
            r.set(v, val);
        }
        r
    }

    fn sol(pairs: &[(VariableId, u64)], d: f64) -> FuzzySolution {
        FuzzySolution::new(row(pairs), d)
    }

    #[test]
    fn join_combines_compatible_rows_by_tnorm() {
        // ?x bound in both; compatible where ?x=1.
        let left = FuzzyResultSet::from_solutions(vec![sol(&[(0, 1)], 0.8), sol(&[(0, 2)], 0.9)]);
        let right = FuzzyResultSet::from_solutions(vec![sol(&[(0, 1), (1, 7)], 0.6)]);
        let j = left.join(&right, DegreeNorm::Godel);
        assert_eq!(j.len(), 1);
        assert_eq!(j.solutions[0].row.get(0), Some(1));
        assert_eq!(j.solutions[0].row.get(1), Some(7));
        assert!((j.solutions[0].degree - 0.6).abs() < 1e-6); // min(0.8, 0.6)
    }

    #[test]
    fn union_merges_same_row_by_tconorm() {
        let a = FuzzyResultSet::from_solutions(vec![sol(&[(0, 1)], 0.4)]);
        let b = FuzzyResultSet::from_solutions(vec![sol(&[(0, 1)], 0.7), sol(&[(0, 2)], 0.5)]);
        let u = a.union(&b, DegreeNorm::Godel);
        assert_eq!(u.len(), 2);
        let merged = u.solutions.iter().find(|s| s.row.get(0) == Some(1)).unwrap();
        assert!((merged.degree - 0.7).abs() < 1e-6); // max(0.4, 0.7)
    }

    #[test]
    fn threshold_and_ranking() {
        let set = FuzzyResultSet::from_solutions(vec![
            sol(&[(0, 1)], 0.2),
            sol(&[(0, 2)], 0.9),
            sol(&[(0, 3)], 0.5),
        ]);
        let cut = set.clone().threshold(0.5);
        assert_eq!(cut.len(), 2);
        let ranked = set.top_k(1);
        assert_eq!(ranked.solutions[0].row.get(0), Some(2));
    }

    #[test]
    fn projection_takes_max_over_dropped_vars() {
        // Two rows agree on ?x but differ on ?y; projecting onto ?x existentially
        // keeps the max degree.
        let set = FuzzyResultSet::from_solutions(vec![
            sol(&[(0, 1), (1, 10)], 0.3),
            sol(&[(0, 1), (1, 11)], 0.8),
        ]);
        let p = set.project(&[0], DegreeNorm::Godel);
        assert_eq!(p.len(), 1);
        assert_eq!(p.solutions[0].row.get(1), None);
        assert!((p.solutions[0].degree - 0.8).abs() < 1e-6);
    }

    #[test]
    fn negation_complements_degree() {
        let set = FuzzyResultSet::from_solutions(vec![sol(&[(0, 1)], 0.3)]);
        let n = set.negate(DegreeNorm::Godel);
        assert!((n.solutions[0].degree - 0.7).abs() < 1e-6);
    }
}
