//! The bridge from the crisp SPARQL executor to the fuzzy algebra.
//!
//! The engine evaluates a pattern and yields [`BindingRow`]s; these helpers attach a
//! degree to each (from a fuzzy `FILTER` membership, a fuzzy-RDF triple degree, or a
//! similarity score) and assemble a [`FuzzyResultSet`]. A multi-pattern (BGP) query is
//! then the conjunctive **join** of the per-pattern fuzzy sets.

use super::solution::{FuzzyResultSet, FuzzySolution};
use super::DegreeNorm;
use crate::sparql_ast::BindingRow;

/// Annotate a slice of already-evaluated rows with a degree per row.
pub fn annotate<F>(rows: &[BindingRow], degree_of: F) -> FuzzyResultSet
where
    F: Fn(&BindingRow) -> f64,
{
    FuzzyResultSet::from_solutions(
        rows.iter().map(|r| FuzzySolution::new(*r, degree_of(r))).collect(),
    )
}

/// Pull rows from a live engine operator and annotate them. `pull` is the engine's
/// row-at-a-time step (typically a closure wrapping `PhysicalOperator::next(ctx, row)`):
/// it fills the row and returns `true` while more solutions remain. This is the genuine
/// integration shape — no copy of the executor, no `SparqlQueryContext` leak into this
/// library. `cap` bounds the pull (fail-safe against an unbounded operator).
pub fn collect_from<P, F>(mut pull: P, degree_of: F, cap: usize) -> FuzzyResultSet
where
    P: FnMut(&mut BindingRow) -> bool,
    F: Fn(&BindingRow) -> f64,
{
    let mut out = Vec::new();
    let mut row = BindingRow::new();
    while out.len() < cap {
        row.clear();
        if !pull(&mut row) {
            break;
        }
        out.push(FuzzySolution::new(row, degree_of(&row)));
    }
    FuzzyResultSet::from_solutions(out)
}

/// A conjunctive (basic graph pattern) query over per-pattern fuzzy result sets: join
/// them all under `norm`, then apply the α-cut `threshold`. An empty pattern list is an
/// empty result. This is the f-SPARQL `AND` of graded triple patterns.
pub fn conjunctive_query(
    pattern_sets: &[FuzzyResultSet],
    norm: DegreeNorm,
    threshold: f64,
) -> FuzzyResultSet {
    let mut it = pattern_sets.iter();
    let mut acc = match it.next() {
        Some(first) => first.clone(),
        None => return FuzzyResultSet::new(),
    };
    for next in it {
        acc = acc.join(next, norm);
    }
    acc.threshold(threshold)
}

#[cfg(test)]
mod tests {
    use super::super::membership::approximately;
    use super::*;
    use crate::sparql_ast::{BindingRow, VariableId};

    fn row(pairs: &[(VariableId, u64)]) -> BindingRow {
        let mut r = BindingRow::new();
        for &(v, val) in pairs {
            r.set(v, val);
        }
        r
    }

    #[test]
    fn annotate_with_a_fuzzy_filter() {
        // Slot 1 holds an age; "≈ 30 ± 10" is the fuzzy FILTER.
        let rows = [row(&[(0, 100), (1, 30)]), row(&[(0, 101), (1, 25)]), row(&[(0, 102), (1, 50)])];
        let set = annotate(&rows, |r| approximately(r.get(1).unwrap() as f64, 30.0, 10.0));
        let ranked = set.order_by_degree_desc();
        assert_eq!(ranked.solutions[0].row.get(0), Some(100)); // exactly 30 → degree 1
        assert!(ranked.solutions.last().unwrap().degree.abs() < 1e-9); // 50 → 0
    }

    #[test]
    fn collect_from_a_simulated_operator() {
        // A fake engine operator yielding three rows, then false.
        let data = [row(&[(0, 1), (1, 30)]), row(&[(0, 2), (1, 28)])];
        let mut i = 0;
        let pull = |out: &mut BindingRow| {
            if i < data.len() {
                *out = data[i];
                i += 1;
                true
            } else {
                false
            }
        };
        let set = collect_from(pull, |r| approximately(r.get(1).unwrap() as f64, 30.0, 5.0), 1000);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn collect_respects_cap() {
        // An unbounded operator must be reined in by `cap`.
        let pull = |out: &mut BindingRow| {
            out.set(0, 1);
            true
        };
        let set = collect_from(pull, |_| 1.0, 10);
        assert_eq!(set.len(), 10);
    }

    #[test]
    fn conjunctive_bgp_join_and_threshold() {
        // Pattern A: ?x age≈30 ; Pattern B: ?x linked to ?y. Join on ?x, threshold.
        let a = annotate(
            &[row(&[(0, 1), (1, 30)]), row(&[(0, 2), (1, 20)])],
            |r| approximately(r.get(1).unwrap() as f64, 30.0, 10.0),
        );
        let b = annotate(&[row(&[(0, 1), (2, 99)])], |_| 1.0);
        let q = conjunctive_query(&[a, b], DegreeNorm::Godel, 0.5);
        assert_eq!(q.len(), 1);
        assert_eq!(q.solutions[0].row.get(2), Some(99));
        assert!(q.solutions[0].degree >= 0.5);
    }

    #[test]
    fn empty_patterns_is_empty() {
        assert!(conjunctive_query(&[], DegreeNorm::Godel, 0.0).is_empty());
    }
}
