//! Quality objective and the per-tet score derived from it. Score is always
//! "higher = better"; passes accept an operation only when the minimum score
//! over the affected cells strictly increases.

use super::*;

// ---------------------------------------------------------------------------
//  Objective + score
// ---------------------------------------------------------------------------

/// Quality objective. The score is always **higher = better**; passes accept
/// an operation only when the minimum score over the affected cells strictly
/// increases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetImproveObjective {
    /// Maximise the minimum dihedral angle (the standard sliver-removal
    /// objective). Score = `min_dihedral`.
    MinDihedral,
    /// Minimise the maximum radius-edge ratio (Ruppert-style Delaunay
    /// refinement objective). Score = `-radius_edge` (higher = lower ratio =
    /// better).
    RadiusEdge,
    /// Maximise the minimum scaled Jacobian. Score = `scaled_jacobian`.
    ScaledJacobian,
}

/// Score of a tet under the objective (higher = better).
#[inline]
pub(super) fn score(q: &TetQuality, obj: TetImproveObjective) -> f64 {
    match obj {
        TetImproveObjective::MinDihedral => q.min_dihedral,
        TetImproveObjective::RadiusEdge => -q.radius_edge,
        TetImproveObjective::ScaledJacobian => q.scaled_jacobian,
    }
}

/// Score a tet directly from its four corners.
#[inline]
pub(super) fn score_corners(
    a: Point3,
    b: Point3,
    c: Point3,
    d: Point3,
    obj: TetImproveObjective,
) -> f64 {
    let q = tet_quality_points(a, b, c, d);
    if !q.valid {
        // Invalid (inverted/degenerate) -> worst possible score.
        f64::NEG_INFINITY
    } else {
        score(&q, obj)
    }
}
